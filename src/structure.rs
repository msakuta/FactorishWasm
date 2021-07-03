use super::{
    burner::Burner, factory::Factory, items::ItemType, water_well::FluidBox, DropItem,
    FactorishState, Inventory, InventoryTrait, Recipe,
};
use rotate_enum::RotateEnum;
use serde::{Deserialize, Serialize};
use specs::{
    Component, DenseVecStorage, Entities, Entity, ReadStorage, System, VecStorage, World, WorldExt,
    WriteStorage,
};
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

#[derive(Debug)]
pub(crate) struct BoundingBox {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize, RotateEnum)]
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
    CreateItem(DropItem),
}

pub(crate) enum ItemResponse {
    None,
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
    fn bounding_box(&self, entity: Entity, state: &FactorishState) -> Option<BoundingBox> {
        let position = state
            .world
            .read_component::<Position>()
            .get(entity)
            .copied()?;
        let size = self.size();
        Some(BoundingBox {
            x0: position.x,
            y0: position.y,
            x1: position.x + size.width,
            y1: position.y + size.height,
        })
    }
    fn contains(&self, entity: Entity, pos: &Position, state: &FactorishState) -> bool {
        self.bounding_box(entity, state)
            .map(|bb| bb.x0 <= pos.x && pos.x < bb.x1 && bb.y0 <= pos.y && pos.y < bb.y1)
            .unwrap_or(false)
    }
    fn draw(
        &self,
        entity: Entity,
        _components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        is_tooptip: bool,
    ) -> Result<(), JsValue>;
    fn desc(&self, _entity: Entity, _state: &FactorishState) -> String {
        String::from("")
    }
    fn frame_proc(
        &mut self,
        _components: &mut StructureComponents,
        _state: &mut FactorishState,
    ) -> Result<FrameProcResult, ()> {
        Ok(FrameProcResult::None)
    }
    /// event handler for costruction events around the structure.
    fn on_construction(&mut self, _other: &dyn Structure, _construct: bool) -> Result<(), JsValue> {
        Ok(())
    }
    /// event handler for costruction events for this structure itself.
    fn on_construction_self(&mut self, _construct: bool) -> Result<(), JsValue> {
        Ok(())
    }
    fn movable(&self) -> bool {
        false
    }
    fn rotate(&mut self, _components: &mut StructureComponents) -> Result<(), ()> {
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
    fn connection(&self, entity: Entity, state: &FactorishState) -> [bool; 4] {
        let position = if let Some(position) = state.world.read_component::<Position>().get(entity)
        {
            *position
        } else {
            return [false; 4];
        };
        // let mut structures_copy = structures.clone();
        let has_fluid_box = |x, y| {
            use specs::Join;
            if x < 0 || state.width <= x as u32 || y < 0 || state.height <= y as u32 {
                return false;
            }
            if let Some(structure) = (
                &state.world.read_component::<StructureBoxed>(),
                &state.world.read_component::<Position>(),
            )
                .join()
                .find(|(_, position)| **position == Position { x, y })
            {
                return structure.0.fluid_box().is_some();
            }
            false
        };

        // Fluid containers connect to other containers
        let Position { x, y } = position;
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

impl Component for Energy {
    type Storage = VecStorage<Self>;
}

#[derive(Component)]
pub(crate) struct Movable;

pub(crate) struct StructureComponents<'a> {
    pub position: Option<Position>,
    pub rotation: Option<Rotation>,
    pub burner: Option<&'a mut Burner>,
    pub energy: Option<&'a mut Energy>,
    pub factory: Option<&'a mut Factory>,
}

impl<'a> StructureComponents<'a> {
    pub fn new_with_position_and_rotation(position: Position, rotation: Rotation) -> Self {
        Self {
            position: Some(position),
            rotation: Some(rotation),
            burner: None,
            energy: None,
            factory: None,
        }
    }
}

impl<'a> Default for StructureComponents<'a> {
    fn default() -> Self {
        Self {
            position: None,
            rotation: None,
            burner: None,
            energy: None,
            factory: None,
        }
    }
}

pub(crate) type StructureDynamic = dyn Structure + Send + Sync;
pub(crate) type StructureBoxed = Box<StructureDynamic>;

pub(crate) struct StructureBundle<'a> {
    pub dynamic: &'a mut StructureDynamic,
    pub components: StructureComponents<'a>,
}

impl<'a> StructureBundle<'a> {
    pub(crate) fn new(
        dynamic: &'a mut StructureDynamic,
        position: Option<Position>,
        rotation: Option<Rotation>,
        burner: Option<&'a mut Burner>,
        energy: Option<&'a mut Energy>,
        factory: Option<&'a mut Factory>,
    ) -> Self {
        Self {
            dynamic,
            components: StructureComponents {
                position,
                rotation,
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

    pub(crate) fn rotate(&mut self) -> Result<(), ()> {
        if self.dynamic.rotate(&mut self.components).is_ok() {
            return Ok(());
        }
        if let Some(ref mut rotation) = self.components.rotation {
            *rotation = rotation.next();
        }
        Ok(())
    }

    pub(crate) fn set_rotation(&mut self, rotation: &Rotation) -> Result<(), ()> {
        self.components.rotation = Some(*rotation);
        Ok(())
    }
}
