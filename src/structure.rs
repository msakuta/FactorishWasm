mod iter;

use super::{
    burner::Burner,
    drop_items::DropItem,
    dyn_iter::{DynIter, DynIterMut},
    factory::Factory,
    inventory::{InventoryType, STACK_SIZE},
    items::ItemType,
    underground_belt::UnderDirection,
    water_well::FluidBox,
    FactorishState, Inventory, InventoryTrait, Recipe,
};
use rotate_enum::RotateEnum;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

#[macro_export]
macro_rules! serialize_impl {
    () => {
        fn js_serialize(&self) -> serde_json::Result<serde_json::Value> {
            serde_json::to_value(self)
        }
    };
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct StructureId {
    pub id: u32,
    pub gen: u32,
}

pub(crate) struct StructureEntryIterator<'a>(&'a mut [StructureEntry], &'a mut [StructureEntry]);

impl<'a> DynIter for StructureEntryIterator<'a> {
    type Item = StructureEntry;
    fn dyn_iter(&self) -> Box<dyn Iterator<Item = &Self::Item> + '_> {
        Box::new(self.0.iter().chain(self.1.iter()))
    }
    fn as_dyn_iter(&self) -> &dyn DynIter<Item = Self::Item> {
        self
    }
}

impl<'a> DynIterMut for StructureEntryIterator<'a> {
    fn dyn_iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut Self::Item> + '_> {
        Box::new(self.0.iter_mut().chain(self.1.iter_mut()))
    }
}

pub(crate) use self::iter::StructureDynIter;

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    pub(crate) fn div_mod(&self, size: i32) -> (Position, Position) {
        let div = Position::new(self.x.div_euclid(size), self.y.div_euclid(size));
        let mod_ = Position::new(self.x.rem_euclid(size), self.y.rem_euclid(size));
        (div, mod_)
    }

    pub(crate) fn add(&self, o: (i32, i32)) -> Position {
        Self {
            x: self.x + o.0,
            y: self.y + o.1,
        }
    }
    pub(crate) fn distance(&self, position: &Position) -> i32 {
        (position.x - self.x).abs().max((position.y - self.y).abs())
    }

    /// Check whether the positions are neighbors. Return false if they are exactly the same.
    #[allow(dead_code)]
    pub(crate) fn is_neighbor(&self, pos2: &Position) -> bool {
        [[-1, 0], [0, -1], [1, 0], [0, 1]].iter().any(|rel_pos| {
            let pos = Position {
                x: pos2.x + rel_pos[0],
                y: pos2.y + rel_pos[1],
            };
            *self == pos
        })
    }

    pub(crate) fn neighbor_index(&self, pos2: &Position) -> Option<u32> {
        for (i, rel_pos) in [[-1, 0], [0, -1], [1, 0], [0, 1]].iter().enumerate() {
            let pos = Position {
                x: pos2.x + rel_pos[0],
                y: pos2.y + rel_pos[1],
            };
            if *self == pos {
                return Some(i as u32);
            }
        }
        None
    }
}

impl From<&[i32; 2]> for Position {
    fn from(xy: &[i32; 2]) -> Self {
        Self { x: xy[0], y: xy[1] }
    }
}

pub(crate) struct Size {
    pub width: i32,
    pub height: i32,
}

pub(crate) struct BoundingBox {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

#[derive(Copy, Clone, Serialize, Deserialize, RotateEnum, PartialEq)]
pub(crate) enum Rotation {
    Left,
    Top,
    Right,
    Bottom,
}

impl Rotation {
    pub fn delta(&self) -> (i32, i32) {
        match self {
            Rotation::Left => (-1, 0),
            Rotation::Top => (0, -1),
            Rotation::Right => (1, 0),
            Rotation::Bottom => (0, 1),
        }
    }

    pub fn delta_inv(&self) -> (i32, i32) {
        let delta = self.delta();
        (-delta.0, -delta.1)
    }

    pub fn angle_deg(&self) -> i32 {
        self.angle_4() * 90
    }

    pub fn angle_4(&self) -> i32 {
        match self {
            Rotation::Left => 2,
            Rotation::Top => 3,
            Rotation::Right => 0,
            Rotation::Bottom => 1,
        }
    }

    pub fn angle_rad(&self) -> f64 {
        self.angle_deg() as f64 * std::f64::consts::PI / 180.
    }

