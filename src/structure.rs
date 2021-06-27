use super::{
    burner::Burner, factory::Factory, items::ItemType, water_well::FluidBox, DropItem,
    FactorishState, Inventory, InventoryTrait, Recipe,
};
use rotate_enum::RotateEnum;
use serde::{Deserialize, Serialize};
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

#[derive(Eq, PartialEq, Hash, Copy, Clone, Debug, Serialize, Deserialize)]
pub(crate) struct Position {
    pub x: i32,
    pub y: i32,
}

impl Position {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
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

#[derive(Copy, Clone, Serialize, Deserialize, RotateEnum)]
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

    pub fn is_vertial(&self) -> bool {
        !self.is_horizontal()
    }
}

pub(crate) enum FrameProcResult {
    None,
    InventoryChanged(Position),
}

pub(crate) enum ItemResponse {
    Move(i32, i32),
    Consume,
}

pub(crate) type ItemResponseResult = (ItemResponse, Option<FrameProcResult>);

use std::fmt::Debug;

pub(crate) trait DynIter {
    type Item;
    fn dyn_iter(&self) -> Box<dyn Iterator<Item = &Self::Item> + '_>;
    fn as_dyn_iter(&self) -> &dyn DynIter<Item = Self::Item>;
}
impl<T, Item> DynIter for T
where
    for<'a> &'a T: IntoIterator<Item = &'a Item>,
{
    type Item = Item;
    fn dyn_iter(&self) -> Box<dyn Iterator<Item = &Self::Item> + '_> {
        Box::new(self.into_iter())
    }
    fn as_dyn_iter(&self) -> &dyn DynIter<Item = Self::Item> {
        self
    }
}

pub(crate) trait DynIterMut: DynIter {
    fn dyn_iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut Self::Item> + '_>;
}
impl<T, Item> DynIterMut for T
where
    for<'a> &'a T: IntoIterator<Item = &'a Item>,
    for<'a> &'a mut T: IntoIterator<Item = &'a mut Item>,
{
    fn dyn_iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut Self::Item> + '_> {
        Box::new(self.into_iter())
    }
}

pub(crate) trait Structure {
    fn name(&self) -> &str;
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
    fn desc(&self, _components: &StructureComponents, _state: &FactorishState) -> String {
        String::from("")
    }
    fn frame_proc(
        &mut self,
        _components: &mut StructureComponents,
        _state: &mut FactorishState,
        _structures: &mut dyn DynIterMut<Item = StructureBundle>,
    ) -> Result<FrameProcResult, ()> {
        Ok(FrameProcResult::None)
    }
    /// event handler for costruction events around the structure.
    fn on_construction(&mut self, _other: &dyn Structure, _construct: bool) -> Result<(), JsValue> {
        Ok(())
    }
    /// event handler for costruction events for this structure itself.
    fn on_construction_self(
        &mut self,
        _others: &dyn DynIter<Item = StructureBundle>,
        _construct: bool,
    ) -> Result<(), JsValue> {
        Ok(())
    }
    fn movable(&self) -> bool {
        false
    }
    fn rotate(&mut self) -> Result<(), ()> {
        Err(())
    }
    fn set_rotation(&mut self, _rotation: &Rotation) -> Result<(), ()> {
        Err(())
    }
    /// Called every frame for each item that is on this structure.
    fn item_response(
        &mut self,
        _components: &mut StructureComponents,
        _item: &DropItem,
    ) -> Result<ItemResponseResult, ()> {
        Err(())
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
    fn can_input(&self, item_type: &ItemType) -> bool {
        if let Some(recipe) = self.get_selected_recipe() {
            recipe.input.get(item_type).is_some()
        } else {
            false
        }
    }
    /// Query a set of items that this structure can output. Actual output would not happen until `output()`, thus
    /// this method is immutable. It should return empty Inventory if it cannot output anything.
    fn can_output(&self) -> Inventory {
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
    fn get_recipes(&self) -> Vec<Recipe> {
        vec![]
    }
    fn select_recipe(&mut self, _factory: &mut Factory, _index: usize) -> Result<bool, JsValue> {
        Err(JsValue::from_str("recipes not available"))
    }
    fn get_selected_recipe(&self) -> Option<&Recipe> {
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
        components: &StructureComponents,
        state: &FactorishState,
        structures: &dyn DynIter<Item = StructureBundle>,
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
                .find(|s| s.components.position == Some(Position { x, y }))
            {
                return structure.dynamic.fluid_box().is_some();
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

pub(crate) struct StructureComponents {
    pub position: Option<Position>,
    pub burner: Option<Burner>,
    pub energy: Option<Energy>,
    pub factory: Option<Factory>,
}

impl StructureComponents {
    fn _new_with_position(position: Position) -> Self {
        Self {
            position: Some(position),
            burner: None,
            energy: None,
            factory: None,
        }
    }
}

impl Default for StructureComponents {
    fn default() -> Self {
        Self {
            position: None,
            burner: None,
            energy: None,
            factory: None,
        }
    }
}

pub(crate) struct StructureBundle {
    pub dynamic: Box<dyn Structure>,
    pub components: StructureComponents,
}

impl StructureBundle {
    pub(crate) fn new(
        dynamic: Box<dyn Structure>,
        position: Option<Position>,
        burner: Option<Burner>,
        energy: Option<Energy>,
        factory: Option<Factory>,
    ) -> Self {
        Self {
            dynamic,
            components: StructureComponents {
                position,
                burner,
                energy,
                factory,
            },
        }
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
        self.dynamic.can_input(item_type)
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

    pub(crate) fn can_output(&self) -> Inventory {
        let mut ret = self.dynamic.can_output();
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
}
