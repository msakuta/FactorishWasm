mod iter;

use super::{
    burner::Burner,
    drop_items::DropItem,
    dyn_iter::{DynIter, DynIterMut},
    factory::Factory,
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

pub(crate) trait Structure {
    fn name(&self) -> &str;

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
            recipe.input.get(item_type).is_some()
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
    fn inventory(&self, _is_input: bool) -> Option<&Inventory> {
        None
    }
    fn inventory_mut(&mut self, _is_input: bool) -> Option<&mut Inventory> {
        None
    }
    /// Some structures don't have an inventory, but still can have some item, e.g. inserter hands.
    /// We need to retrieve them when we destory such a structure, or we might lose items into void.
    /// It will take away the inventory by default, destroying the instance's inventory.
    fn destroy_inventory(&mut self) -> Inventory {
        let mut ret = self
            .inventory_mut(true)
            .map_or(Inventory::new(), |inventory| std::mem::take(inventory));
        ret.merge(
            self.inventory_mut(false)
                .map_or(Inventory::new(), |inventory| std::mem::take(inventory)),
        );
        ret
    }
    /// Returns a list of recipes. The return value is wrapped in a Cow because some
    /// structures can return dynamically configured list of recipes, while some others
    /// have static fixed list of recipes. In reality, all our structures return a fixed list though.
    fn get_recipes(&self) -> Cow<[Recipe]> {
        Cow::from(&[][..])
    }
    fn select_recipe(&mut self, _factory: &mut Factory, _index: usize) -> Result<bool, JsValue> {
        Err(JsValue::from_str("recipes not available"))
    }
    fn get_selected_recipe(&self) -> Option<&Recipe> {
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
    fn power_outlet(&mut self, _demand: f64) -> Option<f64> {
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

pub(crate) struct ComponentError(&'static str);

impl From<ComponentError> for JsValue {
    fn from(ce: ComponentError) -> Self {
        js_str!("UndergroundPipe without {}", ce.0)
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

    pub fn get_position(&self) -> Result<Position, ComponentError> {
        self.position.ok_or_else(|| ComponentError("Position"))
    }

    pub fn get_rotation(&self) -> Result<Rotation, ComponentError> {
        self.rotation.ok_or_else(|| ComponentError("Rotation"))
    }

    pub fn get_fluid_box_first(&self) -> Result<&FluidBox, ComponentError> {
        self.fluid_boxes
            .first()
            .ok_or_else(|| ComponentError("FluidBox"))
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
            || self
                .components
                .factory
                .as_ref()
                .map(|factory| factory.can_input(item_type))
                .unwrap_or(false)
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

    pub(crate) fn inventory(&self, is_input: bool) -> Option<&Inventory> {
        if let Some(inventory) = self.dynamic.inventory(is_input) {
            return Some(inventory);
        } else {
            self.components
                .factory
                .as_ref()
                .map(|factory| factory.inventory(is_input))?
        }
    }

    pub(crate) fn inventory_mut(&mut self, is_input: bool) -> Option<&mut Inventory> {
        if let Some(inventory) = self.dynamic.inventory_mut(is_input) {
            return Some(inventory);
        } else {
            self.components
                .factory
                .as_mut()
                .map(|factory| factory.inventory_mut(is_input))?
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
}

pub(crate) struct StructureEntry {
    pub gen: u32,
    pub bundle: Option<StructureBundle>,
}
