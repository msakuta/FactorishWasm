use super::{
    items::{DropItem, ItemType},
    structure::{Energy, Structure},
    FactorishState, FrameProcResult, Inventory, InventoryTrait, Position, COAL_POWER,
};
use serde::{Deserialize, Serialize};
use specs::{Component, DenseVecStorage, ReadStorage, System, WriteStorage};
use wasm_bindgen::JsValue;
use web_sys::CanvasRenderingContext2d;

#[derive(Serialize, Deserialize, Component)]
#[storage(DenseVecStorage)]
pub(crate) struct Burner {
    pub inventory: Inventory,
    pub capacity: usize,
}

impl Burner {
    pub fn js_serialize(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }

    pub fn draw(
        &self,
        energy: Option<&Energy>,
        position: &Position,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
    ) -> Result<(), JsValue> {
        let energy = energy.ok_or_else(|| js_str!("Burner without Energy component"))?;
        if energy.value < 1e-3 && state.sim_time % 1. < 0.5 {
            if let Some(img) = state.image_fuel_alarm.as_ref() {
                let (x, y) = (position.x as f64 * 32., position.y as f64 * 32.);
                context.draw_image_with_image_bitmap(&img.bitmap, x, y)?;
            } else {
                return js_err!("fuel alarm image not available");
            }
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

pub(crate) struct BurnerSystem {
    pub events: Vec<FrameProcResult>,
}

impl<'a> System<'a> for BurnerSystem {
    type SystemData = (
        ReadStorage<'a, Position>,
        WriteStorage<'a, Burner>,
        WriteStorage<'a, Energy>,
    );

    fn run(&mut self, (position, mut burner, mut energy): Self::SystemData) {
        use specs::Join;
        for (position, burner, energy) in (&position, &mut burner, &mut energy).join() {
            // console_log!("burner {:?}", position);
            if let Some(amount) = burner.inventory.get_mut(&ItemType::CoalOre) {
                if 0 < *amount && energy.value < 1e-3 {
                    burner.inventory.remove_item(&ItemType::CoalOre);
                    energy.value += COAL_POWER;
                    energy.max = energy.max.max(energy.value);
                    self.events
                        .push(FrameProcResult::InventoryChanged(*position));
                }
            }
        }
    }
}
