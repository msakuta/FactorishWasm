use super::items::ItemType;
use super::water_well::FluidBox;
use super::{DropItem, FactorishState, Inventory, Recipe, log};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
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
}

impl From<&[i32; 2]> for Position {
    fn from(xy: &[i32; 2]) -> Self {
        Self { x: xy[0], y: xy[1] }
    }
}

#[derive(Copy, Clone)]
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

    pub fn next(&mut self) {
        *self = match self {
            Rotation::Left => Rotation::Top,
            Rotation::Top => Rotation::Right,
            Rotation::Right => Rotation::Bottom,
            Rotation::Bottom => Rotation::Left,
        }
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

pub(crate) trait Structure {
    fn name(&self) -> &str;
    fn position(&self) -> &Position;
    fn draw(
        &self,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
    ) -> Result<(), JsValue>;
    fn desc(&self, _state: &FactorishState) -> String {
        String::from("")
    }
    fn frame_proc(
        &mut self,
        _state: &mut FactorishState,
        _structures: &mut dyn Iterator<Item = &mut Box<dyn Structure>>,
    ) -> Result<FrameProcResult, ()> {
        Ok(FrameProcResult::None)
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
    fn output<'a>(
        &'a mut self,
        _state: &mut FactorishState,
        _position: &Position,
    ) -> Result<(DropItem, Box<dyn FnOnce(&DropItem) + 'a>), ()> {
        Err(())
    }
    fn inventory(&self) -> Option<&Inventory> {
        None
    }
    fn inventory_mut(&mut self) -> Option<&mut Inventory> {
        None
    }
    /// Some structures don't have an inventory, but still can have some item, e.g. inserter hands.
    /// We need to retrieve them when we destory such a structure, or we might lose items into void.
    /// It will take away the inventory by default, destroying the instance's inventory.
    fn destroy_inventory(&mut self) -> Inventory {
        self.inventory_mut()
            .map_or(Inventory::new(), |inventory| std::mem::take(inventory))
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
    fn fluid_box(&self) -> Option<&FluidBox> {
        None
    }
    fn fluid_box_mut(&mut self) -> Option<&mut FluidBox> {
        None
    }
    fn connection(&self, state: &FactorishState, structures: &mut dyn Iterator<Item = &Box<dyn Structure>>) -> u32 {
        // let mut structures_copy = structures.clone();
        let mut has_fluid_box = |x, y| {
            if x < 0 || state.width <= x as u32 || y < 0 || state.height <= y as u32 {
                return false;
            }
            if let Some(structure) = structures.map(|s| s).find(|s| *s.position() == Position{x, y}) {
                return structure.fluid_box().is_some();
            }
            return false;
        };

        // Fluid containers connect to other containers
        let Position { x, y } = *self.position();
        let l = has_fluid_box(x - 1, y) as u32;
        let t = has_fluid_box(x, y - 1) as u32;
        let r = has_fluid_box(x + 1, y) as u32;
        let b = has_fluid_box(x, y + 1) as u32;
        console_log!("connection {:?}", [l, t, r, b]);
        return l | (t << 1) | (r << 2) | (b << 3);
    }
}
