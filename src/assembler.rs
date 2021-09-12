use super::{
    drop_items::DropItem,
    gl::{
        draw_electricity_alarm_gl,
        utils::{enable_buffer, Flatten},
        ShaderBundle,
    },
    inventory::{Inventory, InventoryTrait, InventoryType},
    items::get_item_image_url,
    research::TechnologyTag,
    serialize_impl,
    structure::{default_add_inventory, Structure, StructureDynIter, StructureId},
    FactorishState, FrameProcResult, ItemType, Position, Recipe, TILE_SIZE,
};
use cgmath::{Matrix3, Matrix4, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, WebGlRenderingContext as GL};

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
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        if depth == 0 {
            let (x, y) = (
                self.position.x as f64 * TILE_SIZE,
                self.position.y as f64 * TILE_SIZE,
            );
            match state.image_assembler.as_ref() {
                Some(img) => {
                    let sx = if self.progress.is_some() && 0. < self.power {
                        ((((state.sim_time * 5.) as isize) % 4) * 32) as f64
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
                None => return Err(JsValue::from_str("assembler image not available")),
            }
            return Ok(());
        }

        Ok(())
    }

    fn draw_gl(
        &self,
        state: &FactorishState,
        gl: &GL,
        depth: i32,
        is_ghost: bool,
    ) -> Result<(), JsValue> {
        let (x, y) = (
            self.position.x as f32 + state.viewport.x as f32,
            self.position.y as f32 + state.viewport.y as f32,
        );

        let get_shader = || -> Result<&ShaderBundle, JsValue> {
            let shader = state
                .assets
                .textured_shader
                .as_ref()
                .ok_or_else(|| js_str!("Shader not found"))?;
            gl.use_program(Some(&shader.program));
            gl.uniform1f(shader.alpha_loc.as_ref(), if is_ghost { 0.5 } else { 1. });
            Ok(shader)
        };

        let shape = |shader: &ShaderBundle| -> Result<(), JsValue> {
            enable_buffer(&gl, &state.assets.screen_buffer, 2, shader.vertex_position);
            gl.uniform_matrix4fv_with_f32_array(
                shader.transform_loc.as_ref(),
                false,
                (state.get_world_transform()?
                    * Matrix4::from_scale(2.)
                    * Matrix4::from_translation(Vector3::new(x, y, 0.)))
                .flatten(),
            );
            Ok(())
        };

        match depth {
            0 => {
                let shader = get_shader()?;
                gl.active_texture(GL::TEXTURE0);
                gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_assembler));
                let sx = if self.progress.is_some() && 0. < self.power {
                    (((state.sim_time * 5.) as isize) % 4 + 1) as f32
                } else {
                    0.
                };
                gl.uniform_matrix3fv_with_f32_array(
                    shader.tex_transform_loc.as_ref(),
                    false,
                    (Matrix3::from_nonuniform_scale(1. / 4., 1.)
                        * Matrix3::from_translation(Vector2::new(sx, 0.)))
                    .flatten(),
                );

                shape(shader)?;
                gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
            }
            2 => {
                if !is_ghost && self.recipe.is_some() && self.power == 0. {
                    draw_electricity_alarm_gl((x, y), state, gl)?;
                }
            }
            _ => (),
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
        me: StructureId,
        state: &mut FactorishState,
        structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        if let Some(recipe) = &self.recipe {
            let mut ret = FrameProcResult::None;
            // First, check if we need to refill the energy buffer in order to continue the current work.
            // Refill the energy from the fuel
            if self.power < recipe.power_cost {
                let mut accumulated = 0.;
                for network in &state
                    .power_networks
                    .iter()
                    .find(|network| network.sinks.contains(&me))
                {
                    for id in network.sources.iter() {
                        if let Some(source) = structures.get_mut(*id) {
                            let demand = self.max_power - self.power - accumulated;
                            if let Some(energy) = source.power_outlet(demand) {
                                accumulated += energy;
                                // console_log!("draining {:?}kJ of energy with {:?} demand, from {:?}, accumulated {:?}", energy, demand, structure.name(), accumulated);
                            }
                        }
                    }
                }
                self.power += accumulated;
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
                    console_log!("inputting from Assembler {}", recipe.output.len());
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
                    console_log!("outputting from Assembler {}", recipe.output.len());
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
        if self.recipe.is_some() {
            if 0 < default_add_inventory(self, InventoryType::Input, &o.type_, 1) {
                return Ok(());
            } else {
                return Err(JsValue::from_str("Item is not part of recipe"));
            }
        }
        Err(JsValue::from_str("Recipe is not initialized"))
    }

    fn can_output(&self, _structures: &StructureDynIter) -> Inventory {
        self.output_inventory.clone()
    }

    fn output(&mut self, _state: &mut FactorishState, item_type: &ItemType) -> Result<(), ()> {
        if self.output_inventory.remove_item(item_type) {
            Ok(())
        } else {
            Err(())
        }
    }

    fn inventory(&self, invtype: InventoryType) -> Option<&Inventory> {
        Some(match invtype {
            InventoryType::Input => &self.input_inventory,
            InventoryType::Output => &self.output_inventory,
            _ => return None,
        })
    }

    fn inventory_mut(&mut self, invtype: InventoryType) -> Option<&mut Inventory> {
        Some(match invtype {
            InventoryType::Input => &mut self.input_inventory,
            InventoryType::Output => &mut self.output_inventory,
            _ => return None,
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

    fn get_recipes(&self) -> std::borrow::Cow<[Recipe]> {
        static RECIPES: once_cell::sync::Lazy<Vec<Recipe>> = once_cell::sync::Lazy::new(|| {
            vec![
                Recipe::new(
                    hash_map!(ItemType::IronPlate => 2usize),
                    hash_map!(ItemType::Gear => 1usize),
                    20.,
                    50.,
                ),
                Recipe::new_with_requires(
                    hash_map!(ItemType::IronPlate => 1usize, ItemType::Gear => 1usize),
                    hash_map!(ItemType::TransportBelt => 1usize),
                    20.,
                    50.,
                    hash_set!(TechnologyTag::Transportation),
                ),
                Recipe::new_with_requires(
                    hash_map!(ItemType::TransportBelt => 1, ItemType::Gear => 2),
                    hash_map!(ItemType::UndergroundBelt => 1usize),
                    20.,
                    50.,
                    hash_set!(TechnologyTag::Transportation),
                ),
                Recipe::new_with_requires(
                    hash_map!(ItemType::TransportBelt => 2, ItemType::Gear => 2),
                    hash_map!(ItemType::Splitter => 1),
                    25.,
                    40.,
                    hash_set!(TechnologyTag::Transportation),
                ),
                Recipe::new(
                    hash_map!(ItemType::IronPlate => 5usize),
                    hash_map!(ItemType::Chest => 1usize),
                    20.,
                    50.,
                ),
                Recipe::new(
                    hash_map!(ItemType::StoneOre => 5usize),
                    hash_map!(ItemType::Furnace => 1usize),
                    20.,
                    20.,
                ),
                Recipe::new_with_requires(
                    hash_map!(ItemType::SteelPlate => 5usize, ItemType::Furnace => 1),
                    hash_map!(ItemType::ElectricFurnace => 1usize),
                    20.,
                    20.,
                    hash_set!(TechnologyTag::SteelWorks),
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
                    hash_map!(ItemType::IronPlate => 5, ItemType::Gear => 3, ItemType::CopperWire => 10),
                    hash_map!(ItemType::Lab => 1),
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
                    hash_map!(ItemType::Pipe => 10),
                    hash_map!(ItemType::UndergroundPipe => 2),
                    20.,
                    20.,
                ),
                Recipe::new(
                    hash_map!(ItemType::IronPlate => 5, ItemType::Gear => 5),
                    hash_map!(ItemType::OffshorePump => 1),
                    150.,
                    150.,
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
                Recipe::new(
                    hash_map!(ItemType::IronPlate => 2, ItemType::CopperWire => 2),
                    hash_map!(ItemType::ElectPole => 1),
                    20.,
                    20.,
                ),
                Recipe::new(
                    hash_map!(ItemType::IronPlate => 1, ItemType::Gear => 1),
                    hash_map!(ItemType::SciencePack1 => 1),
                    50.,
                    50.,
                ),
            ]
        });

        std::borrow::Cow::from(&RECIPES[..])
    }

    fn select_recipe(&mut self, index: usize) -> Result<bool, JsValue> {
        self.recipe = Some(
            self.get_recipes()
                .get(index)
                .ok_or_else(|| js_str!("recipes index out of bound {:?}", index))?
                .clone(),
        );
        Ok(true)
    }

    fn get_selected_recipe(&self) -> Option<&Recipe> {
        self.recipe.as_ref()
    }

    fn get_progress(&self) -> Option<f64> {
        self.progress
    }

    fn power_sink(&self) -> bool {
        true
    }

    serialize_impl!();
}
