use super::{
    gl::utils::{enable_buffer, Flatten},
    structure::{Structure, StructureDynIter, StructureId},
    water_well::FluidBox,
    FactorishState, FrameProcResult, Position, Rotation, TILE_SIZE,
};
use cgmath::{Matrix3, Matrix4, Rad, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, WebGlRenderingContext as GL};

const UNDERGROUND_REACH: i32 = 10;

#[derive(Serialize, Deserialize)]
pub(crate) struct UndergroundPipe {
    position: Position,
    rotation: Rotation,

    /// Items in the underground belt. First value is the absolute position in the underground belt
    /// from the entrance.
    fluid_box: FluidBox,
}

impl UndergroundPipe {
    pub(crate) fn new(position: Position, rotation: Rotation) -> Self {
        Self {
            position,
            rotation,
            fluid_box: FluidBox::new(true, true),
        }
    }

    /// Distance to possibly connecting underground belt.
    fn distance(&self, target: &Position) -> Option<i32> {
        let src = self.position;
        if !match self.rotation {
            Rotation::Left | Rotation::Right => target.y == src.y,
            Rotation::Top | Rotation::Bottom => target.x == src.x,
        } {
            return None;
        }
        let dx = target.x - src.x;
        let dy = target.y - src.y;
        Some(match self.rotation {
            Rotation::Left => dx,
            Rotation::Right => -dx,
            Rotation::Top => dy,
            Rotation::Bottom => -dy,
        })
    }
}

impl Structure for UndergroundPipe {
    fn name(&self) -> &str {
        "Underground Pipe"
    }

    fn position(&self) -> &Position {
        &self.position
    }

