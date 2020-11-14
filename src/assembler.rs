use super::items::get_item_image_url;
use super::structure::Structure;
use super::{
    DropItem, FactorishState, FrameProcResult, Inventory, InventoryTrait, ItemType, Position,
    Recipe, COAL_POWER,
};
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

pub(crate) struct Assembler {
    position: Position,
    inventory: Inventory,
    progress: Option<f64>,
    power: f64,
    max_power: f64,
    recipe: Option<Recipe>,
}

impl Assembler {
    pub(crate) fn new(position: &Position) -> Self {
        Assembler {
            position: *position,
            inventory: Inventory::new(),
            progress: None,
            power: 20.,
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
                context.draw_image_with_image_bitmap(&img.bitmap, x, y)?;
            }
            None => return Err(JsValue::from_str("assembler image not available")),
        }

        Ok(())
    }

    fn desc(&self, _state: &FactorishState) -> String {
        format!(
            "{}<br>{}",
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
                    .map(|item| format!("{}<br>", &generate_item_image(get_item_image_url(_state, &item.0), true, 1)))
                    .fold::<String, _>("".to_string(), |a, s| a + &s)
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
        _structures: &mut dyn Iterator<Item = &mut Box<dyn Structure>>,
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

            if self.progress.is_none() {
                // First, check if we have enough ingredients to finish this recipe.
                // If we do, consume the ingredients and start the progress timer.
                // We can't start as soon as the recipe is set because we may not have enough ingredients
                // at the point we set the recipe.
                if recipe
                    .input
                    .iter()
                    .map(|(item, count)| count <= &self.inventory.count_item(item))
                    .all(|b| b)
                {
                    for (item, count) in &recipe.input {
                        self.inventory.remove_items(item, *count);
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
        if let Some(recipe) = &self.recipe {
            if 0 < recipe.input.count_item(&o.type_) || 0 < recipe.output.count_item(&o.type_) {
                self.inventory.add_item(&o.type_);
                return Ok(());
            } else {
                return Err(JsValue::from_str("Item is not part of recipe"));
            }
        }
        Err(JsValue::from_str("Recipe is not initialized"))
    }

    fn output<'a>(
        &'a mut self,
        state: &mut FactorishState,
        position: &Position,
    ) -> Result<(DropItem, Box<dyn FnOnce(&DropItem) + 'a>), ()> {
        if let Some(ref mut item) = self.inventory.iter_mut().next() {
            if 0 < *item.1 {
                let item_type = *item.0;
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

    fn get_recipes(&self) -> Vec<Recipe> {
        vec![
            Recipe {
                input: [(ItemType::IronPlate, 2usize)]
                    .iter()
                    .map(|(k, v)| (*k, *v))
                    .collect(),
                output: [(ItemType::Gear, 1usize)]
                    .iter()
                    .map(|(k, v)| (*k, *v))
                    .collect(),
                power_cost: 20.,
                recipe_time: 50.,
            },
            Recipe {
                input: [(ItemType::IronPlate, 1usize), (ItemType::Gear, 1usize)]
                    .iter()
                    .map(|(k, v)| (*k, *v))
                    .collect(),
                output: [(ItemType::TransportBelt, 1usize)]
                    .iter()
                    .map(|(k, v)| (*k, *v))
                    .collect(),
                power_cost: 20.,
                recipe_time: 50.,
            },
            Recipe {
                input: [(ItemType::IronPlate, 5usize)]
                    .iter()
                    .map(|(k, v)| (*k, *v))
                    .collect(),
                output: [(ItemType::Chest, 1usize)]
                    .iter()
                    .map(|(k, v)| (*k, *v))
                    .collect(),
                power_cost: 20.,
                recipe_time: 50.,
            },
        ]
    }

    fn select_recipe(&mut self, index: usize) -> Result<bool, JsValue> {
        self.recipe = Some(
            self.get_recipes()
                .get(index)
                .ok_or(js_err!("recipes index out of bound {:?}", index))?
                .clone(),
        );
        Ok(true)
    }

    fn get_selected_recipe(&self) -> Option<&Recipe> {
        self.recipe.as_ref()
    }
}
