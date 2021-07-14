use super::{
    dyn_iter::{DynIter, DynIterMut},
    items::ItemType,
    water_well::FluidBox,
    DropItem, FactorishState, Inventory, InventoryTrait, Recipe,
};
use rotate_enum::RotateEnum;
use serde::{Deserialize, Serialize};
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

#[macro_export]
macro_rules! draw_fuel_alarm {
    ($self_:expr, $state:expr, $context:expr) => {
        if $self_.recipe.is_some() && $self_.power == 0. && $state.sim_time % 1. < 0.5 {
            if let Some(img) = $state.image_fuel_alarm.as_ref() {
                let (x, y) = (
                    $self_.position.x as f64 * 32.,
                    $self_.position.y as f64 * 32.,
                );
                $context.draw_image_with_image_bitmap(&img.bitmap, x, y)?;
            } else {
                return js_err!("fuel alarm image not available");
            }
        }
    };
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
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

/// A structure that allow random access to structure array excluding single element.
/// It is convenient when you want to have mutable reference to two elements in the array at the same time.
pub(crate) struct StructureDynIter<'a> {
    left_start: usize,
    left: &'a mut [StructureEntry],
    right_start: usize,
    right: &'a mut [StructureEntry],
}

impl<'a> StructureDynIter<'a> {
    pub(crate) fn new(
        source: &'a mut [StructureEntry],
        split_idx: usize,
    ) -> Result<(&'a mut StructureEntry, Self), JsValue> {
        let (left, right) = source.split_at_mut(split_idx);
        let (center, right) = right
            .split_first_mut()
            .ok_or_else(|| JsValue::from_str("Structures split fail"))?;
        Ok((
            center,
            Self {
                left_start: 0,
                left,
                right_start: split_idx + 1,
                right,
            },
        ))
    }

    /// Accessor without generation checking.
    #[allow(dead_code)]
    pub(crate) fn get_at(&self, idx: usize) -> Option<&StructureEntry> {
        if self.left_start <= idx && idx < self.left_start + self.left.len() {
            self.left.get(idx - self.left_start)
        } else if self.right_start <= idx && idx < self.right_start + self.right.len() {
            self.right.get(idx - self.right_start)
        } else {
            None
        }
    }

    /// Mutable accessor without generation checking.
    #[allow(dead_code)]
    pub(crate) fn get_at_mut(&mut self, idx: usize) -> Option<&mut StructureEntry> {
        if self.left_start <= idx && idx < self.left_start + self.left.len() {
            self.left.get_mut(idx - self.left_start)
        } else if self.right_start <= idx && idx < self.right_start + self.right.len() {
            self.right.get_mut(idx - self.right_start)
        } else {
            None
        }
    }

    /// Accessor with generation checking.
    #[allow(dead_code)]
    pub(crate) fn get(&self, id: StructureId) -> Option<&dyn Structure> {
        let idx = id.id as usize;
        if self.left_start <= idx && idx < self.left_start + self.left.len() {
            self.left
                .get(idx - self.left_start)
                .filter(|s| s.gen == id.gen)
                .map(|s| s.dynamic.as_deref())
                .flatten()
        } else if self.right_start <= idx && idx < self.right_start + self.right.len() {
            self.right
                .get(idx - self.right_start)
                .filter(|s| s.gen == id.gen)
                .map(|s| s.dynamic.as_deref())
                .flatten()
        } else {
            None
        }
    }

    /// Mutable accessor with generation checking.
    pub(crate) fn get_mut(&mut self, id: StructureId) -> Option<&mut (dyn Structure + '_)> {
        let idx = id.id as usize;
        if self.left_start <= idx && idx < self.left_start + self.left.len() {
            self.left
                .get_mut(idx - self.left_start)
                .filter(|s| s.gen == id.gen)
                .map(|s| s.dynamic.as_deref_mut().map(|s| s as &mut dyn Structure))
                // Interestingly, we need .map(|s| s as &mut dyn Structure) to compile.
                // .map(|s| s.dynamic.as_deref_mut())
                .flatten()
        } else if self.right_start <= idx && idx < self.right_start + self.right.len() {
            self.right
                .get_mut(idx - self.right_start)
                .filter(|s| s.gen == id.gen)
                .map(|s| s.dynamic.as_deref_mut().map(|s| s as &mut dyn Structure))
                // .map(|s| s.dynamic.as_deref_mut())
                .flatten()
        } else {
            None
        }
    }
}

impl<'a> DynIter for StructureDynIter<'a> {
    type Item = dyn Structure;
    fn dyn_iter(&self) -> Box<dyn Iterator<Item = &Self::Item> + '_> {
        Box::new(
            self.left
                .iter()
                .chain(self.right.iter())
                .filter_map(|s| s.dynamic.as_deref()),
        )
    }
    fn as_dyn_iter(&self) -> &dyn DynIter<Item = Self::Item> {
        self
    }
}

impl<'a> DynIterMut for StructureDynIter<'a> {
    fn dyn_iter_mut(&mut self) -> Box<dyn Iterator<Item = &mut Self::Item> + '_> {
        Box::new(
            self.left
                .iter_mut()
                .chain(self.right.iter_mut())
                .filter_map(|s| s.dynamic.as_deref_mut()),
        )
    }
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

pub(crate) trait Structure {
    fn name(&self) -> &str;
    fn position(&self) -> &Position;
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
    fn desc(&self, _state: &FactorishState) -> String {
        String::from("")
    }
    fn frame_proc(
        &mut self,
        _state: &mut FactorishState,
        _structures: &mut StructureDynIter,
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
        _others: &dyn DynIter<Item = StructureEntry>,
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
    fn burner_inventory(&self) -> Option<&Inventory> {
        None
    }
    fn add_burner_inventory(&mut self, _item_type: &ItemType, _amount: isize) -> isize {
        0
    }
    fn burner_energy(&self) -> Option<(f64, f64)> {
        None
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
    fn select_recipe(&mut self, _index: usize) -> Result<bool, JsValue> {
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
