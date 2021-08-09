use super::{
    gl::utils::{enable_buffer, Flatten},
    pipe::Pipe,
    serialize_impl,
    structure::{Structure, StructureDynIter, StructureId},
    water_well::{FluidBox, FluidType},
    FactorishState, FrameProcResult, Position, Recipe,
};
use cgmath::{Matrix3, Matrix4, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, WebGlRenderingContext as GL};

use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub(crate) struct SteamEngine {
    position: Position,
    progress: Option<f64>,
    power: f64,
    max_power: f64,
    recipe: Option<Recipe>,
    input_fluid_box: FluidBox,
}

impl SteamEngine {
    pub(crate) fn new(position: &Position) -> Self {
        SteamEngine {
            position: *position,
            progress: None,
            power: 0.,
            max_power: 100.,
            recipe: Some(Recipe {
                input: HashMap::new(),
                input_fluid: Some(FluidType::Steam),
                output: HashMap::new(),
                output_fluid: None,
                power_cost: -100.,
                recipe_time: 100.,
            }),
            input_fluid_box: FluidBox::new(true, false),
        }
    }

    const FLUID_PER_PROGRESS: f64 = 100.;
    const COMBUSTION_EPSILON: f64 = 1e-6;

    fn combustion_rate(&self) -> f64 {
        if let Some(ref recipe) = self.recipe {
            ((self.max_power - self.power) / recipe.power_cost.abs())
                .min(1. / recipe.recipe_time)
                .min(self.input_fluid_box.amount / Self::FLUID_PER_PROGRESS)
                .min(1.)
                .max(0.)
        } else {
            0.
        }
    }
}

impl Structure for SteamEngine {
    fn name(&self) -> &str {
        "Steam Engine"
    }

    fn position(&self) -> &Position {
        &self.position
    }

    fn draw(
        &self,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_tooltip: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        Pipe::draw_int(self, state, context, depth, false)?;
        let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
        match state.image_steam_engine.as_ref() {
            Some(img) => {
                let sx = if self.progress.is_some()
                    && Self::COMBUSTION_EPSILON < self.combustion_rate()
                {
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

        Ok(())
    }

    fn draw_gl(
        &self,
        state: &FactorishState,
        gl: &GL,
        depth: i32,
        is_ghost: bool,
    ) -> Result<(), JsValue> {
        Pipe::draw_gl_int(self, state, gl, depth, false, is_ghost)?;
        let (x, y) = (
            self.position.x as f32 + state.viewport.x as f32,
            self.position.y as f32 + state.viewport.y as f32,
        );
        if depth != 0 {
            return Ok(());
        }
        let shader = state
            .assets
            .textured_shader
            .as_ref()
            .ok_or_else(|| js_str!("Shader not found"))?;
        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), if is_ghost { 0.5 } else { 1. });
        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_steam_engine));
        let sx = if self.progress.is_some() && Self::COMBUSTION_EPSILON < self.combustion_rate() {
            (((state.sim_time * 5.) as isize) % 2 + 1) as f32
        } else {
            0.
        };
        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            (Matrix3::from_nonuniform_scale(1. / 3., 1.)
                * Matrix3::from_translation(Vector2::new(sx, 0.)))
            .flatten(),
        );

        enable_buffer(&gl, &state.assets.screen_buffer, 2, shader.vertex_position);
        gl.uniform_matrix4fv_with_f32_array(
            shader.transform_loc.as_ref(),
            false,
            (state.get_world_transform()?
                * Matrix4::from_scale(2.)
                * Matrix4::from_translation(Vector3::new(x, y, 0.)))
            .flatten(),
        );
        gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
        Ok(())
    }

    fn desc(&self, _state: &FactorishState) -> String {
        if self.recipe.is_some() {
            // Progress bar
            format!("{}{}{}{}{}Input fluid: {}",
                format!("Progress: {:.0}%<br>", self.progress.unwrap_or(0.) * 100.),
                "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
                format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                    self.progress.unwrap_or(0.) * 100.),
                format!(r#"Power: {:.1}kJ <div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>
                <div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>"#,
                self.power,
                if 0. < self.max_power { (self.power) / self.max_power * 100. } else { 0. }),
                format!("<div>Combustion rate: {:.1}</div>", self.combustion_rate()),
                self.input_fluid_box.desc())
        // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
        // "Outputs: <br>" +
        // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
        } else {
            String::from("No recipe")
        }
    }

    fn frame_proc(
        &mut self,
        _me: StructureId,
        _state: &mut FactorishState,
        structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        self.input_fluid_box.simulate(structures);
        if let Some(recipe) = &self.recipe {
            if self.input_fluid_box.type_ == recipe.input_fluid {
                self.progress = Some(0.);
            }
            let ret = FrameProcResult::None;

            if let Some(prev_progress) = self.progress {
                // Proceed only if we have sufficient energy in the buffer.
                let progress = self.combustion_rate();
                if 1. <= prev_progress + progress {
                    self.progress = None;
                    return Ok(FrameProcResult::InventoryChanged(self.position));
                } else if Self::COMBUSTION_EPSILON < progress {
                    self.progress = Some(prev_progress + progress);
                    self.power -= progress * recipe.power_cost;
                    self.input_fluid_box.amount -= progress * Self::FLUID_PER_PROGRESS;
                }
            }
            return Ok(ret);
        }
        Ok(FrameProcResult::None)
    }

    fn get_selected_recipe(&self) -> Option<&Recipe> {
        self.recipe.as_ref()
    }

    fn fluid_box(&self) -> Option<Vec<&FluidBox>> {
        Some(vec![&self.input_fluid_box])
    }

    fn fluid_box_mut(&mut self) -> Option<Vec<&mut FluidBox>> {
        Some(vec![&mut self.input_fluid_box])
    }

    fn power_source(&self) -> bool {
        true
    }

    fn power_outlet(&mut self, demand: f64) -> Option<f64> {
        let energy = demand.min(self.power);
        self.power -= energy;
        Some(energy)
    }

    serialize_impl!();
}
