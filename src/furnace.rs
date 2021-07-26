use super::{
    items::item_to_str,
    structure::{Structure, StructureDynIter, StructureId},
    DropItem, FactorishState, FrameProcResult, Inventory, InventoryTrait, ItemType, Position,
    Recipe, TempEnt, COAL_POWER,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

const FUEL_CAPACITY: usize = 10;

#[derive(Serialize, Deserialize)]
pub(crate) struct Furnace {
    position: Position,
    input_inventory: Inventory,
    output_inventory: Inventory,
    progress: Option<f64>,
    power: f64,
    max_power: f64,
    recipe: Option<Recipe>,
}

impl Furnace {
    pub(crate) fn new(position: &Position) -> Self {
        Furnace {
            position: *position,
            input_inventory: Inventory::new(),
            output_inventory: Inventory::new(),
            progress: None,
            power: 20.,
            max_power: 20.,
            recipe: None,
        }
    }
}

impl Structure for Furnace {
    fn name(&self) -> &str {
        "Furnace"
    }

    fn position(&self) -> &Position {
        &self.position
    }

    fn draw(
        &self,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        is_toolbar: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
        match state.image_furnace.as_ref() {
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
        if !is_toolbar {
            crate::draw_fuel_alarm!(self, state, context);
        }

        Ok(())
    }

    fn desc(&self, _state: &FactorishState) -> String {
        format!(
            "{}<br>{}{}",
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
            format!("Input Items: <br>{}", self.input_inventory.describe()),
            format!("Output Items: <br>{}", self.output_inventory.describe())
        )
    }

    fn frame_proc(
        &mut self,
        _me: StructureId,
        state: &mut FactorishState,
        _structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        if let Some(recipe) = &self.recipe {
            let mut ret = FrameProcResult::None;
            // First, check if we need to refill the energy buffer in order to continue the current work.
            if self.input_inventory.get(&ItemType::CoalOre).is_some() {
                // Refill the energy from the fuel
                if self.power < recipe.power_cost {
                    self.power += COAL_POWER;
                    self.max_power = self.power;
                    self.input_inventory.remove_item(&ItemType::CoalOre);
                    ret = FrameProcResult::InventoryChanged(self.position);
                }
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
                if state.rng.next() < progress * 10. {
                    state
                        .temp_ents
                        .push(TempEnt::new(&mut state.rng, self.position));
                }
                if 1. <= prev_progress + progress {
                    self.progress = None;

                    // Produce outputs into inventory
                    for output_item in &recipe.output {
                        self.output_inventory.add_item(&output_item.0);
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
        // Fuels are always welcome.
        if o.type_ == ItemType::CoalOre
            && self.input_inventory.count_item(&ItemType::CoalOre) < FUEL_CAPACITY
        {
            self.input_inventory.add_item(&ItemType::CoalOre);
            return Ok(());
        }

        if self.recipe.is_none() {
            match o.type_ {
                ItemType::IronOre => {
                    self.recipe = Some(Recipe::new(
                        hash_map!(ItemType::IronOre => 1usize),
                        hash_map!(ItemType::IronPlate => 1usize),
                        20.,
                        50.,
                    ));
                }
                ItemType::CopperOre => {
                    self.recipe = Some(Recipe::new(
                        hash_map!(ItemType::CopperOre => 1usize),
                        hash_map!(ItemType::CopperPlate => 1usize),
                        20.,
                        50.,
                    ));
                }
                _ => {
                    return Err(JsValue::from_str(&format!(
                        "Cannot smelt {}",
                        item_to_str(&o.type_)
                    )))
                }
            }
        }

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

    fn can_input(&self, item_type: &ItemType) -> bool {
        if *item_type == ItemType::CoalOre {
            if self.input_inventory.count_item(item_type) < FUEL_CAPACITY {
                return true;
            }
        }
        if let Some(recipe) = &self.recipe {
            recipe.input.get(item_type).is_some()
        } else {
            matches!(item_type, ItemType::IronOre | ItemType::CopperOre)
        }
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
                hash_map!(ItemType::IronOre => 1usize),
                hash_map!(ItemType::IronPlate => 1usize),
                20.,
                50.,
            ),
            Recipe::new(
                hash_map!(ItemType::CopperOre => 1usize),
                hash_map!(ItemType::CopperPlate => 1usize),
                20.,
                50.,
            ),
        ]
    }

    fn get_selected_recipe(&self) -> Option<&Recipe> {
        self.recipe.as_ref()
    }

    fn serialize(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }
}
