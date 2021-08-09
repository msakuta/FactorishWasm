use super::{
    factory::Factory,
    gl::{
        utils::{enable_buffer, Flatten},
        ShaderBundle,
    },
    inventory::InventoryTrait,
    items::get_item_image_url,
    serialize_impl,
    structure::{
        Energy, Structure, StructureBundle, StructureComponents, StructureDynIter, StructureId,
    },
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
pub(crate) struct Assembler {}

impl Assembler {
    pub(crate) fn new(position: &Position) -> StructureBundle {
        StructureBundle::new(
            Box::new(Assembler {}),
            Some(*position),
            None,
            None,
            Some(Energy {
                value: 0.,
                max: 100.,
            }),
            Some(Factory::new()),
            vec![],
        )
    }
}

impl Structure for Assembler {
    fn name(&self) -> &str {
        "Assembler"
    }

    fn draw(
        &self,
        components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        let (x, y) = if let Some(position) = &components.position {
            (position.x as f64 * TILE_SIZE, position.y as f64 * TILE_SIZE)
        } else {
            (0., 0.)
        };
        if depth == 0 {
            match state.image_assembler.as_ref() {
                Some(img) => {
                    let sx = if let Some((energy, factory)) =
                        components.energy.as_ref().zip(components.factory.as_ref())
                    {
                        if factory.progress.is_some() && 0. < energy.value {
                            ((((state.sim_time * 5.) as isize) % 4) * 32) as f64
                        } else {
                            0.
                        }
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
        components: &StructureComponents,
        state: &FactorishState,
        gl: &GL,
        depth: i32,
        is_ghost: bool,
    ) -> Result<(), JsValue> {
        let position = components.position.ok_or_else(|| js_str!("Assembler without Position"))?;
        let factory = components.factory.as_ref().ok_or_else(|| js_str!("Assembler without Factory"))?;
        let energy = components.energy.as_ref().ok_or_else(|| js_str!("Assembler without Energy"))?;
        let (x, y) = (
            position.x as f32 + state.viewport.x as f32,
            position.y as f32 + state.viewport.y as f32,
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
                let sx = if factory.progress.is_some() && 0. < energy.value {
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
                if let Some((
                    energy,
                    Factory {
                        recipe: Some(_recipe),
                        ..
                    },
                )) = components.energy.as_ref().zip(components.factory.as_ref())
                {
                    if !is_ghost && energy.value == 0. && state.sim_time % 1. < 0.5 {
                        let shader = get_shader()?;
                        gl.active_texture(GL::TEXTURE0);
                        gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_electricity_alarm));
                        gl.uniform_matrix3fv_with_f32_array(
                            shader.tex_transform_loc.as_ref(),
                            false,
                            Matrix3::from_scale(1.).flatten(),
                        );

                        shape(shader)?;
                        gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
                    }
                }
            }
            _ => (),
        }
        Ok(())
    }

    fn desc(&self, components: &StructureComponents, _state: &FactorishState) -> String {
        let (energy, factory) =
            if let Some(bundle) = components.energy.as_ref().zip(components.factory.as_ref()) {
                bundle
            } else {
                return "Energy or Factory component not found".to_string();
            };
        format!(
            "{}<br>{}{}",
            if let Some(recipe) = &factory.recipe {
                // Progress bar
                format!("{}{}{}{}",
                    format!("Progress: {:.0}%<br>", factory.progress.unwrap_or(0.) * 100.),
                    "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
                    format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                        factory.progress.unwrap_or(0.) * 100.),
                    format!(r#"Power: {:.1}kJ <div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>
                    <div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>"#,
                    energy.value,
                    if 0. < energy.max { energy.value / energy.max * 100. } else { 0. }),
                    )
                + &generate_item_image(&_state.image_time.as_ref().unwrap().url, true, recipe.recipe_time as usize) + "<br>" +
                "Outputs: <br>" +
                &recipe.output.iter()
                    .map(|item| format!("{}<br>", &generate_item_image(get_item_image_url(_state, &item.0), true, *item.1)))
                    .fold::<String, _>("".to_string(), |a, s| a + &s)
            } else {
                String::from("No recipe")
            },
            format!("Input Items: <br>{}", factory.input_inventory.describe()),
            format!("Output Items: <br>{}", factory.output_inventory.describe())
        )
    }

    fn frame_proc(
        &mut self,
        me: StructureId,
        components: &mut StructureComponents,
        state: &mut FactorishState,
        structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        if let StructureComponents {
            position: Some(position),
            energy: Some(energy),
            factory:
                Some(Factory {
                    recipe: Some(recipe),
                    ..
                }),
            ..
        } = components
        {
            let mut ret = FrameProcResult::None;
            // First, check if we need to refill the energy buffer in order to continue the current work.
            // Refill the energy from the fuel
            if energy.value < recipe.power_cost {
                let mut accumulated = 0.;
                for network in &state
                    .power_networks
                    .iter()
                    .find(|network| network.sinks.contains(&me))
                {
                    for id in network.sources.iter() {
                        if let Some(source) = structures.get_mut(*id) {
                            let demand = energy.max - energy.value - accumulated;
                            if let Some(energy) = source.dynamic.power_outlet(demand) {
                                accumulated += energy;
                                // console_log!("draining {:?}kJ of energy with {:?} demand, from {:?}, accumulated {:?}", energy, demand, structure.name(), accumulated);
                            }
                        }
                    }
                }
                energy.value += accumulated;
                ret = FrameProcResult::InventoryChanged(*position);
            }

            return Ok(ret);
        }
        Ok(FrameProcResult::None)
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
                Recipe::new(
                    hash_map!(ItemType::IronPlate => 1usize, ItemType::Gear => 1usize),
                    hash_map!(ItemType::TransportBelt => 1usize),
                    20.,
                    50.,
                ),
                Recipe::new(
                    hash_map!(ItemType::TransportBelt => 1, ItemType::Gear => 2),
                    hash_map!(ItemType::UndergroundBelt => 1usize),
                    20.,
                    50.,
                ),
                Recipe::new(
                    hash_map!(ItemType::TransportBelt => 2, ItemType::Gear => 2),
                    hash_map!(ItemType::Splitter => 1),
                    25.,
                    40.,
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
            ]
        });

        std::borrow::Cow::from(&RECIPES[..])
    }

    fn select_recipe(&mut self, factory: &mut Factory, index: usize) -> Result<bool, JsValue> {
        factory.recipe = Some(
            self.get_recipes()
                .get(index)
                .ok_or_else(|| js_str!("recipes index out of bound {:?}", index))?
                .clone(),
        );
        Ok(true)
    }

    fn power_sink(&self) -> bool {
        true
    }

    serialize_impl!();
}
