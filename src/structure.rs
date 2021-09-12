mod iter;

use crate::inventory::STACK_SIZE;

use super::{
    drop_items::DropItem,
    dyn_iter::{DynIter, DynIterMut},
    inventory::InventoryType,
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
        fn serialize(&self) -> serde_json::Result<serde_json::Value> {
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

    pub fn is_vertcial(&self) -> bool {
        !self.is_horizontal()
    }
}

pub(crate) enum FrameProcResult {
    None,
    InventoryChanged(Position),
    UpdateResearch,
}

pub(crate) enum ItemResponse {
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
const RECIPE_CAPACITY_MULTIPLIER: usize = 3;

/// If recipe was not selected for a furnace, it is allowed to insert any item, but limit the amount by this value.
const DEFAULT_MAX_CAPACITY: usize = 50;

/// Chest storage size, matching to Factorio
const STORAGE_MAX_SLOTS: usize = 48;

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
                if let Some((recipe, inventory)) =
                    s.get_selected_recipe().zip(s.inventory(inventory_type))
                {
                    let capacity = recipe.input.count_item(item_type) * RECIPE_CAPACITY_MULTIPLIER;
                    let existing_count = inventory.count_item(item_type);
                    if existing_count < capacity {
                        count = count.min((capacity - existing_count) as isize);
                    } else {
                        count = 0;
                    }
                } else {
                    count = DEFAULT_MAX_CAPACITY as isize;
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
    fn name(&self) -> &str;
    fn position(&self) -> &Position;
    fn rotation(&self) -> Option<Rotation> {
        None
    }

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
    fn bounding_box(&self) -> BoundingBox {
        let (position, size) = (self.position(), self.size());
        BoundingBox {
            x0: position.x,
            y0: position.y,
            x1: position.x + size.width,
            y1: position.y + size.height,
        }
    }
    fn contains(&self, pos: &Position) -> bool {
        let bb = self.bounding_box();
        bb.x0 <= pos.x && pos.x < bb.x1 && bb.y0 <= pos.y && pos.y < bb.y1
    }
    fn draw(
        &self,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        is_tooptip: bool,
    ) -> Result<(), JsValue>;
    fn draw_gl(
        &self,
        _state: &FactorishState,
        _gl: &web_sys::WebGlRenderingContext,
        _depth: i32,
        _is_ghost: bool,
    ) -> Result<(), JsValue> {
        Ok(())
    }
    fn desc(&self, _state: &FactorishState) -> String {
        String::from("")
    }
    fn frame_proc(
        &mut self,
        _me: StructureId,
        _state: &mut FactorishState,
        _structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        Ok(FrameProcResult::None)
    }
    /// event handler for costruction events around the structure.
    fn on_construction(
        &mut self,
        _other_id: StructureId,
        _other: &dyn Structure,
        _others: &StructureDynIter,
        _construct: bool,
    ) -> Result<(), JsValue> {
        Ok(())
    }
    /// event handler for costruction events for this structure itself.
    fn on_construction_self(
        &mut self,
        _id: StructureId,
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
        _state: &mut FactorishState,
        _others: &StructureDynIter,
    ) -> Result<(), RotateErr> {
        Err(RotateErr::NotSupported)
    }
    fn set_rotation(&mut self, _rotation: &Rotation) -> Result<(), ()> {
        Err(())
    }
    /// Called every frame for each item that is on this structure.
    fn item_response(&mut self, _item: &DropItem) -> Result<ItemResponseResult, ()> {
        Err(())
    }
    fn input(&mut self, _o: &DropItem) -> Result<(), JsValue> {
        Err(JsValue::from_str("Not supported"))
    }
    /// Returns wheter the structure can accept an item as the input. If this structure is a factory
    /// that returns recipes by get_selected_recipe(), it will check if it's in the inputs.
    fn can_input(&self, item_type: &ItemType) -> bool {
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
    fn can_output(&self, _structures: &StructureDynIter) -> Inventory {
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
    fn select_recipe(
        &mut self,
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
    fn fluid_connections(&self) -> [bool; 4] {
        [true; 4]
    }
    /// Method to return underground pipe reach length.
    fn under_pipe_reach(&self) -> Option<i32> {
        None
    }
    fn fluid_box(&self) -> Option<Vec<&FluidBox>> {
        None
    }
    fn fluid_box_mut(&mut self) -> Option<Vec<&mut FluidBox>> {
        None
    }
    fn connection(
        &self,
        state: &FactorishState,
        structures: &dyn DynIter<Item = StructureEntry>,
    ) -> [bool; 4] {
        // let mut structures_copy = structures.clone();
        let has_fluid_box = |x, y| {
            if x < 0 || state.width <= x as u32 || y < 0 || state.height <= y as u32 {
                return false;
            }
            if let Some(structure) = structures
                .dyn_iter()
                .filter_map(|s| s.dynamic.as_deref())
                .find(|s| *s.position() == Position { x, y })
            {
                return structure.fluid_box().is_some();
            }
            false
        };

        // Fluid containers connect to other containers
        let Position { x, y } = *self.position();
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
    fn power_outlet(&mut self, _demand: f64) -> Option<f64> {
        None
    }
    fn wire_reach(&self) -> u32 {
        3
    }
    fn serialize(&self) -> serde_json::Result<serde_json::Value>;
}

pub(crate) type StructureBoxed = Box<dyn Structure>;

pub(crate) struct StructureEntry {
    pub gen: u32,
    pub dynamic: Option<StructureBoxed>,
}
