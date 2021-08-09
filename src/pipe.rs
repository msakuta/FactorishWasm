use super::{
    gl::utils::{enable_buffer, Flatten},
    structure::{Structure, StructureDynIter, StructureId},
    water_well::FluidBox,
    FactorishState, FrameProcResult, Position,
};
use cgmath::{Matrix3, Matrix4, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, WebGlRenderingContext as GL};

#[derive(Serialize, Deserialize)]
pub(crate) struct Pipe {
    position: Position,
    fluid_box: FluidBox,
}

impl Pipe {
    pub(crate) fn new(position: &Position) -> Self {
        Pipe {
            position: *position,
            fluid_box: FluidBox::new(true, true),
        }
    }

    pub(crate) fn draw_int(
        structure: &dyn Structure,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        draw_center: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        let position = structure.position();
        let (x, y) = (position.x as f64 * 32., position.y as f64 * 32.);
        match state.image_pipe.as_ref() {
            Some(img) => {
                let connections = structure
                    .fluid_box()
                    .map(|fluid_boxes| {
                        Some(
                            fluid_boxes
                                .first()?
                                .connect_to
                                .iter()
                                .enumerate()
                                .filter(|(_, b)| b.is_some())
                                .fold(0, |acc, (i, _)| acc | (1 << i)),
                        )
                    })
                    .flatten()
                    .unwrap_or(0);
                // Skip drawing center dot? if there are no connections
                if !draw_center && connections == 0 {
                    return Ok(());
                }
                let sx = (connections % 4 * 32) as f64;
                let sy = ((connections / 4) * 32) as f64;
                context.draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    &img.bitmap,
                    sx,
                    sy,
                    32.,
                    32.,
                    x,
                    y,
                    32.,
                    32.,
                )?;
            }
            None => return Err(JsValue::from_str("pipe image not available")),
        }

        Ok(())
    }

    pub(crate) fn draw_gl_int(
        structure: &dyn Structure,
        state: &FactorishState,
        gl: &GL,
        depth: i32,
        draw_center: bool,
        is_ghost: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        }
        let position = structure.position();
        let (x, y) = (
            position.x as f32 + state.viewport.x as f32,
            position.y as f32 + state.viewport.y as f32,
        );
        let connections = structure
            .fluid_box()
            .map(|fluid_boxes| {
                Some(
                    fluid_boxes
                        .first()?
                        .connect_to
                        .iter()
                        .enumerate()
                        .filter(|(_, b)| b.is_some())
                        .fold(0, |acc, (i, _)| acc | (1 << i)),
                )
            })
            .flatten()
            .unwrap_or(0);
        // Skip drawing center dot? if there are no connections
        if !draw_center && connections == 0 {
            return Ok(());
        }
        let sx = (connections % 4) as f32;
        let sy = (connections / 4) as f32;
        let shader = state
            .assets
            .textured_shader
            .as_ref()
            .ok_or_else(|| js_str!("Shader not found"))?;
        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), if is_ghost { 0.5 } else { 1. });
        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_pipe));
        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            (Matrix3::from_nonuniform_scale(1. / 4., 1. / 4.)
                * Matrix3::from_translation(Vector2::new(sx, sy)))
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
}

impl Structure for Pipe {
    fn name(&self) -> &str {
        "Pipe"
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
        Self::draw_int(self, state, context, depth, true)
    }

    fn draw_gl(
        &self,
        state: &FactorishState,
        gl: &GL,
        depth: i32,
        is_ghost: bool,
    ) -> Result<(), JsValue> {
        Self::draw_gl_int(self, state, gl, depth, true, is_ghost)
    }

    fn desc(&self, _state: &FactorishState) -> String {
        self.fluid_box.desc()
        // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
        // "Outputs: <br>" +
        // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
    }

    fn frame_proc(
        &mut self,
        _me: StructureId,
        _state: &mut FactorishState,
        structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        self.fluid_box.simulate(structures);
        Ok(FrameProcResult::None)
    }

    fn fluid_box(&self) -> Option<Vec<&FluidBox>> {
        Some(vec![&self.fluid_box])
    }

    fn fluid_box_mut(&mut self) -> Option<Vec<&mut FluidBox>> {
        Some(vec![&mut self.fluid_box])
    }

    crate::serialize_impl!();
}