    pub fn is_horizontal(&self) -> bool {
        matches!(self, Rotation::Left | Rotation::Right)
    }

    pub fn is_vertical(&self) -> bool {
        !self.is_horizontal()
    }
}

pub(crate) enum FrameProcResult {
    None,
    InventoryChanged(Position),
    UpdateResearch,
}

pub(crate) enum ItemResponse {
    None,
    Move(i32, i32),
    Consume,
}

pub(crate) type ItemResponseResult = (ItemResponse, Option<FrameProcResult>);

#[derive(Debug)]
pub(crate) enum RotateErr {
    NotFound,
    NotSupported,
    Other(JsValue),
}

/// Factories will have input inventory capacity of recipe ingredients enough to make this many products
pub(crate) const RECIPE_CAPACITY_MULTIPLIER: usize = 3;

/// Chest storage size, matching to Factorio
const STORAGE_MAX_SLOTS: usize = 48;

const DEFAULT_MAX_CAPACITY: usize = 50;

pub(crate) fn default_add_inventory(
    s: &mut (impl Structure + ?Sized),
    inventory_type: InventoryType,
    item_type: &ItemType,
    count: isize,
) -> isize {
    let mut count = count;
    if 0 < count {
        match inventory_type {
            InventoryType::Input => {
                if let Some(inventory) = s.inventory(inventory_type) {
                    let capacity = if let Some(recipe) = s.get_selected_recipe() {
                        recipe.input.count_item(item_type) * RECIPE_CAPACITY_MULTIPLIER
                    } else if s.auto_recipe() {
                        if let Some(recipe) = s
                            .get_recipes()
                            .iter()
                            .find(|recipe| recipe.input.contains_key(item_type))
                        {
                            recipe.input.count_item(item_type) * RECIPE_CAPACITY_MULTIPLIER
                        } else {
                            return 0;
                        }
                    } else {
                        return 0;
                    };
                    let existing_count = inventory.count_item(item_type);
                    if existing_count < capacity {
                        count = count.min((capacity - existing_count) as isize);
                    } else {
                        count = 0;
                    }
                } else {
                    return 0;
                }
            }
            InventoryType::Storage => {
                if let Some(inventory) = s.inventory(inventory_type) {
                    let occupied_slots = inventory.count_slots() as isize;
                    let mut left_count =
                        (STORAGE_MAX_SLOTS as isize - occupied_slots) * STACK_SIZE as isize;
                    let last_stack = inventory.count_item(item_type) % STACK_SIZE;
                    if 0 < last_stack {
                        left_count += (STACK_SIZE - last_stack) as isize;
                    }
                    count = count.min(left_count);
                }
            }
            _ => (),
        }
    }
    if let Some(inventory) = s.inventory_mut(inventory_type) {
        if 0 < count {
            inventory.add_items(item_type, count as usize);
            count
        } else {
            -(inventory.remove_items(item_type, count.abs() as usize) as isize)
        }
    } else {
        0
    }
}

pub(crate) trait Structure {
    fn name(&self) -> &'static str;

    /// Specialized method to get underground belt direction.
    /// We don't like to put this to Structure trait method, but we don't have an option
    /// as long as we use trait object polymorphism.
    /// TODO: Revise needed in ECS.
    fn under_direction(&self) -> Option<UnderDirection> {
        None
    }

