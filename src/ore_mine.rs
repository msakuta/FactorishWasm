use super::items::ItemType;
use super::structure::{Structure, DynIterMut};
use super::{
    draw_direction_arrow, DropItem, FactorishState, FrameProcResult, Position, Recipe, Rotation,
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
                    context.draw_image_with_image_bitmap(&img.bitmap, x, y)?;
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
        structures: &mut dyn DynIterMut<Item = Box<dyn Structure>>,
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
                    output: hash_map!(ItemType::IronOre => 1usize),
                    power_cost: 0.1,
                    recipe_time: 80.,
                });
            } else if 0 < tile.coal_ore {
                self.recipe = Some(Recipe {
                    input: HashMap::new(),
                    output: hash_map!(ItemType::CoalOre => 1usize),
                    power_cost: 0.1,
                    recipe_time: 80.,
                });
            } else if 0 < tile.copper_ore {
                self.recipe = Some(Recipe {
                    input: HashMap::new(),
                    output: hash_map!(ItemType::CopperOre => 1usize),
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

            let output =
                |state: &mut FactorishState, item, position: &Position, cooldown: &mut f64| {
                    if let Some(tile) = state.tile_at_mut(&[position.x, position.y]) {
                        *cooldown = recipe.recipe_time;
                        match item {
                            ItemType::IronOre => tile.iron_ore -= 1,
                            ItemType::CoalOre => tile.coal_ore -= 1,
                            ItemType::CopperOre => tile.copper_ore -= 1,
                            _ => (),
                        }
                    }
                };

            // Proceed only if we have sufficient energy in the buffer.
            let progress = (self.power / recipe.power_cost).min(1.);
            if self.cooldown < progress {
                self.cooldown = 0.;
                let output_position = self.position.add(self.rotation.delta());
                let mut str_iter = structures.dyn_iter_mut().map(|v| v);
                if let Some(structure) = str_iter.find(|s| *s.position() == output_position) {
                    let mut it = recipe.output.iter();
                    if let Some(item) = it.next() {
                        if structure
                            .input(&DropItem {
                                id: 1,
                                type_: *item.0,
                                x: output_position.x,
                                y: output_position.y,
                            })
                            .is_ok()
                        {
                            output(state, *item.0, &self.position, &mut self.cooldown);
                            return Ok(FrameProcResult::InventoryChanged(output_position));
                        }
                    }
                    if !structure.movable() {
                        return Ok(FrameProcResult::None);
                    }
                }
                if !state.hit_check(output_position.x, output_position.y, None) {
                    // let dest_tile = state.board[dx as usize + dy as usize * state.width as usize];
                    let mut it = recipe.output.iter();
                    if let Some(item) = it.next() {
                        if let Err(_code) =
                            state.new_object(output_position.x, output_position.y, *item.0)
                        {
                            // console_log!("Failed to create object: {:?}", code);
                        } else {
                            output(state, *item.0, &self.position, &mut self.cooldown);
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

    fn input(&mut self, item: &DropItem) -> Result<(), JsValue> {
        if item.type_ == ItemType::CoalOre && self.power == 0. {
            self.max_power = 100.;
            self.power = 100.;
            Ok(())
        } else {
            Err(JsValue::from_str("not inputtable to ore mine"))
        }
    }

    fn can_input(&self, item_type: &ItemType) -> bool {
        *item_type == ItemType::CoalOre && self.power == 0.
    }
}
