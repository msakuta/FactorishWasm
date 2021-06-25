use super::{
    items::{DropItem, ItemType},
    structure::Structure,
    FrameProcResult, Inventory, InventoryTrait, COAL_POWER,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::JsValue;

#[derive(Serialize, Deserialize)]
pub(crate) struct Burner {
    pub inventory: Inventory,
    pub capacity: usize,
    pub energy: f64,
    pub max_energy: f64,
}

impl Burner {
    pub fn js_serialize(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
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

    pub fn frame_proc(&mut self, structure: &mut dyn Structure) -> Result<FrameProcResult, ()> {
        if let Some(amount) = self.inventory.get_mut(&ItemType::CoalOre) {
            if 0 < *amount && self.energy == 0. {
                self.inventory.remove_item(&ItemType::CoalOre);
                self.energy += COAL_POWER;
                self.max_energy = self.max_energy.max(self.energy);
                return Ok(FrameProcResult::InventoryChanged(*structure.position()));
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
