use super::{
    gl::utils::{enable_buffer, Flatten},
    pipe::Pipe,
    structure::{Structure, StructureBundle, StructureComponents, StructureDynIter, StructureId},
    water_well::{FluidBox, FluidType},
    FactorishState, FrameProcResult, Position,
};
use cgmath::{Matrix3, Matrix4, Vector3};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, WebGlRenderingContext as GL};

#[derive(Serialize, Deserialize)]
pub(crate) struct OffshorePump;

impl OffshorePump {
    pub(crate) fn new(position: &Position) -> StructureBundle {
        StructureBundle::new(
            Box::new(OffshorePump),
            Some(*position),
            None,
            None,
            None,
            None,
            vec![FluidBox::new(false, true).set_type(&FluidType::Water)],
        )
    }
}

impl Structure for OffshorePump {
    fn name(&self) -> &str {
        "Offshore Pump"
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
        Pipe::draw_int(self, components, state, context, depth, false)?;
        let position = components
            .position
            .ok_or_else(|| js_str!("Offshore Pump without Position"))?;
        let (x, y) = (position.x as f64 * 32., position.y as f64 * 32.);
        match state.image_offshore_pump.as_ref() {
            Some(img) => {
                context.draw_image_with_image_bitmap(&img.bitmap, x, y)?;
            }
            None => return Err(JsValue::from_str("furnace image not available")),
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
        if depth != 0 {
            return Ok(());
        };
        let position = components
            .position
            .ok_or_else(|| js_str!("OffshorePump without Position"))?;
        Pipe::draw_gl_int(components, state, gl, depth, false, is_ghost)?;
        let (x, y) = (
            position.x as f32 + state.viewport.x as f32,
            position.y as f32 + state.viewport.y as f32,
        );
        let shader = state
            .assets
            .textured_shader
            .as_ref()
            .ok_or_else(|| js_str!("Shader not found"))?;
        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), if is_ghost { 0.5 } else { 1. });
        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_offshore_pump));
        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::from_scale(1.).flatten(),
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

    fn desc(&self, components: &StructureComponents, _state: &FactorishState) -> String {
        format!(
            "{}<br>{}",
            components
                .fluid_boxes
                .iter()
                .map(|fb| fb.desc())
                .fold(String::new(), |s, desc| s + "<br>" + &desc),
            "Outputs: Water<br>",
        )
    }

    fn frame_proc(
        &mut self,
        _me: StructureId,
        components: &mut StructureComponents,
        _state: &mut FactorishState,
        _structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        let output_fluid_box = components.fluid_boxes.get_mut(0).ok_or_else(|| ())?;
        output_fluid_box.amount = (output_fluid_box.amount + 1.).min(output_fluid_box.max_amount);
        Ok(FrameProcResult::None)
    }

    crate::serialize_impl!();
}
