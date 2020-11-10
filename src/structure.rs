use super::{
    DropItem, FactorishState, FrameProcResult, Inventory, ItemResponseResult, Position, Rotation,
};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

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
}
