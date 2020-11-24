use super::pipe::Pipe;
use super::structure::{DynIterMut, Structure};
use super::water_well::{FluidBox, FluidType};
use super::{
    DropItem, FactorishState, FrameProcResult, Inventory, InventoryTrait, ItemType, Position,
    Recipe, TempEnt, COAL_POWER, serialize_impl
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub(crate) struct Boiler {
    position: Position,
    inventory: Inventory,
    progress: Option<f64>,
    power: f64,
    max_power: f64,
    recipe: Option<Recipe>,
    input_fluid_box: FluidBox,
    output_fluid_box: FluidBox,
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
                input_fluid: Some(FluidType::Water),
                output: HashMap::new(),
                output_fluid: Some(FluidType::Steam),
                power_cost: 100.,
                recipe_time: 30.,
            }),
            input_fluid_box: FluidBox::new(true, false, [false; 4]),
            output_fluid_box: FluidBox::new(false, true, [false; 4]),
        }
    }

    const FLUID_PER_PROGRESS: f64 = 100.;
    const COMBUSTION_EPSILON: f64 = 1e-6;

    fn combustion_rate(&self) -> f64 {
        if let Some(ref recipe) = self.recipe {
            (self.power / recipe.power_cost)
                .min(1. / recipe.recipe_time)
                .min(self.input_fluid_box.amount / Self::FLUID_PER_PROGRESS)
                .min(
                    (self.output_fluid_box.max_amount - self.output_fluid_box.amount)
                        / Self::FLUID_PER_PROGRESS,
                )
                .min(1.)
        } else {
            0.
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
        Pipe::draw_int(self, state, context, depth, false)?;
        let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
        match state.image_boiler.as_ref() {
            Some(img) => {
                let sx = if self.progress.is_some()
                    && Self::COMBUSTION_EPSILON < self.combustion_rate()
                {
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
                format!("{}{}{}{}Input fluid: {}Output fluid: {}",
                    format!("Progress: {:.0}%<br>", self.progress.unwrap_or(0.) * 100.),
                    "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
                    format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                        self.progress.unwrap_or(0.) * 100.),
                    format!(r#"Power: {:.1}kJ <div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>
                    <div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>"#,
                    self.power,
                    if 0. < self.max_power { (self.power) / self.max_power * 100. } else { 0. }),
                    self.input_fluid_box.desc(),
                    self.output_fluid_box.desc())
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
        state: &mut FactorishState,
        structures: &mut dyn DynIterMut<Item = Box<dyn Structure>>,
    ) -> Result<FrameProcResult, ()> {
        let connections = self.connection(state, structures.as_dyn_iter());
        self.output_fluid_box.connect_to = connections;
        self.input_fluid_box
            .simulate(&self.position, state, &mut structures.dyn_iter_mut());
        self.output_fluid_box
            .simulate(&self.position, state, &mut structures.dyn_iter_mut());
        if let Some(recipe) = &self.recipe {
            if self.input_fluid_box.type_ == Some(FluidType::Water) {
                self.progress = Some(0.);
            }
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
                let progress = self.combustion_rate();
                if state.rng.next() < progress * 10. {
                    state
                        .temp_ents
                        .push(TempEnt::new(&mut state.rng, self.position));
                }
                if 1. <= prev_progress + progress {
                    self.progress = None;

                    // Produce outputs into inventory
                    for output_item in &recipe.output {
                        self.inventory.add_item(&output_item.0);
                    }
                    return Ok(FrameProcResult::InventoryChanged(self.position));
                } else if Self::COMBUSTION_EPSILON < progress {
                    self.progress = Some(prev_progress + progress);
                    self.power -= progress * recipe.power_cost;
                    self.output_fluid_box.type_ = Some(FluidType::Steam);
                    self.output_fluid_box.amount += progress * Self::FLUID_PER_PROGRESS;
                    self.input_fluid_box.amount -= progress * Self::FLUID_PER_PROGRESS;
                }
            }
            return Ok(ret);
        }
        Ok(FrameProcResult::None)
    }

    fn input(&mut self, o: &DropItem) -> Result<(), JsValue> {
        // Fuels are always welcome.
        if o.type_ == ItemType::CoalOre {
            self.inventory.add_item(&ItemType::CoalOre);
            return Ok(());
        }

        Err(JsValue::from_str("Recipe is not initialized"))
    }

    fn can_input(&self, item_type: &ItemType) -> bool {
        *item_type == ItemType::CoalOre
    }

    fn output(&mut self, _state: &mut FactorishState, item_type: &ItemType) -> Result<(), ()> {
        if self.inventory.remove_item(item_type) {
            Ok(())
        } else {
            Err(())
        }
    }

    fn inventory(&self, is_input: bool) -> Option<&Inventory> {
        if is_input {
            Some(&self.inventory)
        } else {
            None
        }
    }

    fn inventory_mut(&mut self, is_input: bool) -> Option<&mut Inventory> {
        if is_input {
            Some(&mut self.inventory)
        } else {
            None
        }
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

    fn fluid_box(&self) -> Option<Vec<&FluidBox>> {
        Some(vec![&self.input_fluid_box, &self.output_fluid_box])
    }

    fn fluid_box_mut(&mut self) -> Option<Vec<&mut FluidBox>> {
        Some(vec![&mut self.input_fluid_box, &mut self.output_fluid_box])
    }

    serialize_impl!();
}
