use super::{
    drop_items::DropItem,
    items::ItemType,
    structure::{Energy, Structure},
    FactorishState, FrameProcResult, Inventory, InventoryTrait, Position, COAL_POWER,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;
use web_sys::CanvasRenderingContext2d;

pub(crate) const FUEL_CAPACITY: usize = 10;

#[derive(Serialize, Deserialize)]
pub(crate) struct Burner {
    pub inventory: Inventory,
    pub capacity: usize,
}

impl Burner {
    pub fn js_serialize(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    pub fn _draw(
        &self,
        energy: Option<&Energy>,
        _position: &Position,
        state: &FactorishState,
        _context: &CanvasRenderingContext2d,
    ) -> Result<(), JsValue> {
        let energy = energy.ok_or_else(|| js_str!("Burner without Energy component"))?;
        if energy.value < 1e-3 && state.sim_time % 1. < 0.5 {
            // if let Some(img) = state.image_fuel_alarm.as_ref() {
            //     let (x, y) = (position.x as f64 * 32., position.y as f64 * 32.);
            //     context.draw_image_with_image_bitmap(&img.bitmap, x, y)?;
            // } else {
            //     return js_err!("fuel alarm image not available");
            // }
        }
        Ok(())
    }

    pub fn add_burner_inventory(&mut self, item_type: &ItemType, amount: isize) -> isize {
        if amount < 0 {
            let existing = self.inventory.count_item(item_type);
            let removed = existing.min((-amount) as usize);
            self.inventory.remove_items(item_type, removed);
            -(removed as isize)
        } else if *item_type == ItemType::CoalOre {
            let add_amount = amount
                .min((self.capacity - self.inventory.count_item(&ItemType::CoalOre)) as isize);
            self.inventory.add_items(item_type, add_amount as usize);
            add_amount
        } else {
            0
        }
    }

    pub fn frame_proc(
        &mut self,
        position: Option<&mut Position>,
        energy: Option<&mut Energy>,
        _structure: &mut dyn Structure,
    ) -> Result<FrameProcResult, ()> {
        let position = position.ok_or(())?;
        let energy = energy.ok_or(())?; //|| js_str!("Burner without Energy component"))?;
        if let Some(amount) = self.inventory.get_mut(&ItemType::CoalOre) {
            if 0 < *amount && energy.value < 1e-3 {
                self.inventory.remove_item(&ItemType::CoalOre);
                energy.value += COAL_POWER;
                energy.max = energy.max.max(energy.value);
                return Ok(FrameProcResult::InventoryChanged(*position));
            }
        }
        Ok(FrameProcResult::None)
    }

    pub fn input(&mut self, item: &DropItem) -> Result<(), JsValue> {
        // Fuels are always welcome.
        if item.type_ == ItemType::CoalOre
            && self.inventory.count_item(&ItemType::CoalOre) < self.capacity
        {
            self.inventory.add_item(&ItemType::CoalOre);
            return Ok(());
        }
        Err(JsValue::from_str("not inputtable to ore mine"))
    }

    pub fn can_input(&self, item_type: &ItemType) -> bool {
        *item_type == ItemType::CoalOre
            && self.inventory.count_item(&ItemType::CoalOre) < self.capacity
    }

    pub fn destroy_inventory(&mut self) -> Inventory {
        std::mem::take(&mut self.inventory)
    }
}
