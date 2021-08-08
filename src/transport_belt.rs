use super::{
    drop_items::DropItem,
    gl::utils::enable_buffer,
    structure::{ItemResponse, ItemResponseResult, Structure, StructureDynIter},
    FactorishState, Position, RotateErr, Rotation, TILE_SIZE,
};
use cgmath::{Matrix3, Matrix4, Rad, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, WebGlRenderingContext as GL};

#[derive(Serialize, Deserialize)]
pub(crate) struct TransportBelt {
    position: Position,
    rotation: Rotation,
}

impl TransportBelt {
    pub(crate) fn new(x: i32, y: i32, rotation: Rotation) -> Self {
        TransportBelt {
            position: Position { x, y },
            rotation,
        }
    }

    pub(crate) fn transport_item(
        rotation: Rotation,
        item: &DropItem,
    ) -> Result<ItemResponseResult, ()> {
        let vx = rotation.delta().0;
        let vy = rotation.delta().1;
        let ax = if rotation.is_vertcial() {
            (item.x as f64 / TILE_SIZE).floor() * TILE_SIZE + TILE_SIZE / 2.
        } else {
            item.x as f64
        };
        let ay = if rotation.is_horizontal() {
            (item.y as f64 / TILE_SIZE).floor() * TILE_SIZE + TILE_SIZE / 2.
        } else {
            item.y as f64
        };
        let moved_x = ax as i32 + vx;
        let moved_y = ay as i32 + vy;
        Ok((ItemResponse::Move(moved_x, moved_y), None))
    }
}

impl Structure for TransportBelt {
    fn name(&self) -> &str {
        "Transport Belt"
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
        if depth != 0 {
            return Ok(());
        };
        match state.image_belt.as_ref() {
            Some(img) => {
                let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
                context.save();
                context.translate(x + 16., y + 16.)?;
                context.rotate(self.rotation.angle_rad())?;
                context.translate(-(x + 16.), -(y + 16.))?;
                for i in 0..2 {
                    context
                        .draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                            &img.bitmap,
                            i as f64 * 32. - (state.sim_time * 16.) % 32.,
                            0.,
                            32.,
                            32.,
                            self.position.x as f64 * 32.,
                            self.position.y as f64 * 32.,
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
        state: &FactorishState,
        gl: &GL,
        depth: i32,
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        let (x, y) = (
            self.position.x as f32 + state.viewport.x as f32,
            self.position.y as f32 + state.viewport.y as f32,
        );
        match depth {
            0 => {
                let shader = state
                    .assets
                    .textured_shader
                    .as_ref()
                    .ok_or_else(|| js_str!("Shader not found"))?;
                gl.use_program(Some(&shader.program));
                gl.active_texture(GL::TEXTURE0);
                gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_belt));
                enable_buffer(&gl, &state.assets.screen_buffer, 2, shader.vertex_position);
                let sx = -((state.sim_time * 16.) % 32. / 32.) as f32;
                gl.uniform_matrix3fv_with_f32_array(
                    shader.tex_transform_loc.as_ref(),
                    false,
                    <Matrix3<f32> as AsRef<[f32; 9]>>::as_ref(
                        &(Matrix3::from_translation(Vector2::new(sx, 0.))
                            * Matrix3::from_angle_z(Rad(-self.rotation.angle_rad() as f32))),
                    ),
                );

                gl.uniform_matrix4fv_with_f32_array(
                    shader.transform_loc.as_ref(),
                    false,
                    <Matrix4<f32> as AsRef<[f32; 16]>>::as_ref(
                        &(state.get_world_transform()?
                            * Matrix4::from_scale(2.)
                            * Matrix4::from_translation(Vector3::new(x, y, 0.))),
                    ),
                );
                gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
            }
            _ => (),
        }
        Ok(())
    }

    fn movable(&self) -> bool {
        true
    }

    fn rotate(
        &mut self,
        _state: &mut FactorishState,
        _others: &StructureDynIter,
    ) -> Result<(), RotateErr> {
        self.rotation = self.rotation.next();
        Ok(())
    }

    fn set_rotation(&mut self, rotation: &Rotation) -> Result<(), ()> {
        self.rotation = *rotation;
        Ok(())
    }

    fn item_response(&mut self, item: &DropItem) -> Result<ItemResponseResult, ()> {
        Self::transport_item(self.rotation, item)
    }

    crate::serialize_impl!();
}
