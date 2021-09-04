use super::{
    burner::{Burner, FUEL_CAPACITY},
    factory::Factory,
    furnace::RECIPES,
    gl::{
        draw_electricity_alarm_gl,
        utils::{enable_buffer, Flatten},
    },
    inventory::Inventory,
    items::item_to_str,
    serialize_impl,
    structure::Energy,
    structure::{Structure, StructureBundle, StructureComponents, StructureDynIter, StructureId},
    DropItem, FactorishState, FrameProcResult, InventoryTrait, ItemType, Position, Recipe,
};
use cgmath::{Matrix3, Matrix4, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, WebGlRenderingContext as GL};

#[derive(Serialize, Deserialize)]
pub(crate) struct ElectricFurnace {}

impl ElectricFurnace {
    pub(crate) fn new(position: &Position) -> StructureBundle {
        StructureBundle::new(
            Box::new(ElectricFurnace {}),
            Some(*position),
            None,
            Some(Burner {
                inventory: Inventory::new(),
                capacity: FUEL_CAPACITY,
            }),
            Some(Energy {
                value: 0.,
                max: 100.,
            }),
            Some(Factory::new()),
            vec![],
        )
    }
}

impl Structure for ElectricFurnace {
    fn name(&self) -> &str {
        "Electric Furnace"
    }

    fn draw(
        &self,
        components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };

        let (x, y) = if let Some(position) = &components.position {
            (position.x as f64 * 32., position.y as f64 * 32.)
        } else {
            (0., 0.)
        };
        match state.image_electric_furnace.as_ref() {
            Some(img) => {
                let sx = if let Some((energy, factory)) =
                    components.energy.as_ref().zip(components.factory.as_ref())
                {
                    if factory.progress.is_some() && 0. < energy.value {
                        ((((state.sim_time * 5.) as isize) % 2 + 1) * 32) as f64
                    } else {
                        0.
                    }
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
            None => return Err(JsValue::from_str("electric furnace image not available")),
        }

        Ok(())
    }

    fn draw_gl(
        &self,
        components: &StructureComponents,
        state: &FactorishState,
        gl: &web_sys::WebGlRenderingContext,
        depth: i32,
        is_ghost: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        let position = components
            .position
            .ok_or_else(|| js_str!("Furnace without Position"))?;
        let factory = components
            .factory
            .as_ref()
            .ok_or_else(|| js_str!("Furnace without Factory"))?;
        let energy = components
            .energy
            .as_ref()
            .ok_or_else(|| js_str!("Furnace without Energy"))?;
        let shader = state
            .assets
            .textured_shader
            .as_ref()
            .ok_or_else(|| js_str!("Shader not found"))?;
        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), if is_ghost { 0.5 } else { 1. });
        let (x, y) = (
            position.x as f32 + state.viewport.x as f32,
            position.y as f32 + state.viewport.y as f32,
        );
        if depth == 2 {
            if !is_ghost && factory.recipe.is_some() && energy.value == 0. {
                draw_electricity_alarm_gl((x, y), state, gl)?;
            }
        }
        if depth != 0 {
            return Ok(());
        };
        let shader = state
            .assets
            .textured_shader
            .as_ref()
            .ok_or_else(|| js_str!("Shader not found"))?;
        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), if is_ghost { 0.5 } else { 1. });
        let texture = &state.assets.tex_electric_furnace;
        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(texture));
        let sx = if factory.progress.is_some() && 0. < energy.value {
            (((state.sim_time * 5.) as isize) % 2 + 1) as f32 / 3.
        } else {
            0.
        };
        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            (Matrix3::from_translation(Vector2::new(sx, 0.))
                * Matrix3::from_nonuniform_scale(1. / 3., 1.))
            .flatten(),
        );

        enable_buffer(&gl, &state.assets.screen_buffer, 2, shader.vertex_position);
        gl.uniform_matrix4fv_with_f32_array(
            shader.transform_loc.as_ref(),
            false,
            &(state.get_world_transform()?
                * Matrix4::from_scale(2.)
                * Matrix4::from_translation(Vector3::new(x, y, 0.)))
            .flatten(),
        );
        gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

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
            if factory.recipe.is_some() {
                // Progress bar
                format!("{}{}{}{}",
                    format!("Progress: {:.0}%<br>", factory.progress.unwrap_or(0.) * 100.),
                    "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
                    format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                        factory.progress.unwrap_or(0.) * 100.),
                    format!(r#"Power: {:.1}kJ <div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>
                    <div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>"#,
                    energy.value,
                    if 0. < energy.max { (energy.value) / energy.max * 100. } else { 0. }),
                    )
            // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
            // "Outputs: <br>" +
            // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
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
        let position = components.position.ok_or_else(|| ())?;
        let factory = components.factory.as_mut().ok_or_else(|| ())?;
        let energy = components.energy.as_mut().ok_or_else(|| ())?;
        if factory.recipe.is_none() {
            factory.recipe = RECIPES
                .iter()
                .find(|recipe| {
                    recipe
                        .input
                        .iter()
                        .all(|(type_, count)| *count <= factory.input_inventory.count_item(&type_))
                })
                .cloned();
        }
        if let Some(recipe) = &factory.recipe {
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
                            if let Some(energy) =
                                source.dynamic.power_outlet(&mut source.components, demand)
                            {
                                accumulated += energy;
                                // console_log!("draining {:?}kJ of energy with {:?} demand, from {:?}, accumulated {:?}", energy, demand, structure.name(), accumulated);
                            }
                        }
                    }
                }
                energy.value += accumulated;
            }

            let mut ret = FrameProcResult::None;

            if factory.progress.is_none() {
                // First, check if we have enough ingredients to finish this recipe.
                // If we do, consume the ingredients and start the progress timer.
                // We can't start as soon as the recipe is set because we may not have enough ingredients
                // at the point we set the recipe.
                if recipe
                    .input
                    .iter()
                    .map(|(item, count)| count <= &factory.input_inventory.count_item(item))
                    .all(|b| b)
                {
                    for (item, count) in &recipe.input {
                        factory.input_inventory.remove_items(item, *count);
                    }
                    factory.progress = Some(0.);
                    ret = FrameProcResult::InventoryChanged(position);
                } else {
                    factory.recipe = None;
                    return Ok(FrameProcResult::None); // Return here to avoid borrow checker
                }
            }

            if let Some(prev_progress) = factory.progress {
                // Proceed only if we have sufficient energy in the buffer.
                let progress = (energy.value / recipe.power_cost)
                    .min(1. / recipe.recipe_time)
                    .min(1.);
                if 1. <= prev_progress + progress {
                    factory.progress = None;

                    // Produce outputs into inventory
                    for output_item in &recipe.output {
                        factory.output_inventory.add_item(&output_item.0);
                    }
                    return Ok(FrameProcResult::InventoryChanged(position));
                } else {
                    factory.progress = Some(prev_progress + progress);
                    energy.value -= progress * recipe.power_cost;
                }
            }
            return Ok(ret);
        }
        Ok(FrameProcResult::None)
    }

    fn input(&mut self, components: &mut StructureComponents, o: &DropItem) -> Result<(), JsValue> {
        let factory = components
            .factory
            .as_mut()
            .ok_or_else(|| js_str!("ElectricFurnace without Factory component"))?;
        let burner = components
            .burner
            .as_mut()
            .ok_or_else(|| js_str!("ElectricFurnace without Burner component"))?;
        if o.type_ == ItemType::CoalOre
            && burner.inventory.count_item(&ItemType::CoalOre) < FUEL_CAPACITY
        {
            burner.inventory.add_item(&ItemType::CoalOre);
            return Ok(());
        }

        if factory.recipe.is_none() {
            if let Some(recipe) = RECIPES
                .iter()
                .find(|recipe| recipe.input.contains_key(&o.type_))
            {
                factory.recipe = Some(recipe.clone());
            } else {
                return Err(JsValue::from_str(&format!(
                    "Cannot smelt {}",
                    item_to_str(&o.type_)
                )));
            }
        }

        // if 0 < default_add_inventory(self, InventoryType::Input, &o.type_, 1) {
        Ok(())
        // } else {
        //     Err(JsValue::from_str("Item is not part of recipe"))
        // }
    }

    fn can_input(&self, components: &StructureComponents, item_type: &ItemType) -> bool {
        let factory = if let Some(factory) = components.factory.as_ref() {
            factory
        } else {
            return false;
        };
        if let Some(recipe) = &factory.recipe {
            recipe.input.get(item_type).is_some()
        } else {
            RECIPES
                .iter()
                .any(|recipe| recipe.input.contains_key(item_type))
        }
    }

    fn get_recipes(&self) -> std::borrow::Cow<[Recipe]> {
        std::borrow::Cow::from(&RECIPES[..])
    }

    fn auto_recipe(&self) -> bool {
        true
    }

    fn power_sink(&self) -> bool {
        true
    }

    serialize_impl!();
}