    fn rotation(&self) -> Option<Rotation> {
        Some(self.rotation)
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
        };
        match state.image_pipe.as_ref() {
            Some(img) => {
                context.save();
                context.draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    &img.bitmap,
                    match self.rotation {
                        Rotation::Left => 0.,
                        Rotation::Top => 1.,
                        Rotation::Right => 2.,
                        Rotation::Bottom => 3.,
                    } * TILE_SIZE,
                    TILE_SIZE * 4.,
                    TILE_SIZE,
                    TILE_SIZE,
                    self.position.x as f64 * TILE_SIZE,
                    self.position.y as f64 * TILE_SIZE,
                    TILE_SIZE,
                    TILE_SIZE,
                )?;
                context.restore();
            }
            None => return Err(JsValue::from_str("underground pipe image not available")),
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
                gl.uniform1f(shader.alpha_loc.as_ref(), if is_ghost { 0.5 } else { 1. });
                gl.active_texture(GL::TEXTURE0);
                gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_pipe));
                let sx = ((self.rotation.angle_4() + 2) % 4) as f32;
                gl.uniform_matrix3fv_with_f32_array(
                    shader.tex_transform_loc.as_ref(),
                    false,
                    (Matrix3::from_nonuniform_scale(1. / 4., 1. / 8.)
                        * Matrix3::from_translation(Vector2::new(sx, 4.)))
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
            }
            2 => {
                if state.alt_mode && matches!(self.rotation, Rotation::Left | Rotation::Top) {
                    if let Some(dist) = self.fluid_box.connect_to[self.rotation.angle_4() as usize]
                        .and_then(|id| state.get_structure(id))
                        .and_then(|s| self.distance(s.position()))
                    {
                        let shader = state
                            .assets
                            .textured_shader
                            .as_ref()
                            .ok_or_else(|| js_str!("Shader not found"))?;
                        gl.use_program(Some(&shader.program));
                        gl.active_texture(GL::TEXTURE0);
                        gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_connect_overlay));

                        let scales = ((dist + 1) as f32, 1.);
                        let (scale_x, scale_y) = if self.rotation.is_horizontal() {
                            (scales.0, scales.1)
                        } else {
                            (scales.1, scales.0)
                        };
                        gl.uniform_matrix3fv_with_f32_array(
                            shader.tex_transform_loc.as_ref(),
                            false,
                            (Matrix3::from_angle_z(Rad(self.rotation.angle_rad() as f32))
                                * Matrix3::from_nonuniform_scale(scale_x, scale_y))
                            .flatten(),
                        );

                        enable_buffer(&gl, &state.assets.screen_buffer, 2, shader.vertex_position);
                        gl.uniform_matrix4fv_with_f32_array(
                            shader.transform_loc.as_ref(),
                            false,
                            (state.get_world_transform()?
                                * Matrix4::from_scale(2.)
                                * Matrix4::from_translation(Vector3::new(x, y, 0.))
                                * Matrix4::from_nonuniform_scale(scale_x, scale_y, 1.))
                            .flatten(),
                        );
                        gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
                    }
                }
            }
            _ => (),
        }
        Ok(())
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

    fn set_rotation(&mut self, rotation: &Rotation) -> Result<(), ()> {
        self.rotation = *rotation;
        Ok(())
    }

    fn desc(&self, _state: &FactorishState) -> String {
        self.fluid_box.desc()
    }

    fn on_construction(
        &mut self,
        other_id: StructureId,
        other: &dyn Structure,
        others: &StructureDynIter,
        construct: bool,
    ) -> Result<(), JsValue> {
        if !construct {
            return Ok(());
        }

        if other.rotation() != Some(self.rotation.next().next()) {
            return Ok(());
        }

        let underground_reach = if let Some(reach) = other.under_pipe_reach() {
            reach.min(UNDERGROUND_REACH)
        } else {
            return Ok(());
        };

        let opos = *other.position();
        let d = if let Some(d) = self.distance(&opos) {
            d
        } else {
            return Ok(());
        };

        if d < 1 || underground_reach < d {
            return Ok(());
        }

        let connect_index = self.rotation.angle_4() as usize;

        // If there is already an underground belt with shorter distance, don't connect to the new one.
        if let Some(target) =
            self.fluid_box.connect_to[connect_index].and_then(|target| others.get(target))
        {
            let target_pos = target.position();
            if let Some(target_d) = self.distance(target_pos) {
                if 0 < target_d && target_d < d {
                    return Ok(());
                }
            }
        }

        self.fluid_box.connect_to[connect_index] = Some(other_id);

        Ok(())
    }

    fn on_construction_self(
        &mut self,
        _id: StructureId,
        others: &StructureDynIter,
        _construct: bool,
    ) -> Result<(), JsValue> {
        let connect_index = self.rotation.angle_4() as usize;

        if let Some((id, _)) = others.dyn_iter_id().find(|(_, other)| {
            if other.rotation() != Some(self.rotation.next().next()) {
                return false;
            }

            let underground_reach = if let Some(reach) = other.under_pipe_reach() {
                reach.min(UNDERGROUND_REACH)
            } else {
                return false;
            };

            let opos = *other.position();
            let d = if let Some(d) = self.distance(&opos) {
                d
            } else {
                return false;
            };

            if d < 1 || underground_reach < d {
                return false;
            }

            // If there is already an underground belt with shorter distance, don't connect to the new one.
            if let Some(target) =
                self.fluid_box.connect_to[connect_index].and_then(|target| others.get(target))
            {
                let target_pos = target.position();
                if let Some(target_d) = self.distance(target_pos) {
                    if 0 < target_d && target_d < d {
                        return false;
                    }
                }
            }

            true
        }) {
            self.fluid_box.connect_to[connect_index] = Some(id);
        }
        Ok(())
    }

    fn fluid_connections(&self) -> [bool; 4] {
        let mut ret = [false; 4];
        ret[self.rotation.angle_4() as usize] = true;
        ret
    }

    fn under_pipe_reach(&self) -> Option<i32> {
        Some(UNDERGROUND_REACH)
    }

    fn fluid_box(&self) -> Option<Vec<&FluidBox>> {
        Some(vec![&self.fluid_box])
    }

    fn fluid_box_mut(&mut self) -> Option<Vec<&mut FluidBox>> {
        Some(vec![&mut self.fluid_box])
    }

    crate::serialize_impl!();
}
