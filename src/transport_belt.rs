use super::{
    drop_items::DropItem,
    gl::{
        utils::{enable_buffer, Flatten},
        ShaderBundle,
    },
    structure::{
        ItemResponse, ItemResponseResult, Structure, StructureBundle, StructureComponents,
        StructureDynIter,
    },
    FactorishState, Position, RotateErr, Rotation, SIM_DELTA_TIME, TILE_SIZE,
};
use cgmath::{Matrix3, Matrix4, Rad, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, WebGlRenderingContext as GL};

pub(crate) const BELT_SPEED: f64 = 0.25;

#[derive(Serialize, Deserialize)]
pub(crate) struct TransportBelt {}

impl TransportBelt {
    pub(crate) fn new(position: Position, rotation: Rotation) -> StructureBundle {
        StructureBundle {
            dynamic: Box::new(TransportBelt {}),
            components: StructureComponents::new_with_position_and_rotation(position, rotation),
        }
    }

    pub(crate) fn transport_item(
        rotation: Rotation,
        item: &DropItem,
    ) -> Result<ItemResponseResult, JsValue> {
        let vx = rotation.delta().0 as f64 * BELT_SPEED;
        let vy = rotation.delta().1 as f64 * BELT_SPEED;
        let ax = if rotation.is_vertical() {
            (item.x as f64 / TILE_SIZE).floor() * TILE_SIZE + TILE_SIZE / 2.
        } else {
            item.x as f64
        };
        let ay = if rotation.is_horizontal() {
            (item.y as f64 / TILE_SIZE).floor() * TILE_SIZE + TILE_SIZE / 2.
        } else {
            item.y as f64
        };
        let moved_x = ax + vx;
        let moved_y = ay + vy;
        Ok((ItemResponse::Move(moved_x, moved_y), None))
    }

    /// Apply transformation matrix for texture with belt scrolling.
    pub(crate) fn belt_texture_gl(
        gl: &GL,
        state: &FactorishState,
        shader: &ShaderBundle,
        transform: impl Fn(Matrix3<f32>) -> Matrix3<f32>,
    ) -> Result<(), JsValue> {
        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_belt));
        enable_buffer(&gl, &state.assets.screen_buffer, 2, shader.vertex_position);
        let sx = -((state.sim_time / SIM_DELTA_TIME * BELT_SPEED / TILE_SIZE) % 1.) as f32;
        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            transform(Matrix3::from_translation(Vector2::new(sx, 0.))).flatten(),
        );

        Ok(())
    }
}

impl Structure for TransportBelt {
    fn name(&self) -> &'static str {
        "Transport Belt"
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
        match state.image_belt.as_ref() {
            Some(img) => {
                let (x, y) = if let Some(position) = components.position.as_ref() {
                    (position.x as f64 * 32., position.y as f64 * 32.)
                } else {
                    (0., 0.)
                };
                context.save();
                context.translate(x + 16., y + 16.)?;
                components
                    .rotation
                    .map(|rotation| context.rotate(rotation.angle_rad()));
                context.translate(-(x + 16.), -(y + 16.))?;
                for i in 0..2 {
                    context
                        .draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                            &img.bitmap,
                            i as f64 * 32. - (state.sim_time * 16.) % 32.,
                            0.,
                            32.,
                            32.,
                            x,
                            y,
                            32.,
                            32.,
                        )?;
                }
                context.restore();
            }
            None => return Err(JsValue::from_str("belt image not available")),
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
        let position = components.get_position(self)?;
        let rotation = components.get_rotation(self)?;

        let (x, y) = (
            position.x as f32 + state.viewport.x as f32,
            position.y as f32 + state.viewport.y as f32,
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
        TransportBelt::belt_texture_gl(gl, state, shader, |scroll| {
            scroll * Matrix3::from_angle_z(Rad(-rotation.angle_rad() as f32))
        })?;

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

    fn movable(&self) -> bool {
        true
    }

    fn rotate(
        &mut self,
        components: &mut StructureComponents,
        _state: &mut FactorishState,
        _others: &StructureDynIter,
    ) -> Result<(), RotateErr> {
        if let Some(ref mut rotation) = components.rotation {
            *rotation = rotation.next();
            Ok(())
        } else {
            Err(RotateErr::NotSupported)
        }
    }

    fn set_rotation(
        &mut self,
        components: &mut StructureComponents,
        rotation: &Rotation,
    ) -> Result<(), ()> {
        if let Some(ref mut self_rotation) = components.rotation {
            *self_rotation = *rotation;
        }
        Ok(())
    }

    fn item_response(
        &mut self,
        components: &mut StructureComponents,
        item: &DropItem,
    ) -> Result<ItemResponseResult, JsValue> {
        Self::transport_item(
            components
                .rotation
                .ok_or_else(|| js_str!("TransportBelt without Rotation component"))?,
            item,
        )
    }

    crate::serialize_impl!();
}
