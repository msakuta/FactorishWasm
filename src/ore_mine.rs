use super::{
    burner::Burner,
    draw_direction_arrow,
    items::ItemType,
    structure::{DynIterMut, Energy, Structure, StructureBundle, StructureComponents},
    DropItem, FactorishState, FrameProcResult, Inventory, Position, Recipe, Rotation, TempEnt,
    TILE_SIZE,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;
use specs::{World, WorldExt, Entity, Builder};

const FUEL_CAPACITY: usize = 10;

#[derive(Serialize, Deserialize)]
pub(crate) struct OreMine {
    progress: f64,
    recipe: Option<Recipe>,
}

impl OreMine {
    pub(crate) fn new(world: &World, x: i32, y: i32, rotation: Rotation) -> Entity {
        world.create_entity()
            .with(Box::new(OreMine {
                progress: 0.,
                recipe: None,
            }) as Box<dyn Structure + Send + Sync>)
            .with(Position { x, y })
            .with(rotation)
            .with(Burner {
                inventory: Inventory::new(),
                capacity: FUEL_CAPACITY,
            })
            .with(Energy {
                value: 25.,
                max: 100.,
            })
            .build()
    }
}

impl Structure for OreMine {
    fn name(&self) -> &str {
        "Ore Mine"
    }

    fn draw(
        &self,
        components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        let (x, y) = if let Some(position) = components.position.as_ref() {
            (position.x as f64 * TILE_SIZE, position.y as f64 * TILE_SIZE)
        } else {
            (0., 0.)
        };
        match depth {
            0 => match state.image_mine.as_ref() {
                Some(img) => {
                    let progress = if let Some(ref recipe) = self.recipe {
                        (0f64/*self.power / recipe.power_cost*/)
                            .min(1. / recipe.recipe_time)
                            .min(1. - self.progress)
                    } else {
                        0.
                    };
                    let sx = if 0. < progress {
                        (((state.sim_time * 5.) as isize) % 2 + 1) as f64 * TILE_SIZE
                    } else {
                        0.
                    };
                    context
                        .draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                            &img.bitmap,
                            sx,
                            0.,
                            TILE_SIZE,
                            TILE_SIZE,
                            x,
                            y,
                            TILE_SIZE,
                            TILE_SIZE,
                        )?;
                }
                None => return Err(JsValue::from_str("mine image not available")),
            },
            2 => {
                draw_direction_arrow(
                    (x, y),
                    &components.rotation.unwrap_or(Rotation::Left),
                    state,
                    context,
                )?;
            }
            _ => (),
        }

        Ok(())
    }

    fn desc(&self, components: &StructureComponents, state: &FactorishState) -> String {
        let (position, energy) =
            if let Some(energy) = components.position.as_ref().zip(components.energy.as_ref()) {
                energy
            } else {
                return "Position or Energy not found".to_string();
            };
        let tile = &state.board[position.x as usize + position.y as usize * state.width as usize];
        if let Some(_recipe) = &self.recipe {
            // Progress bar
            format!("{}{}{}{}{}",
                format!("Progress: {:.0}%<br>", self.progress * 100.),
                "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
                format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                    self.progress * 100.),
                format!(r#"Power: {:.1}kJ <div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>
                 <div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>"#,
                    energy.value,
                    if 0. < energy.max { (energy.value) / energy.max * 100. } else { 0. }),
                format!("Expected output: {}", if 0 < tile.iron_ore { tile.iron_ore } else if 0 < tile.coal_ore { tile.coal_ore } else { tile.copper_ore }))
        // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
        // "Outputs: <br>" +
        // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
        } else {
            String::from("Empty")
        }
    }

    fn frame_proc(
        &mut self,
        components: &mut StructureComponents,
        state: &mut FactorishState,
        structures: &mut dyn DynIterMut<Item = StructureBundle>,
    ) -> Result<FrameProcResult, ()> {
        let position = components.position.as_ref().ok_or(())?;
        let rotation = components.rotation.as_ref().ok_or(())?;
        let energy = components.energy.as_mut().ok_or(())?;

        let otile = &state.tile_at(position);
        if otile.is_none() {
            return Ok(FrameProcResult::None);
        }
        let tile = otile.unwrap();

        let ret = FrameProcResult::None;

        if self.recipe.is_none() {
            if 0 < tile.iron_ore {
                self.recipe = Some(Recipe::new(
                    HashMap::new(),
                    hash_map!(ItemType::IronOre => 1usize),
                    8.,
                    80.,
                ));
            } else if 0 < tile.coal_ore {
                self.recipe = Some(Recipe::new(
                    HashMap::new(),
                    hash_map!(ItemType::CoalOre => 1usize),
                    8.,
                    80.,
                ));
            } else if 0 < tile.copper_ore {
                self.recipe = Some(Recipe::new(
                    HashMap::new(),
                    hash_map!(ItemType::CopperOre => 1usize),
                    8.,
                    80.,
                ));
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

            let output = |state: &mut FactorishState, item, position: &Position| {
                if let Ok(val) = if let Some(tile) = state.tile_at_mut(&position) {
                    match item {
                        ItemType::IronOre => Ok(&mut tile.iron_ore),
                        ItemType::CoalOre => Ok(&mut tile.coal_ore),
                        ItemType::CopperOre => Ok(&mut tile.copper_ore),
                        _ => Err(()),
                    }
                } else {
                    Err(())
                } {
                    if 0 < *val {
                        *val -= 1;
                        Ok(*val)
                    } else {
                        Err(())
                    }
                } else {
                    Err(())
                }
            };

            // Proceed only if we have sufficient energy in the buffer.
            let progress = (energy.value / recipe.power_cost)
                .min(1. / recipe.recipe_time)
                .min(1. - self.progress);
            if state.rng.next() < progress * 5. {
                state
                    .temp_ents
                    .push(TempEnt::new(&mut state.rng, *position));
            }
            if 1. <= self.progress + progress {
                self.progress = 0.;
                let output_position = position.add(rotation.delta());
                let mut str_iter = structures.dyn_iter_mut();
                if let Some(structure) =
                    str_iter.find(|s| s.components.position == Some(output_position))
                {
                    let mut it = recipe.output.iter();
                    if let Some(item) = it.next() {
                        // Check whether we can input first
                        if structure.can_input(item.0) {
                            if let Ok(val) = output(state, *item.0, position) {
                                structure
                                    .input(&DropItem {
                                        id: 0,
                                        type_: *item.0,
                                        x: output_position.x,
                                        y: output_position.y,
                                    })
                                    .map_err(|_| ())?;
                                if val == 0 {
                                    self.recipe = None;
                                }
                                return Ok(FrameProcResult::InventoryChanged(output_position));
                            } else {
                                self.recipe = None;
                                return Err(());
                            };
                        }
                    }
                    if !structure.dynamic.movable() {
                        return Ok(FrameProcResult::None);
                    }
                }
                if !state.hit_check(output_position.x, output_position.y, None) {
                    // let dest_tile = state.board[dx as usize + dy as usize * state.width as usize];
                    let mut it = recipe.output.iter();
                    if let Some(item) = it.next() {
                        assert!(it.next().is_none());
                        if let Err(_code) =
                            state.new_object(output_position.x, output_position.y, *item.0)
                        {
                            // console_log!("Failed to create object: {:?}", code);
                        } else if let Ok(val) = output(state, *item.0, position) {
                            if val == 0 {
                                self.recipe = None;
                            }
                        }
                    } else {
                        return Err(());
                    }
                }
            } else {
                self.progress += progress;
                energy.value -= progress * recipe.power_cost;
            }
        }
        Ok(ret)
    }

    crate::serialize_impl!();
}