    fn size(&self) -> Size {
        Size {
            width: 1,
            height: 1,
        }
    }
    fn bounding_box(&self, components: &StructureComponents) -> Option<BoundingBox> {
        let position = &components.position?;
        let (position, size) = (position, self.size());
        Some(BoundingBox {
            x0: position.x,
            y0: position.y,
            x1: position.x + size.width,
            y1: position.y + size.height,
        })
    }
    fn contains(&self, components: &StructureComponents, pos: &Position) -> bool {
        self.bounding_box(components)
            .map(|bb| bb.x0 <= pos.x && pos.x < bb.x1 && bb.y0 <= pos.y && pos.y < bb.y1)
            .unwrap_or(false)
    }
    fn draw(
        &self,
        _components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        is_tooptip: bool,
    ) -> Result<(), JsValue>;
    fn draw_gl(
        &self,
        _components: &StructureComponents,
        _state: &FactorishState,
        _gl: &web_sys::WebGlRenderingContext,
        _depth: i32,
        _is_ghost: bool,
    ) -> Result<(), JsValue> {
        Ok(())
    }
    fn desc(&self, _components: &StructureComponents, _state: &FactorishState) -> String {
        String::from("")
    }
    fn frame_proc(
        &mut self,
        _me: StructureId,
        _components: &mut StructureComponents,
        _state: &mut FactorishState,
        _structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        Ok(FrameProcResult::None)
    }
    /// event handler for costruction events around the structure.
    fn on_construction(
        &mut self,
        _components: &mut StructureComponents,
        _other_id: StructureId,
        _other: &StructureBundle,
        _others: &StructureDynIter,
        _construct: bool,
    ) -> Result<(), JsValue> {
        Ok(())
    }
    /// event handler for costruction events for this structure itself.
    fn on_construction_self(
        &mut self,
        _id: StructureId,
        _components: &mut StructureComponents,
        _others: &StructureDynIter,
        _construct: bool,
    ) -> Result<(), JsValue> {
        Ok(())
    }
    fn movable(&self) -> bool {
        false
    }
    fn rotate(
        &mut self,
        _components: &mut StructureComponents,
        _state: &mut FactorishState,
        _others: &StructureDynIter,
    ) -> Result<(), RotateErr> {
        Err(RotateErr::NotSupported)
    }
    fn set_rotation(
        &mut self,
        _components: &mut StructureComponents,
        _rotation: &Rotation,
    ) -> Result<(), ()> {
        Err(())
    }
    /// Called every frame for each item that is on this structure.
    fn item_response(
        &mut self,
        _components: &mut StructureComponents,
        _item: &DropItem,
    ) -> Result<ItemResponseResult, JsValue> {
        Err(js_str!("ItemResponse not implemented"))
    }
    fn input(
        &mut self,
        _components: &mut StructureComponents,
        _o: &DropItem,
    ) -> Result<(), JsValue> {
        Err(JsValue::from_str("Not supported"))
    }
    /// Returns wheter the structure can accept an item as the input. If this structure is a factory
    /// that returns recipes by get_selected_recipe(), it will check if it's in the inputs.
    fn can_input(&self, _components: &StructureComponents, item_type: &ItemType) -> bool {
        if let Some(recipe) = self.get_selected_recipe() {
            if let Some(inventory) = self.inventory(InventoryType::Input) {
                // Two times the product requirements
                inventory.count_item(item_type) < recipe.input.get(item_type).unwrap_or(&0) * 2
            } else {
                recipe.input.get(item_type).is_some()
            }
        } else {
            false
        }
    }
    /// Query a set of items that this structure can output. Actual output would not happen until `output()`, thus
    /// this method is immutable. It should return empty Inventory if it cannot output anything.
    fn can_output(
        &self,
        _components: &StructureComponents,
        _structures: &StructureDynIter,
    ) -> Inventory {
        Inventory::new()
    }
    /// Perform actual output. The operation should always succeed since the output-tability is checked beforehand
    /// with `can_output`.
    fn output(&mut self, _state: &mut FactorishState, _item_type: &ItemType) -> Result<(), ()> {
        Err(())
    }
    /// Attempt to add or remove items from a burner inventory and returns actual item count moved.
    /// Positive amount means adding items to the burner, otherwise remove.
    /// If it has limited capacity, positive amount may return less value than given.
    fn add_inventory(
        &mut self,
        inventory_type: InventoryType,
        item_type: &ItemType,
        count: isize,
    ) -> isize {
        default_add_inventory(self, inventory_type, item_type, count)
    }
    fn burner_energy(&self) -> Option<(f64, f64)> {
        None
    }
    fn inventory(&self, _inventory_type: InventoryType) -> Option<&Inventory> {
        None
    }
    fn inventory_mut(&mut self, _inventory_type: InventoryType) -> Option<&mut Inventory> {
        None
    }
    /// Some structures don't have an inventory, but still can have some item, e.g. inserter hands.
    /// We need to retrieve them when we destory such a structure, or we might lose items into void.
    /// It will take away the inventory by default, destroying the instance's inventory.
    fn destroy_inventory(&mut self) -> Inventory {
        let mut ret = self
            .inventory_mut(InventoryType::Input)
            .map_or(Inventory::new(), |inventory| std::mem::take(inventory));
        if let Some(inv) = self.inventory_mut(InventoryType::Output) {
            ret.merge(std::mem::take(inv));
        }
        if let Some(inv) = self.inventory_mut(InventoryType::Storage) {
            ret.merge(std::mem::take(inv));
        }
        ret
    }
    /// Returns a list of recipes. The return value is wrapped in a Cow because some
    /// structures can return dynamically configured list of recipes, while some others
    /// have static fixed list of recipes. In reality, all our structures return a fixed list though.
    fn get_recipes(&self) -> Cow<[Recipe]> {
        Cow::from(&[][..])
    }
    fn auto_recipe(&self) -> bool {
        false
    }
    fn select_recipe(
        &mut self,
        _factory: Option<&mut Factory>,
        _index: usize,
        _player_inventory: &mut Inventory,
    ) -> Result<bool, JsValue> {
        Err(JsValue::from_str("recipes not available"))
    }
    fn get_selected_recipe(&self) -> Option<&Recipe> {
        None
    }
    fn get_progress(&self) -> Option<f64> {
        None
    }
    fn fluid_connections(&self, _components: &StructureComponents) -> [bool; 4] {
        [true; 4]
    }
    /// Method to return underground pipe reach length.
    fn under_pipe_reach(&self) -> Option<i32> {
        None
    }
    fn connection(
        &self,
        components: &StructureComponents,
        state: &FactorishState,
        structures: &dyn DynIter<Item = StructureEntry>,
    ) -> [bool; 4] {
        let position = if let Some(position) = components.position.as_ref() {
            position
        } else {
            return [false; 4];
        };
        // let mut structures_copy = structures.clone();
        let has_fluid_box = |x, y| {
            if x < 0 || state.width <= x as u32 || y < 0 || state.height <= y as u32 {
                return false;
            }
            if let Some(structure) = structures
                .dyn_iter()
                .filter_map(|s| s.bundle.as_ref())
                .find(|s| s.components.position == Some(Position { x, y }))
            {
                return !structure.components.fluid_boxes.is_empty();
            }
            false
        };

        // Fluid containers connect to other containers
        let Position { x, y } = *position;
        let l = has_fluid_box(x - 1, y);
        let t = has_fluid_box(x, y - 1);
        let r = has_fluid_box(x + 1, y);
        let b = has_fluid_box(x, y + 1);
        [l, t, r, b]
    }
    /// If this structure can connect to power grid.
    fn power_source(&self) -> bool {
        false
    }
    /// If this structure drains power from the grid
    fn power_sink(&self) -> bool {
        false
    }
    /// Try to drain power from this structure.
    /// @param demand in kilojoules.
    /// @returns None if it does not support power supply.
    fn power_outlet(&mut self, _components: &mut StructureComponents, _demand: f64) -> Option<f64> {
        None
    }
    fn wire_reach(&self) -> u32 {
        3
    }
    fn js_serialize(&self) -> serde_json::Result<serde_json::Value>;
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Energy {
    pub value: f64,
    pub max: f64,
}

pub(crate) struct ComponentError {
    structure: &'static str,
    component: &'static str,
}

impl From<ComponentError> for JsValue {
    fn from(ce: ComponentError) -> Self {
        js_str!("{} without {} component", ce.structure, ce.component)
    }
}

pub(crate) struct StructureComponents {
    pub position: Option<Position>,
    pub rotation: Option<Rotation>,
    pub burner: Option<Burner>,
    pub energy: Option<Energy>,
    pub factory: Option<Factory>,
    pub fluid_boxes: Vec<FluidBox>,
}

impl StructureComponents {
    pub fn new_with_position(position: Position) -> Self {
        Self {
            position: Some(position),
            rotation: None,
            burner: None,
            energy: None,
            factory: None,
            fluid_boxes: vec![],
        }
    }

