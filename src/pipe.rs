use super::{
    gl::utils::{enable_buffer, Flatten},
    structure::{Position, Structure, StructureBundle, StructureComponents},
    water_well::FluidBox,
    FactorishState, Rotation, TILE_SIZE, TILE_SIZE_I,
};
use cgmath::{Matrix3, Matrix4, Rad, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, WebGlRenderingContext as GL};

#[derive(Serialize, Deserialize)]
pub(crate) struct Pipe;

impl Pipe {
    pub(crate) fn new(position: Position) -> StructureBundle {
        StructureBundle::new(
            Box::new(Pipe),
            Some(position),
            None,
            None,
            None,
            None,
            vec![FluidBox::new(true, true)],
        )
    }

    pub(crate) fn draw_int(
        _structure: &dyn Structure,
        components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        draw_center: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        let (x, y) = if let Some(position) = components.position.as_ref() {
            (position.x as f64 * TILE_SIZE, position.y as f64 * TILE_SIZE)
        } else {
            (0., 0.)
        };
        match state.image_pipe.as_ref() {
            Some(img) => {
                let connections = components
                    .fluid_boxes
                    .first()
                    .map(|fluid_box| {
                        fluid_box
                            .connect_to
                            .iter()
                            .enumerate()
                            .filter(|(_, b)| b.is_some())
                            .fold(0, |acc, (i, _)| acc | (1 << i))
                    })
                    .unwrap_or(0);
                // Skip drawing center dot? if there are no connections
                if !draw_center && connections == 0 {
                    return Ok(());
                }
                let sx = (connections % 4 * TILE_SIZE_I) as f64;
                let sy = ((connections / 4) * TILE_SIZE_I) as f64;
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
        dynamic: &dyn Structure,
        components: &StructureComponents,
        state: &FactorishState,
        gl: &GL,
        depth: i32,
        draw_center: bool,
        is_ghost: bool,
    ) -> Result<(), JsValue> {
        match depth {
            0 => {
                Self::draw_pipe_gl(gl, dynamic, components, state, draw_center, is_ghost)?;
            }
            2 => {
                if state.alt_mode {
                    Self::draw_flow_overlay_gl(gl, dynamic, components, state)?;
                }
            }
            _ => (),
        }
        Ok(())
    }

    fn draw_pipe_gl(
        gl: &GL,
        dynamic: &dyn Structure,
        components: &StructureComponents,
        state: &FactorishState,
        draw_center: bool,
        is_ghost: bool,
    ) -> Result<(), JsValue> {
        let position = components.get_position(dynamic)?;
        let (x, y) = (
            position.x as f32 + state.viewport.x as f32,
            position.y as f32 + state.viewport.y as f32,
        );

        let connections = components.fluid_boxes.iter().fold(0, |con, fluid_box| {
            con | fluid_box
                .connect_to
                .iter()
                .enumerate()
                .filter(|(_, b)| b.is_some())
                .fold(0, |acc, (i, _)| acc | (1 << i))
        });
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
            (Matrix3::from_nonuniform_scale(1. / 4., 1. / 8.)
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

    fn draw_flow_overlay_gl(
        gl: &GL,
        dynamic: &dyn Structure,
        components: &StructureComponents,
        state: &FactorishState,
    ) -> Result<(), JsValue> {
        let position = components.get_position(dynamic)?;
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
        gl.uniform1f(shader.alpha_loc.as_ref(), 1.);
        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_flow_direction));
        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            (Matrix3::from_scale(0.5) * Matrix3::from_translation(Vector2::new(1., 1.))).flatten(),
        );

        if let Some(flows) = components
            .fluid_boxes
            .first()
            .and_then(|fluid_box| Some(fluid_box.flow))
        {
            const ROTATIONS: [Rotation; 4] = [
                Rotation::Left,
                Rotation::Top,
                Rotation::Right,
                Rotation::Bottom,
            ];
            const MIN_FLOW: f64 = 1e-6;
            for (_i, (flow, rotation)) in flows.iter().zip(ROTATIONS.iter()).enumerate() {
                if MIN_FLOW < flow.abs() {
                    let origin = rotation.delta();
                    enable_buffer(&gl, &state.assets.rect_buffer, 2, shader.vertex_position);
                    gl.uniform_matrix4fv_with_f32_array(
                        shader.transform_loc.as_ref(),
                        false,
                        (state.get_world_transform()?
                            * Matrix4::from_scale(2.)
                            * Matrix4::from_translation(Vector3::new(
                                x + 0.5 + origin.0 as f32 * 0.5,
                                y + 0.5 + origin.1 as f32 * 0.5,
                                0.,
                            ))
                            * Matrix4::from_angle_z(
                                Rad(rotation.next().next().angle_rad() as f32),
                            )
                            * Matrix4::from_scale(
                                0.1 + 0.25 * (1. / 6.) * (flow.abs() / MIN_FLOW).log10() as f32,
                            ))
                        .flatten(),
                    );
                    gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
                }
            }
        }
        Ok(())
    }
}

impl Structure for Pipe {
    fn name(&self) -> &'static str {
        "Pipe"
    }

    fn draw(
        &self,
        components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        Self::draw_int(self, components, state, context, depth, true)
    }

    fn draw_gl(
        &self,
        components: &StructureComponents,
        state: &FactorishState,
        gl: &GL,
        depth: i32,
        is_ghost: bool,
    ) -> Result<(), JsValue> {
        Self::draw_gl_int(self, components, state, gl, depth, true, is_ghost)
    }

    fn desc(&self, components: &StructureComponents, _state: &FactorishState) -> String {
        components
            .fluid_boxes
            .iter()
            .map(|p| p.desc())
            .fold("".to_string(), |acc, s| acc + &s)
        // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
        // "Outputs: <br>" +
        // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
    }

    crate::serialize_impl!();
}
