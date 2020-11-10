use super::structure::Structure;
use super::{
    item_to_str, DropItem, FactorishState, FrameProcResult, Inventory, InventoryTrait, ItemType,
    Position, Recipe, COAL_POWER,
};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

pub(crate) struct Furnace {
    position: Position,
    inventory: Inventory,
    progress: f64,
    power: f64,
    max_power: f64,
    recipe: Option<Recipe>,
}

impl Furnace {
    pub(crate) fn new(position: &Position) -> Self {
        Furnace {
            position: *position,
            inventory: Inventory::new(),
            progress: 0.,
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
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
        match state.image_furnace.as_ref() {
            Some(img) => {
                let sx = if 0. < self.progress && 0. < self.power {
                    ((((state.sim_time * 5.) as isize) % 2 + 1) * 32) as f64
                } else {
                    0.
                };
                context.draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    img, sx, 0., 32., 32., x, y, 32., 32.,
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
                    format!("Progress: {:.0}%<br>", self.progress * 100.),
                    "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
                    format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                        self.progress * 100.),
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

            // Proceed only if we have sufficient energy in the buffer.
            let progress = (self.power / recipe.power_cost)
                .min(1. / recipe.recipe_time)
                .min(1.);
            if 1. <= self.progress + progress {
                self.progress = 0.;
                let has_ingredients = recipe
                    .input
                    .iter()
                    .map(|consume_item| {
                        if let Some(entry) = self.inventory.get(&consume_item.0) {
                            *consume_item.1 <= *entry
                        } else {
                            false
                        }
                    })
                    .all(|v| v);

                // First, check if we have enough ingredients to finish this recipe.
                if !has_ingredients {
                    self.recipe = None;
                    return Ok(FrameProcResult::None);
                }

                // Consume inputs from inventory
                for consume_item in &recipe.input {
                    self.inventory.remove_item(&consume_item.0);
                }

                // Produce outputs into inventory
                for output_item in &recipe.output {
                    self.inventory.add_item(&output_item.0);
                }
                return Ok(FrameProcResult::InventoryChanged(self.position));
            } else {
                self.progress += progress;
                self.power -= progress * recipe.power_cost;
            }
            return Ok(ret);
        }
        Ok(FrameProcResult::None)
    }

    fn inventory(&self) -> Option<&Inventory> {
        Some(&self.inventory)
    }

    fn inventory_mut(&mut self) -> Option<&mut Inventory> {
        Some(&mut self.inventory)
    }

    fn input(&mut self, o: &DropItem) -> Result<(), JsValue> {
        if self.recipe.is_none() {
            match o.type_ {
                ItemType::IronOre => {
                    self.recipe = Some(Recipe {
                        input: [(ItemType::IronOre, 1usize)]
                            .iter()
                            .map(|(k, v)| (*k, *v))
                            .collect(),
                        output: [(ItemType::IronPlate, 1usize)]
                            .iter()
                            .map(|(k, v)| (*k, *v))
                            .collect(),
                        power_cost: 20.,
                        recipe_time: 50.,
                    });
                }
                ItemType::CopperOre => {
                    self.recipe = Some(Recipe {
                        input: [(ItemType::CopperOre, 1usize)]
                            .iter()
                            .map(|(k, v)| (*k, *v))
                            .collect(),
                        output: [(ItemType::CopperPlate, 1usize)]
                            .iter()
                            .map(|(k, v)| (*k, *v))
                            .collect(),
                        power_cost: 20.,
                        recipe_time: 50.,
                    });
                }
                _ => {
                    return Err(JsValue::from_str(&format!(
                        "Cannot smelt {}",
                        item_to_str(&o.type_)
                    )))
                }
            }
        }

        // Fuels are always welcome.
        if o.type_ == ItemType::CoalOre {
            self.inventory.add_item(&ItemType::CoalOre);
            return Ok(());
        }

        if let Some(recipe) = &self.recipe {
            if recipe
                .input
                .iter()
                .find(|item| *item.0 == o.type_)
                .is_some()
                || recipe
                    .output
                    .iter()
                    .find(|item| *item.0 == o.type_)
                    .is_some()
            {
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
}