    pub fn new_with_position_and_rotation(position: Position, rotation: Rotation) -> Self {
        Self {
            position: Some(position),
            rotation: Some(rotation),
            burner: None,
            energy: None,
            factory: None,
            fluid_boxes: vec![],
        }
    }

    pub fn get_position(&self, dynamic: &dyn Structure) -> Result<Position, ComponentError> {
        self.position.ok_or_else(|| ComponentError {
            structure: dynamic.name(),
            component: "Position",
        })
    }

    pub fn get_rotation(&self, dynamic: &dyn Structure) -> Result<Rotation, ComponentError> {
        self.rotation.ok_or_else(|| ComponentError {
            structure: dynamic.name(),
            component: "Rotation",
        })
    }

    pub fn get_fluid_box_first(
        &self,
        dynamic: &dyn Structure,
    ) -> Result<&FluidBox, ComponentError> {
        self.fluid_boxes.first().ok_or_else(|| ComponentError {
            structure: dynamic.name(),
            component: "FluidBox",
        })
    }
}

impl Default for StructureComponents {
    fn default() -> Self {
        Self {
            position: None,
            rotation: None,
            burner: None,
            energy: None,
            factory: None,
            fluid_boxes: vec![],
        }
    }
}

pub(crate) type StructureBoxed = Box<dyn Structure>;

pub(crate) struct StructureBundle {
    pub dynamic: StructureBoxed,
    pub components: StructureComponents,
}

impl StructureBundle {
    pub(crate) fn new(
        dynamic: Box<dyn Structure>,
        position: Option<Position>,
        rotation: Option<Rotation>,
        burner: Option<Burner>,
        energy: Option<Energy>,
        factory: Option<Factory>,
        fluid_boxes: Vec<FluidBox>,
    ) -> Self {
        Self {
            dynamic,
            components: StructureComponents {
                position,
                rotation,
                burner,
                energy,
                factory,
                fluid_boxes,
            },
        }
    }

