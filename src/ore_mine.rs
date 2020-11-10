use super::structure::Structure;
use super::{
    draw_direction_arrow, DropItem, FactorishState, FrameProcResult, ItemResponse,
    ItemResponseResult, ItemType, Position, Recipe, Rotation,
};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

pub(crate) struct OreMine {
    position: Position,
    rotation: Rotation,
    cooldown: f64,
    power: f64,
    max_power: f64,
    recipe: Option<Recipe>,
}

impl OreMine {
    pub(crate) fn new(x: i32, y: i32, rotation: Rotation) -> Self {
        OreMine {
            position: Position { x, y },
            rotation,
            cooldown: 0.,
            power: 20.,
            max_power: 20.,
            recipe: None,
        }
    }
}

impl Structure for OreMine {
    fn name(&self) -> &str {
        "Ore Mine"
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
        let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
        match depth {
            0 => match state.image_mine.as_ref() {
                Some(img) => {
                    context.draw_image_with_image_bitmap(img, x, y)?;
                }
                None => return Err(JsValue::from_str("mine image not available")),
            },
            2 => draw_direction_arrow((x, y), &self.rotation, state, context)?,
            _ => (),
        }

        Ok(())
    }

    fn desc(&self, state: &FactorishState) -> String {
        let tile = &state.board
            [self.position.x as usize + self.position.y as usize * state.width as usize];
        if let Some(_recipe) = &self.recipe {
            let recipe_time = 80.;
            // Progress bar
            format!("{}{}{}{}{}",
                format!("Progress: {:.0}%<br>", (recipe_time - self.cooldown) / recipe_time * 100.),
                "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
                format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                    (recipe_time - self.cooldown) / recipe_time * 100.),
                format!(r#"Power: <div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>
                 <div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>"#,
                  if 0. < self.max_power { (self.power) / self.max_power * 100. } else { 0. }),
                format!("Expected output: {}", if 0 < tile.iron_ore { tile.iron_ore } else { tile.coal_ore }))
        // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
        // "Outputs: <br>" +
        // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
        } else {
            String::from("Empty")
        }
    }

    fn frame_proc(
        &mut self,
        state: &mut FactorishState,
        _structures: &mut dyn Iterator<Item = &mut Box<dyn Structure>>,
    ) -> Result<FrameProcResult, ()> {
        let otile = &state.tile_at(&[self.position.x, self.position.y]);
        if otile.is_none() {
            return Ok(FrameProcResult::None);
        }
        let tile = otile.unwrap();

        if self.recipe.is_none() {
            if 0 < tile.iron_ore {
                self.recipe = Some(Recipe {
                    input: HashMap::new(),
                    output: [(ItemType::IronOre, 1usize)]
                        .iter()
                        .map(|(k, v)| (*k, *v))
                        .collect(),
                    power_cost: 0.1,
                    recipe_time: 80.,
                });
            } else if 0 < tile.coal_ore {
                self.recipe = Some(Recipe {
                    input: HashMap::new(),
                    output: [(ItemType::CoalOre, 1usize)]
                        .iter()
                        .map(|(k, v)| (*k, *v))
                        .collect(),
                    power_cost: 0.1,
                    recipe_time: 80.,
                });
            } else if 0 < tile.copper_ore {
                self.recipe = Some(Recipe {
                    input: HashMap::new(),
                    output: [(ItemType::CopperOre, 1usize)]
                        .iter()
                        .map(|(k, v)| (*k, *v))
                        .collect(),
                    power_cost: 0.1,
                    recipe_time: 80.,
                });
            }
        }
        if let Some(recipe) = &self.recipe {
            // First, check if we need to refill the energy buffer in order to continue the current work.
            // if("Coal Ore" in this.inventory){
            //     var coalPower = 100;
            //     // Refill the energy from the fuel
            //     if(this.power < this.recipe.powerCost){
            //         this.power += coalPower;
            //         this.maxPower = this.power;
            //         this.removeItem("Coal Ore");
            //     }
            // }

            // Proceed only if we have sufficient energy in the buffer.
            let progress = (self.power / recipe.power_cost).min(1.);
            if self.cooldown < progress {
                self.cooldown = 0.;
                let output_position = self.position.add(self.rotation.delta());
                if !state.hit_check(output_position.x, output_position.y, None) {
                    // let dest_tile = state.board[dx as usize + dy as usize * state.width as usize];
                    let mut it = recipe.output.iter();
                    if let Some(item) = it.next() {
                        if let Err(_code) =
                            state.new_object(output_position.x, output_position.y, *item.0)
                        {
                            // console_log!("Failed to create object: {:?}", code);
                        } else {
                            if let Some(tile) =
                                state.tile_at_mut(&[self.position.x, self.position.y])
                            {
                                self.cooldown = recipe.recipe_time;
                                match *item.0 {
                                    ItemType::IronOre => tile.iron_ore -= 1,
                                    ItemType::CoalOre => tile.coal_ore -= 1,
                                    ItemType::CopperOre => tile.copper_ore -= 1,
                                    _ => (),
                                }
                            }
                        }
                        assert!(it.next().is_none());
                    } else {
                        return Err(());
                    }
                }
            } else {
                self.cooldown -= progress;
                self.power -= progress * recipe.power_cost;
            }
        }
        Ok(FrameProcResult::None)
    }

    fn rotate(&mut self) -> Result<(), ()> {
        self.rotation.next();
        Ok(())
    }

    fn set_rotation(&mut self, rotation: &Rotation) -> Result<(), ()> {
        self.rotation = *rotation;
        Ok(())
    }

    fn item_response(&mut self, item: &DropItem) -> Result<ItemResponseResult, ()> {
        if item.type_ == ItemType::CoalOre && self.power == 0. {
            self.max_power = 100.;
            self.power = 100.;
            Ok((ItemResponse::Consume, None))
        } else {
            Err(())
        }
    }
}