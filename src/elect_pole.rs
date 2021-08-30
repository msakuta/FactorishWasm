use super::{
    gl::{
        assets::WIRE_SEGMENTS,
        utils::{enable_buffer, vertex_buffer_data, Flatten},
    },
    structure::{Structure, StructureBundle, StructureComponents},
    FactorishState, Position, TILE_SIZE_F, WIRE_ATTACH_X, WIRE_ATTACH_Y, WIRE_HANG,
};
use cgmath::{Matrix3, Matrix4, Vector3};
use serde::{Deserialize, Serialize};
use slice_of_array::SliceFlatExt;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, WebGlRenderingContext as GL};

const WIRE_WIDTH: f32 = 0.5;

#[derive(Serialize, Deserialize)]
pub(crate) struct ElectPole {
    power: f64,
}

impl ElectPole {
    pub(crate) fn new(position: Position) -> StructureBundle {
        StructureBundle {
            dynamic: Box::new(ElectPole { power: 0. }),
            components: StructureComponents::new_with_position(position),
        }
    }
}

impl Structure for ElectPole {
    fn name(&self) -> &str {
        "Electric Pole"
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
        match state.image_elect_pole.as_ref() {
            Some(img) => {
                // let (front, mid) = state.structures.split_at_mut(i);
                // let (center, last) = mid
                //     .split_first_mut()
                //     .ok_or(JsValue::from_str("Structures split fail"))?;

                // We could split and chain like above, but we don't have to, as long as we deal with immutable
                // references.
                context.draw_image_with_image_bitmap(&img.bitmap, x, y)?;
            }
            None => return Err(JsValue::from_str("elect-pole image not available")),
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
        }
        let position = components
            .position
            .ok_or_else(|| js_str!("OreMine without Position"))?;
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
        gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_elect_pole));
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

    fn power_sink(&self) -> bool {
        true
    }

    fn power_source(&self) -> bool {
        true
    }

    fn power_outlet(&mut self, components: &mut StructureComponents, demand: f64) -> Option<f64> {
        let energy = components.energy.as_mut()?;
        let power = demand.min(energy.value);
        energy.value -= power;
        Some(power)
    }

    fn wire_reach(&self) -> u32 {
        5
    }

    crate::serialize_impl!();
}

pub(crate) fn draw_wire_gl(
    gl: &GL,
    start: Position,
    end: Position,
    width: f32,
) -> Result<(), JsValue> {
    let start_pos = (
        start.x as f32 * TILE_SIZE_F + WIRE_ATTACH_X as f32,
        start.y as f32 * TILE_SIZE_F + WIRE_ATTACH_Y as f32,
    );
    let end_pos = (
        end.x as f32 * TILE_SIZE_F + WIRE_ATTACH_X as f32,
        end.y as f32 * TILE_SIZE_F + WIRE_ATTACH_Y as f32,
    );
    let dx = end_pos.0 - start_pos.0;
    let dy = end_pos.1 - start_pos.1;
    let dist = (dx * dx + dy * dy).sqrt();
    let len2 = (WIRE_SEGMENTS as f64 / 2.).sqrt() as f32;
    let center_points = (0..=WIRE_SEGMENTS)
        .map(|i| {
            let fi = i as f32;
            let fi2 = i as f32 - WIRE_SEGMENTS as f32 / 2.;
            [
                start_pos.0 * (1. - fi / WIRE_SEGMENTS as f32)
                    + end_pos.0 * fi / WIRE_SEGMENTS as f32,
                start_pos.1 * (1. - fi / WIRE_SEGMENTS as f32)
                    + end_pos.1 * fi / WIRE_SEGMENTS as f32
                    + (dist * (1. - fi2 * fi2 / len2) * WIRE_HANG as f32) as f32 / TILE_SIZE_F,
            ]
        })
        .collect::<Vec<_>>();

    let mut perp = [0., 0.];
    let mut points = center_points
        .iter()
        .take(WIRE_SEGMENTS as usize)
        .zip(center_points.iter().skip(1))
        .map(|(prev, next)| {
            let diff = [next[0] - prev[0], next[1] - prev[1]];
            let len = (diff[0] * diff[0] + diff[1] * diff[1]).sqrt();
            let factor = WIRE_WIDTH * width / len;
            perp = [diff[1] * factor, -diff[0] * factor];
            [
                prev[0] + perp[0],
                prev[1] + perp[1],
                prev[0] - perp[0],
                prev[1] - perp[1],
            ]
        })
        .collect::<Vec<_>>();
    let back = center_points.last().unwrap();
    points.push([
        back[0] + perp[0],
        back[1] + perp[1],
        back[0] - perp[0],
        back[1] - perp[1],
    ]);

    vertex_buffer_data(&gl, &points.flat());

    gl.draw_arrays(GL::TRIANGLE_STRIP, 0, points.len() as i32 * 2);
    Ok(())
}
