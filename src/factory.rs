use super::{
    drop_items::DropItem,
    inventory::InventoryType,
    items::ItemType,
    structure::{Energy, Position},
    FactorishState, FrameProcResult, Inventory, InventoryTrait, Recipe,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Serialize, Deserialize)]
pub(crate) struct Factory {
    pub input_inventory: Inventory,
    pub output_inventory: Inventory,
    pub recipe: Option<Recipe>,
    pub progress: Option<f64>,
}

impl Factory {
    pub fn new() -> Self {
        Self {
            input_inventory: Inventory::new(),
            output_inventory: Inventory::new(),
            recipe: None,
            progress: None,
        }
    }

    pub fn frame_proc(
        &mut self,
        position: Option<&mut Position>,
        energy: Option<&mut Energy>,
    ) -> Result<FrameProcResult, ()> {
        let position = position.ok_or(())?;
        let energy = energy.ok_or(())?;
        if let Some(recipe) = &self.recipe {
            let mut ret = FrameProcResult::None;

            if self.progress.is_none() {
                // First, check if we have enough ingredients to finish this recipe.
                // If we do, consume the ingredients and start the progress timer.
                // We can't start as soon as the recipe is set because we may not have enough ingredients
                // at the point we set the recipe.
                if recipe
                    .input
                    .iter()
                    .map(|(item, count)| count <= &self.input_inventory.count_item(item))
                    .all(|b| b)
                {
                    for (item, count) in &recipe.input {
                        self.input_inventory.remove_items(item, *count);
                    }
                    self.progress = Some(0.);
                    ret = FrameProcResult::InventoryChanged(*position);
                }
            }

            if let Some(prev_progress) = self.progress {
                // Proceed only if we have sufficient energy in the buffer.
                let progress = (energy.value / recipe.power_cost)
                    .min(1. / recipe.recipe_time)
                    .min(1.);
                if 1. <= prev_progress + progress {
                    self.progress = None;

                    // Produce outputs into inventory
                    for output_item in &recipe.output {
                        self.output_inventory
                            .add_items(&output_item.0, *output_item.1);
                    }
                    return Ok(FrameProcResult::InventoryChanged(*position));
                } else {
                    self.progress = Some(prev_progress + progress);
                    energy.value -= progress * recipe.power_cost;
                }
            }
            return Ok(ret);
        }
        Ok(FrameProcResult::None)
    }

    pub fn input(&mut self, item: &DropItem) -> Result<(), JsValue> {
        if let Some(recipe) = &self.recipe {
            if 0 < recipe.input.count_item(&item.type_) || 0 < recipe.output.count_item(&item.type_)
            {
                self.input_inventory.add_item(&item.type_);
                return Ok(());
            } else {
                return Err(JsValue::from_str("Item is not part of recipe"));
            }
        }
        Ok(())
    }

    pub fn can_output(&self) -> Inventory {
        self.output_inventory.clone()
    }

    pub fn output(&mut self, _state: &mut FactorishState, item_type: &ItemType) -> Result<(), ()> {
        if self.output_inventory.remove_item(item_type) {
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn inventory(&self, inventory_type: InventoryType) -> Option<&Inventory> {
        match inventory_type {
            InventoryType::Input => Some(&self.input_inventory),
            InventoryType::Output => Some(&self.output_inventory),
            _ => None,
        }
    }

    pub fn inventory_mut(&mut self, inventory_type: InventoryType) -> Option<&mut Inventory> {
        match inventory_type {
            InventoryType::Input => Some(&mut self.input_inventory),
            InventoryType::Output => Some(&mut self.output_inventory),
            _ => None,
        }
    }

    pub fn destroy_inventory(&mut self) -> Inventory {
        let mut ret = std::mem::take(&mut self.input_inventory);
        ret.merge(std::mem::take(&mut self.output_inventory));
        // Return the ingredients if it was in the middle of processing a recipe.
        if let Some(mut recipe) = self.recipe.take() {
            if self.progress.is_some() {
                ret.merge(std::mem::take(&mut recipe.input));
            }
        }
        ret
    }
}
