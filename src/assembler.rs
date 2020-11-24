use super::items::get_item_image_url;
use super::structure::{DynIterMut, Structure};
use super::{
    DropItem, FactorishState, FrameProcResult, Inventory, InventoryTrait, ItemType, Position,
    Recipe, serialize_impl,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

fn generate_item_image(item_image: &str, icon_size: bool, count: usize) -> String {
    let size = 32;
    format!("<div style=\"background-image: url('{}'); width: {}px; height: {}px; display: inline-block\"     draggable='false'>{}</div>",
        item_image, size, size,
    if icon_size && 0 < count {
        format!("<span class='overlay noselect' style='position: relative; display: inline-block; width: {}px; height: {}px'>{}</span>", size, size, count)
    } else {
        "".to_string()
    })
}

fn _recipe_html(state: &FactorishState, recipe: &Recipe) -> String {
    let mut ret = String::from("");
    ret += "<div class='recipe-box'>";
    ret += &format!(
        "<span style='display: inline-block; margin: 1px'>{}</span>",
        &generate_item_image("time", true, recipe.recipe_time as usize)
    );
    ret += "<span style='display: inline-block; width: 50%'>";
    for (key, value) in &recipe.input {
        ret += &generate_item_image(get_item_image_url(state, &key), true, *value);
    }
    ret += "</span><img src='img/rightarrow.png' style='width: 20px; height: 32px'><span style='display: inline-block; width: 10%'>";
    for (key, value) in &recipe.output {
        ret += &generate_item_image(get_item_image_url(state, &key), true, *value);
    }
    ret += "</span></div>";
    return ret;
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Assembler {
    position: Position,
    input_inventory: Inventory,
    output_inventory: Inventory,
    progress: Option<f64>,
    power: f64,
    max_power: f64,
    recipe: Option<Recipe>,
}

impl Assembler {
    pub(crate) fn new(position: &Position) -> Self {
        Assembler {
            position: *position,
            input_inventory: Inventory::new(),
            output_inventory: Inventory::new(),
            progress: None,
            power: 0.,
            max_power: 20.,
            recipe: None,
        }
    }
}

impl Structure for Assembler {
    fn name(&self) -> &str {
        "Assembler"
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
        match state.image_assembler.as_ref() {
            Some(img) => {
                let sx = if self.progress.is_some() && 0. < self.power {
                    ((((state.sim_time * 5.) as isize) % 4) * 32) as f64
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
            None => return Err(JsValue::from_str("assembler image not available")),
        }

        Ok(())
    }

    fn desc(&self, _state: &FactorishState) -> String {
        format!(
            "{}<br>{}{}",
            if let Some(recipe) = &self.recipe {
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
                + &generate_item_image(&_state.image_time.as_ref().unwrap().url, true, recipe.recipe_time as usize) + "<br>" +
                "Outputs: <br>" +
                &recipe.output.iter()
                    .map(|item| format!("{}<br>", &generate_item_image(get_item_image_url(_state, &item.0), true, *item.1)))
                    .fold::<String, _>("".to_string(), |a, s| a + &s)
            } else {
                String::from("No recipe")
            },
            format!("Input Items: <br>{}", self.input_inventory.describe()),
            format!("Output Items: <br>{}", self.output_inventory.describe())
        )
    }

    fn frame_proc(
        &mut self,
        _state: &mut FactorishState,
        structures: &mut dyn DynIterMut<Item = Box<dyn Structure>>,
    ) -> Result<FrameProcResult, ()> {
        if let Some(recipe) = &self.recipe {
            let mut ret = FrameProcResult::None;
            // First, check if we need to refill the energy buffer in order to continue the current work.
            // Refill the energy from the fuel
            if self.power < recipe.power_cost {
                let mut accumulated = 0.;
                for structure in structures.dyn_iter_mut() {
                    let target_position = structure.position();
                    if 3 < (target_position.x - self.position.x)
                        .abs()
                        .max((target_position.y - self.position.y).abs())
                    {
                        continue;
                    }
                    let demand = self.max_power - self.power - accumulated;
                    // Energy transmission is instantaneous
                    if let Some(energy) = structure.power_outlet(demand) {
                        accumulated += energy;
                        // console_log!("draining {:?}kJ of energy with {:?} demand, from {:?}, accumulated {:?}", energy, demand, structure.name(), accumulated);
                    }
                }
                self.power += accumulated;
                self.input_inventory.remove_item(&ItemType::CoalOre);
                ret = FrameProcResult::InventoryChanged(self.position);
            }

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
                        self.output_inventory
                            .add_items(&output_item.0, *output_item.1);
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
        if let Some(recipe) = &self.recipe {
            if 0 < recipe.input.count_item(&o.type_) || 0 < recipe.output.count_item(&o.type_) {
                self.input_inventory.add_item(&o.type_);
                return Ok(());
            } else {
                return Err(JsValue::from_str("Item is not part of recipe"));
            }
        }
        Err(JsValue::from_str("Recipe is not initialized"))
    }

    fn can_output(&self) -> Inventory {
        self.output_inventory.clone()
    }

    fn output(&mut self, _state: &mut FactorishState, item_type: &ItemType) -> Result<(), ()> {
        if self.output_inventory.remove_item(item_type) {
            Ok(())
        } else {
            Err(())
        }
    }

    fn inventory(&self, is_input: bool) -> Option<&Inventory> {
        Some(if is_input {
            &self.input_inventory
        } else {
            &self.output_inventory
        })
    }

    fn inventory_mut(&mut self, is_input: bool) -> Option<&mut Inventory> {
        Some(if is_input {
            &mut self.input_inventory
        } else {
            &mut self.output_inventory
        })
    }

    fn destroy_inventory(&mut self) -> Inventory {
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

    fn get_recipes(&self) -> Vec<Recipe> {
        vec![
            Recipe::new(
                hash_map!(ItemType::IronPlate => 2usize),
                hash_map!(ItemType::Gear => 1usize),
                20.,
                50.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 1usize, ItemType::Gear => 1usize),
                hash_map!(ItemType::TransportBelt => 1usize),
                20.,
                50.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 5usize),
                hash_map!(ItemType::Chest => 1usize),
                20.,
                50.,
            ),
            Recipe::new(
                hash_map!(ItemType::CopperPlate => 1usize),
                hash_map!(ItemType::CopperWire => 2usize),
                20.,
                20.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 1, ItemType::CopperWire => 3usize),
                hash_map!(ItemType::Circuit => 1usize),
                20.,
                50.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 5, ItemType::Gear => 5, ItemType::Circuit => 3),
                hash_map!(ItemType::Assembler => 1),
                20.,
                120.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 1, ItemType::Gear => 1, ItemType::Circuit => 1),
                hash_map!(ItemType::Inserter => 1),
                20.,
                20.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 1, ItemType::Gear => 5, ItemType::Circuit => 3),
                hash_map!(ItemType::OreMine => 1),
                100.,
                100.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 2),
                hash_map!(ItemType::Pipe => 1),
                20.,
                20.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 2, ItemType::CopperPlate => 3),
                hash_map!(ItemType::WaterWell => 1),
                100.,
                100.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 5, ItemType::CopperPlate => 5),
                hash_map!(ItemType::Boiler => 1),
                100.,
                100.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 5, ItemType::Gear => 5, ItemType::CopperPlate => 5),
                hash_map!(ItemType::SteamEngine => 1),
                200.,
                200.,
            ),
        ]
    }

    fn select_recipe(&mut self, index: usize) -> Result<bool, JsValue> {
        self.recipe = Some(
            self.get_recipes()
                .get(index)
                .ok_or(js_str!("recipes index out of bound {:?}", index))?
                .clone(),
        );
        Ok(true)
    }

    fn get_selected_recipe(&self) -> Option<&Recipe> {
        self.recipe.as_ref()
    }

    serialize_impl!();
}
