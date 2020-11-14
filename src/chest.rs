use super::items::{DropItem, ItemType};
use super::structure::{ItemResponse, ItemResponseResult, Structure};
use super::{FactorishState, FrameProcResult, Inventory, InventoryTrait, Position};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

const CHEST_CAPACITY: usize = 100;

pub(crate) struct Chest {
    position: Position,
    inventory: Inventory,
}

impl Chest {
    pub(crate) fn new(position: &Position) -> Self {
        Chest {
            position: *position,
            inventory: Inventory::new(),
        }
    }
}

impl Structure for Chest {
    fn name(&self) -> &'static str {
        "Chest"
    }

    fn position(&self) -> &Position {
        &self.position
    }

    fn draw(
        &self,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
        match state.image_chest.as_ref() {
            Some(img) => {
                context.draw_image_with_image_bitmap(&img.bitmap, x, y)?;
                Ok(())
            }
            None => Err(JsValue::from_str("chest image not available")),
        }
    }

    fn desc(&self, _state: &FactorishState) -> String {
        format!(
            "Items: \n{}",
            self.inventory
                .iter()
                .map(|item| format!("{:?}: {}<br>", item.0, item.1))
                .fold(String::from(""), |accum, item| accum + &item)
        )
    }

    fn item_response(&mut self, _item: &DropItem) -> Result<ItemResponseResult, ()> {
        if self.inventory.len() < CHEST_CAPACITY {
            self.inventory.add_item(&_item.type_);
            Ok((
                ItemResponse::Consume,
                Some(FrameProcResult::InventoryChanged(self.position)),
            ))
        } else {
            Err(())
        }
    }

    fn input(&mut self, o: &DropItem) -> Result<(), JsValue> {
        self.item_response(o)
            .map(|_| ())
            .map_err(|_| JsValue::from_str("ItemResponse failed"))
    }

    /// Chest can put any item
    fn can_input(&self, _o: &ItemType) -> bool {
        self.inventory.len() < CHEST_CAPACITY
    }

    fn output<'a>(
        &'a mut self,
        state: &mut FactorishState,
        position: &Position,
    ) -> Result<(DropItem, Box<dyn FnOnce(&DropItem) + 'a>), ()> {
        if let Some(ref mut item) = self.inventory.iter_mut().next() {
            if 0 < *item.1 {
                let item_type = item.0.clone();
                Ok((
                    DropItem {
                        id: state.serial_no,
                        type_: *item.0,
                        x: position.x * 32,
                        y: position.y * 32,
                    },
                    Box::new(move |_| {
                        self.inventory.remove_item(&item_type);
                    }),
                ))
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    fn inventory(&self) -> Option<&Inventory> {
        Some(&self.inventory)
    }

    fn inventory_mut(&mut self) -> Option<&mut Inventory> {
        Some(&mut self.inventory)
    }
}