    pub(crate) fn bounding_box(&self) -> Option<BoundingBox> {
        self.dynamic.bounding_box(&self.components)
    }

    pub(crate) fn input(&mut self, item: &DropItem) -> Result<(), JsValue> {
        self.dynamic
            .input(&mut self.components, item)
            .or_else(|e| {
                if let Some(burner) = self.components.burner.as_mut() {
                    burner.input(item)
                } else {
                    Err(e)
                }
            })
            .or_else(|_| {
                if let Some(factory) = self.components.factory.as_mut() {
                    if self.dynamic.auto_recipe() {
                        factory.recipe = self
                            .dynamic
                            .get_recipes()
                            .iter()
                            .find(|recipe| recipe.input.contains_key(&item.type_))
                            .cloned();
                    }
                    factory.input(item)
                } else {
                    js_err!("No input inventory")
                }
            })
    }

    pub(crate) fn can_input(&self, item_type: &ItemType) -> bool {
        self.dynamic.can_input(&self.components, item_type)
            || self
                .components
                .burner
                .as_ref()
                .map(|burner| burner.can_input(item_type))
                .unwrap_or(false)
            || self.factory_can_input(item_type, 1) == 1
    }

    pub(crate) fn can_output(&self, others: &StructureDynIter) -> Inventory {
        let mut ret = self.dynamic.can_output(&self.components, others);
        if let Some(factory) = self.components.factory.as_ref() {
            ret.merge(factory.can_output());
        }
        ret
    }

    pub(crate) fn output(
        &mut self,
        state: &mut FactorishState,
        item_type: &ItemType,
    ) -> Result<(), ()> {
        if let Ok(ret) = self.dynamic.output(state, item_type) {
            return Ok(ret);
        }
        if let Some(factory) = self.components.factory.as_mut() {
            if let Ok(()) = factory.output(state, item_type) {
                return Ok(());
            }
        }
        Err(())
    }

    pub(crate) fn inventory(&self, inventory_type: InventoryType) -> Option<&Inventory> {
        if let Some(inventory) = self.dynamic.inventory(inventory_type) {
            return Some(inventory);
        } else {
            self.components
                .factory
                .as_ref()
                .and_then(|factory| factory.inventory(inventory_type))
                .or_else(|| {
                    if inventory_type == InventoryType::Burner {
                        self.components
                            .burner
                            .as_ref()
                            .map(|burner| &burner.inventory)
                    } else {
                        None
                    }
                })
        }
    }

    pub(crate) fn inventory_mut(
        &mut self,
        inventory_type: InventoryType,
    ) -> Option<&mut Inventory> {
        if let Some(inventory) = self.dynamic.inventory_mut(inventory_type) {
            return Some(inventory);
        } else {
            self.components
                .factory
                .as_mut()
                .map(|factory| factory.inventory_mut(inventory_type))?
        }
    }

