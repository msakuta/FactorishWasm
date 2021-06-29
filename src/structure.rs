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
    pub rotation: Option<Rotation>,
    pub burner: Option<Burner>,
    pub energy: Option<Energy>,
    pub factory: Option<Factory>,
}

impl StructureComponents {
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

impl Default for StructureComponents {
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

#[derive(Debug, Copy, Clone)]
pub(crate) struct StructureId(u32, u32);

pub type VecStorage<T> = Vec<Option<T>>;
pub struct DenseVecStorage<T> {
    indices: Vec<usize>,
    storage: Vec<Option<T>>,
}

impl<T> DenseVecStorage<T> {
    fn new(size: usize) -> Self {
        Self {
            indices: vec![0; size],
            storage: vec![],
        }
    }

    fn add(&mut self, id: StructureId, component: T) {
        self.indices[id.0 as usize] = self.storage.len();
        self.storage.push(Some(component));
    }

    fn remove(&mut self, id: StructureId) {
        let storage_idx = self.indices[id.0 as usize];
        if self.storage.len() <= storage_idx {
            return;
        }
        self.storage[storage_idx] = None;
    }

    fn clear(&mut self) {
        self.indices.iter_mut().for_each(|i| *i = 0);
        self.storage.clear();
    }

    fn get(&self, idx: u32) -> Option<&T> {
        let storage_idx = self.indices[idx as usize];
        if self.storage.len() <= storage_idx {
            return None;
        }
        self.storage[storage_idx].as_ref()
    }

    fn get_mut(&mut self, idx: u32) -> Option<&mut T> {
        let storage_idx = self.indices[idx as usize];
        if self.storage.len() <= storage_idx {
            return None;
        }
        self.storage[storage_idx].as_mut()
    }
}

impl<T> Default for DenseVecStorage<T> {
    fn default() -> Self {
        Self {
            indices: vec![],
            storage: vec![],
        }
    }
}

pub(crate) struct StructureStorage {
    size: usize,
    pub alive: Vec<bool>,
    pub generation: Vec<u32>,
    pub dynamic: Vec<Option<Box<dyn Structure>>>,
    pub position: VecStorage<Position>,
    pub rotation: VecStorage<Rotation>,
    pub burner: DenseVecStorage<Burner>,
    pub energy: DenseVecStorage<Energy>,
    pub factory: DenseVecStorage<Factory>,
}

impl StructureStorage {
    pub fn new(size: usize) -> Self {
        Self {
            size,
            alive: vec![false; size],
            generation: vec![0; size],
            // Can't use vec! macro to None-initialize since it requires Clone trait
            dynamic: (0..size).map(|_| None).collect(),
            position: vec![None; size],
            rotation: vec![None; size],
            burner: DenseVecStorage::new(size),
            energy: DenseVecStorage::new(size),
            factory: DenseVecStorage::new(size),
        }
    }

    pub fn from_vec(vec: Vec<StructureBundle>) -> Self {
        let mut ret = Self::new(64);
        for s in vec {
            ret.add(s);
        }
        ret
    }

    pub fn add(&mut self, bundle: StructureBundle) {
        if let Some((idx, b)) = self.alive.iter_mut().enumerate().find(|b| !*b.1) {
            *b = true;
            self.generation[idx] += 1;
            let id = StructureId(idx as u32, self.generation[idx]);
            self.dynamic[idx] = Some(bundle.dynamic);
            self.position[idx] = bundle.components.position;
            self.rotation[idx] = bundle.components.rotation;
            if let Some(burner) = bundle.components.burner {
                self.burner.add(id, burner);
            }
            if let Some(energy) = bundle.components.energy {
                self.energy.add(id, energy);
            }
            if let Some(factory) = bundle.components.factory {
                self.factory.add(id, factory);
            }
        }
    }

    pub fn iter(&self) -> StructureIterator {
        StructureIterator {
            storage: &self,
            idx: 0,
        }
    }

    pub fn get_mut(&mut self, idx: u32) -> Option<StructureMut> {
        if (idx as usize) < self.alive.len() && self.alive[idx as usize] {
            Some(StructureMut {
                dynamic: self.dynamic[idx as usize].unwrap().as_mut(),
                components: ComponentsMut {
                    position: self.position[idx as usize].as_mut(),
                    rotation: self.rotation[idx as usize].as_mut(),
                    burner: self.burner.get_mut(idx),
                    energy: self.energy.get_mut(idx),
                    factory: self.factory.get_mut(idx),
                }
            })
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.alive.iter_mut().for_each(|b| *b = false);
        self.dynamic.iter_mut().for_each(|s| *s = None);
        self.position.iter_mut().for_each(|p| *p = None);
        self.rotation.iter_mut().for_each(|p| *p = None);
        self.burner.clear();
        self.energy.clear();
        self.factory.clear();
    }
}

impl Default for StructureStorage {
    fn default() -> Self {
        StructureStorage {
            size: 0,
            alive: vec![],
            generation: vec![],
            dynamic: vec![],
            position: vec![],
            rotation: vec![],
            burner: DenseVecStorage::default(),
            energy: DenseVecStorage::default(),
            factory: DenseVecStorage::default(),
        }
    }
}

impl<'a> IntoIterator for &'a StructureStorage {
    type Item = StructureRef<'a>;
    type IntoIter = StructureIterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        StructureIterator {
            storage: self,
            idx: 0,
        }
    }
}

pub(crate) struct ComponentsMut<'a> {
    pub position: Option<&'a mut Position>,
    pub rotation: Option<&'a mut Rotation>,
    pub burner: Option<&'a mut Burner>,
    pub energy: Option<&'a mut Energy>,
    pub factory: Option<&'a mut Factory>,
}

pub(crate) struct StructureMut<'a> {
    pub dynamic: &'a mut dyn Structure,
    pub components: ComponentsMut<'a>,
}

impl<'a> StructureMut<'a> {
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

pub(crate) struct ComponentsRef<'a> {
    pub position: Option<&'a Position>,
    pub rotation: Option<&'a Rotation>,
    pub burner: Option<&'a Burner>,
    pub energy: Option<&'a Energy>,
    pub factory: Option<&'a Factory>,
}

pub(crate) struct StructureRef<'a> {
    pub dynamic: &'a dyn Structure,
    pub components: ComponentsRef<'a>,
}

pub(crate) struct StructureIterator<'a> {
    storage: &'a StructureStorage,
    idx: u32,
}

impl<'a> Iterator for StructureIterator<'a> {
    type Item = StructureRef<'a>;

    fn next(&mut self) -> Option<StructureRef<'a>> {
        while !self.storage.alive[self.idx as usize] {
            if self.storage.size <= self.idx as usize {
                return None;
            }
        }
        Some(StructureRef {
            dynamic: self.storage.dynamic[self.idx as usize].unwrap().as_ref(),
            components: ComponentsRef {
                position: self.storage.position[self.idx as usize].as_ref(),
                rotation: self.storage.rotation[self.idx as usize].as_ref(),
                burner: self.storage.burner.get(self.idx),
                energy: self.storage.energy.get(self.idx),
                factory: self.storage.factory.get(self.idx),
            }
        })
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
        rotation: Option<Rotation>,
        burner: Option<Burner>,
        energy: Option<Energy>,
        factory: Option<Factory>,
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
