use super::structure::{DynIterMut, Structure};
use super::water_well::FluidBox;
use super::{
    log, DropItem, FactorishState, FrameProcResult, Inventory, InventoryTrait, ItemType, Position,
    Recipe, COAL_POWER,
};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

use std::collections::HashMap;

pub(crate) struct Boiler {
    position: Position,
    inventory: Inventory,
    progress: Option<f64>,
    power: f64,
    max_power: f64,
    recipe: Option<Recipe>,
    _input_fluid_box: FluidBox,
    _output_fluid_box: FluidBox,
}

impl Boiler {
    pub(crate) fn new(position: &Position) -> Self {
        Boiler {
            position: *position,
            inventory: Inventory::new(),
            progress: None,
            power: 0.,
            max_power: 20.,
            recipe: Some(Recipe {
                input: hash_map!(ItemType::CoalOre => 1usize),
                output: HashMap::new(),
                power_cost: 0.,
                recipe_time: 30.,
            }),
            _input_fluid_box: FluidBox::new(true, false, [false; 4]),
            _output_fluid_box: FluidBox::new(false, true, [false; 4]),
        }
    }
}

impl Structure for Boiler {
    fn name(&self) -> &str {
        "Boiler"
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
        match state.image_boiler.as_ref() {
            Some(img) => {
                let sx = if self.progress.is_some() && 0. < self.power {
                    ((((state.sim_time * 5.) as isize) % 2 + 1) * 32) as f64
                } else {
                    0.
                };
                context.draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    &img.bitmap,
                    sx,
                    0.,
                    32.,
                    32.,
                    x,
                    y,
                    32.,
                    32.,
                )?;
            }
            None => return Err(JsValue::from_str("furnace image not available")),
        }

        Ok(())
    }

    fn desc(&self, _state: &FactorishState) -> String {
        format!(
            "{}<br>{}",
            if self.recipe.is_some() {
                // Progress bar
                format!("{}{}{}{}",
                    format!("Progress: {:.0}%<br>", self.progress.unwrap_or(0.) * 100.),
                    "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
                    format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                        self.progress.unwrap_or(0.) * 100.),
                    format!(r#"Power: {:.1}kJ <div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>
                    <div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>"#,
                    self.power,
                    if 0. < self.max_power { (self.power) / self.max_power * 100. } else { 0. }),
                    )
            // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
            // "Outputs: <br>" +
            // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
            } else {
                String::from("No recipe")
            },
            format!(
                "Items: \n{}",
                self.inventory
                    .iter()
                    .map(|item| format!("{:?}: {}<br>", item.0, item.1))
                    .fold(String::from(""), |accum, item| accum + &item)
            )
        )
    }

    fn frame_proc(
        &mut self,
        _state: &mut FactorishState,
        _structures: &mut dyn DynIterMut<Item = Box<dyn Structure>>,
    ) -> Result<FrameProcResult, ()> {
        if let Some(recipe) = &self.recipe {
            let mut ret = FrameProcResult::None;
            // First, check if we need to refill the energy buffer in order to continue the current work.
            if self.inventory.get(&ItemType::CoalOre).is_some() {
                // Refill the energy from the fuel
                if self.power < recipe.power_cost {
                    self.power += COAL_POWER;
                    self.max_power = self.power;
                    self.inventory.remove_item(&ItemType::CoalOre);
                    ret = FrameProcResult::InventoryChanged(self.position);
                }
            }

            if let Some(prev_progress) = self.progress {
                // Proceed only if we have sufficient energy in the buffer.
                let progress = (self.power / recipe.power_cost)
                    .min(1. / recipe.recipe_time)
                    .min(1.);
                if 1. <= prev_progress + progress {
                    self.progress = None;

                    // Produce outputs into inventory
                    for output_item in &recipe.output {
                        self.inventory.add_item(&output_item.0);
                    }
                    return Ok(FrameProcResult::InventoryChanged(self.position));
                } else {
                    self.progress = Some(prev_progress + progress);
                    self.power -= progress * recipe.power_cost;
                }
            }
            return Ok(ret);
        }
        Ok(FrameProcResult::None)
    }

    fn input(&mut self, o: &DropItem) -> Result<(), JsValue> {
        console_log!("Boiler: {:?}", o.type_);
        // Fuels are always welcome.
        if o.type_ == ItemType::CoalOre {
            self.inventory.add_item(&ItemType::CoalOre);
            return Ok(());
        }

        Err(JsValue::from_str("Recipe is not initialized"))
    }

    fn can_input(&self, item_type: &ItemType) -> bool {
        if let Some(recipe) = &self.recipe {
            *item_type == ItemType::CoalOre || recipe.input.get(item_type).is_some()
        } else {
            match item_type {
                ItemType::CoalOre | ItemType::IronOre | ItemType::CopperOre => true,
                _ => false,
            }
        }
    }

    fn output(&mut self, _state: &mut FactorishState, item_type: &ItemType) -> Result<(), ()> {
        if self.inventory.remove_item(item_type) {
            Ok(())
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

    fn destroy_inventory(&mut self) -> Inventory {
        // Return the ingredients if it was in the middle of processing a recipe.
        if let Some(recipe) = self.recipe.take() {
            if self.progress.is_some() {
                let mut ret = std::mem::take(&mut self.inventory);
                ret.merge(recipe.input);
                return ret;
            }
        }
        std::mem::take(&mut self.inventory)
    }

    fn get_selected_recipe(&self) -> Option<&Recipe> {
        self.recipe.as_ref()
    }
}