    /// Return whether an item can be inserted to Factory component.
    ///
    /// Negative `count` means removing items.
    ///
    /// It is separated in a function because the logic is a bit complex.
    fn factory_can_input(&self, item_type: &ItemType, count: isize) -> isize {
        let factory = if let Some(ref factory) = self.components.factory {
            factory
        } else {
            return 0;
        };
        let mut count = count;
        let mut try_move = |inventory: &Inventory, recipe: &Recipe| {
            let existing_count = inventory.count_item(item_type);
            if 0 < count {
                let capacity = recipe.input.count_item(item_type) * RECIPE_CAPACITY_MULTIPLIER;
                if existing_count < capacity {
                    count = count.min((capacity - existing_count) as isize);
                } else {
                    count = 0;
                }
            } else {
                count = -count.abs().min(existing_count as isize);
            }
        };
        if let Some(ref recipe) = factory.recipe {
            try_move(&factory.input_inventory, recipe);
        } else if self.dynamic.auto_recipe() {
            for recipe in self.dynamic.get_recipes().as_ref() {
                if recipe.input.contains_key(item_type) {
                    try_move(&factory.input_inventory, recipe);
                    break;
                }
            }
        } else {
            count = DEFAULT_MAX_CAPACITY as isize;
        }
        count
    }

    /// Try to add items to a structure's inventory and return items actually moved.
    /// The sign is positive if they are added to inventory and negative if they are removed.
    pub(crate) fn add_inventory(
        &mut self,
        inventory_type: InventoryType,
        item_type: &ItemType,
        amount: isize,
    ) -> isize {
        let real_move = |inventory: &mut Inventory, count: isize| {
            if 0 < count {
                inventory.add_items(item_type, count as usize);
                count
            } else {
                -(inventory.remove_items(item_type, count.abs() as usize) as isize)
            }
        };

        match inventory_type {
            InventoryType::Burner => {
                if let Some(ref mut burner) = self.components.burner {
                    burner.add_burner_inventory(item_type, amount)
                } else {
                    return 0;
                }
            }
            InventoryType::Input => {
                let count = self.factory_can_input(item_type, amount);
                if 0 != count {
                    if let Some(ref mut factory) = self.components.factory {
                        return real_move(&mut factory.input_inventory, count);
                    }
                }
                default_add_inventory(self.dynamic.as_mut(), inventory_type, item_type, amount)
            }
            InventoryType::Output => {
                if let Some(ref mut factory) = self.components.factory {
                    let mut count = amount;
                    let inventory = &mut factory.output_inventory;
                    let existing_count = inventory.count_item(item_type);
                    if 0 < count {
                        if let Some(ref mut recipe) = factory.recipe {
                            let capacity =
                                recipe.output.count_item(item_type) * RECIPE_CAPACITY_MULTIPLIER;
                            if existing_count < capacity {
                                count = count.min((capacity - existing_count) as isize);
                            } else {
                                count = 0;
                            }
                        } else {
                            count = DEFAULT_MAX_CAPACITY as isize;
                        }
                    } else {
                        count = -count.abs().min(existing_count as isize);
                    }
                    real_move(&mut factory.output_inventory, count)
                } else {
                    default_add_inventory(self.dynamic.as_mut(), inventory_type, item_type, amount)
                }
            }
            InventoryType::Storage => {
                default_add_inventory(self.dynamic.as_mut(), inventory_type, item_type, amount)
            }
        }
    }

    pub(crate) fn rotate(
        &mut self,
        state: &mut FactorishState,
        others: &StructureDynIter,
    ) -> Result<(), RotateErr> {
        let result = self.dynamic.rotate(&mut self.components, state, others);
        if result.is_ok() {
            return Ok(());
        }
        if let Some(ref mut rotation) = self.components.rotation {
            *rotation = rotation.next();
            Ok(())
        } else {
            result
        }
    }

    pub(crate) fn set_rotation(&mut self, rotation: &Rotation) -> Result<(), ()> {
        self.components.rotation = Some(*rotation);
        Ok(())
    }

    pub(crate) fn get_selected_recipe(&self) -> Option<&Recipe> {
        self.dynamic.get_selected_recipe().or_else(|| {
            self.components
                .factory
                .as_ref()
                .and_then(|factory| factory.recipe.as_ref())
        })
    }
}

pub(crate) struct StructureEntry {
    pub gen: u32,
    pub bundle: Option<StructureBundle>,
}
