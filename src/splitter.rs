use super::{
    drop_items::DropItem,
    gl::{
        utils::{enable_buffer, Flatten},
        ShaderBundle,
    },
    structure::{
        BoundingBox, ItemResponse, ItemResponseResult, RotateErr, Size, Structure, StructureDynIter,
    },
    transport_belt::{TransportBelt, BELT_SPEED},
    FactorishState, Position, Rotation, TILE_SIZE,
};
use cgmath::{Matrix3, Matrix4, Rad, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, WebGlRenderingContext as GL};

#[derive(Serialize, Deserialize)]
pub(crate) struct Splitter {
    position: Position,
    rotation: Rotation,
    direction: i8,
}

impl Splitter {
    pub(crate) fn new(x: i32, y: i32, rotation: Rotation) -> Self {
        Splitter {
            position: Position { x, y },
            rotation,
            direction: 0,
        }
    }
}

impl Structure for Splitter {
    fn name(&self) -> &str {
        "Splitter"
    }

    fn position(&self) -> &Position {
        &self.position
    }

    fn size(&self) -> Size {
        Size {
            width: 1,
            height: 2,
        }
    }

    fn bounding_box(&self) -> BoundingBox {
        let position = self.position();
        match self.rotation {
            Rotation::Left => BoundingBox {
                x0: position.x,
                y0: position.y - 1,
                x1: position.x + 1,
                y1: position.y + 1,
            },
            Rotation::Top => BoundingBox {
                x0: position.x,
                y0: position.y,
                x1: position.x + 2,
                y1: position.y + 1,
            },
            Rotation::Right => BoundingBox {
                x0: position.x,
                y0: position.y,
                x1: position.x + 1,
                y1: position.y + 2,
            },
            Rotation::Bottom => BoundingBox {
                x0: position.x - 1,
                y0: position.y,
                x1: position.x + 1,
                y1: position.y + 1,
            },
        }
    }

    fn draw(
        &self,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 && depth != 1 {
            return Ok(());
        }
        let mut ret = Ok(());
        let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
        context.save();
        context.translate(x + 16., y + 16.)?;
        context.rotate(self.rotation.angle_rad())?;
        context.translate(-(x + 16.), -(y + 16.))?;
        if depth == 0 {
            if let Some(belt) = state.image_belt.as_ref() {
                for n in 0..2 {
                    for i in 0..2 {
                        context
                            .draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                                &belt.bitmap,
                                i as f64 * 32. - (state.sim_time * 16.) % 32.,
                                0.,
                                32.,
                                32.,
                                self.position.x as f64 * 32.,
                                self.position.y as f64 * 32. + n as f64 * TILE_SIZE,
                                32.,
                                32.,
                            )?;
                    }
                }
            } else {
                ret = js_err!("belt image not available");
            }
        } else if depth == 1 {
            if let Some(splitter) = state.image_splitter.as_ref() {
                if depth == 1 {
                    for x in 0..2 {
                        context
                            .draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                                &splitter.bitmap,
                                0.,
                                (if self.direction == 0 { 1 - x } else { x }) as f64 * TILE_SIZE,
                                TILE_SIZE,
                                TILE_SIZE,
                                self.position.x as f64 * TILE_SIZE,
                                (self.position.y + x) as f64 * TILE_SIZE,
                                TILE_SIZE,
                                TILE_SIZE,
                            )?;
                    }
                }
            } else {
                ret = js_err!("splitter image not available");
            }
        }
        context.restore();

        ret
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
            gl.uniform_matrix4fv_with_f32_array(
                shader.transform_loc.as_ref(),
                false,
                (state.get_world_transform()?
                    * Matrix4::from_scale(2.)
                    * Matrix4::from_translation(Vector3::new(x + 0.5, y + 0.5, 0.))
                    * Matrix4::from_angle_z(Rad(self.rotation.angle_rad() as f32))
                    * Matrix4::from_translation(Vector3::new(-0.5, -0.5, 0.))
                    * Matrix4::from_nonuniform_scale(1., 2., 1.))
                .flatten(),
            );
            Ok(())
        };

        match depth {
            0 => {
                let shader = get_shader()?;
                TransportBelt::belt_texture_gl(gl, state, shader, |scroll| {
                    Matrix3::from_nonuniform_scale(1., 2.) * scroll
                })?;

                shape(shader)?;

                gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
            }
            1 => {
                let shader = get_shader()?;
                gl.active_texture(GL::TEXTURE0);
                gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_splitter));
                enable_buffer(&gl, &state.assets.screen_buffer, 2, shader.vertex_position);
                let sy = (if self.direction == 0 { 0.5 } else { 0. }) as f32;
                gl.uniform_matrix3fv_with_f32_array(
                    shader.tex_transform_loc.as_ref(),
                    false,
                    Matrix3::from_translation(Vector2::new(0., sy)).flatten(),
                );

                shape(shader)?;

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
        let vx = self.rotation.delta().0 as f64 * BELT_SPEED;
        let vy = self.rotation.delta().1 as f64 * BELT_SPEED;
        let mut ax = if self.rotation.is_vertcial() {
            (item.x as f64 / TILE_SIZE).floor() * TILE_SIZE + TILE_SIZE / 2.
        } else {
            item.x as f64
        };
        let mut ay = if self.rotation.is_horizontal() {
            (item.y as f64 / TILE_SIZE).floor() * TILE_SIZE + TILE_SIZE / 2.
        } else {
            item.y as f64
        };
        let Position { x: tx, y: ty } = self.position;
        let halftilesize = TILE_SIZE / 2.;
        let mut postdirection = false;
        let shift_direction = self.rotation.clone().next().delta();
        if self.rotation.is_horizontal() {
            // Detect the point where the item passes over the mid point of this entity.
            if ((ax + halftilesize) / TILE_SIZE).floor()
                != ((ax + vx as f64 + halftilesize) / TILE_SIZE).floor()
            {
                ay = (ty + self.direction as i32 * shift_direction.1) as f64 * TILE_SIZE
                    + TILE_SIZE / 2.;
                postdirection = true; // Signal to switch direction
            }
        } else if ((ay + halftilesize) / TILE_SIZE).floor()
            != ((ay + vy as f64 + halftilesize) / TILE_SIZE).floor()
        {
            ax = (tx + self.direction as i32 * shift_direction.0) as f64 * TILE_SIZE
                + TILE_SIZE / 2.;
            postdirection = true; // Signal to switch direction
        }

        if postdirection {
            self.direction = (self.direction + 1) % 2;
        }

        let moved_x = ax + vx;
        let moved_y = ay + vy;
        Ok((ItemResponse::Move(moved_x, moved_y), None))
    }

    crate::serialize_impl!();
}
