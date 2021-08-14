use super::{
    gl::utils::{enable_buffer, Flatten},
    structure::{Structure, StructureBundle, StructureComponents, StructureDynIter, StructureId},
    water_well::FluidBox,
    FactorishState, FrameProcResult, Position, Rotation, TILE_SIZE,
};
use cgmath::{Matrix3, Matrix4, Rad, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, WebGlRenderingContext as GL};

const UNDERGROUND_REACH: i32 = 10;

#[derive(Serialize, Deserialize)]
pub(crate) struct UndergroundPipe {}

impl UndergroundPipe {
    pub(crate) fn new(position: Position, rotation: Rotation) -> StructureBundle {
        StructureBundle {
            dynamic: Box::new(Self {}),
            components: StructureComponents {
                position: Some(position),
                rotation: Some(rotation),
                fluid_boxes: vec![FluidBox::new(true, true)],
                ..StructureComponents::default()
            },
        }
    }

    /// Distance to possibly connecting underground belt.
    fn distance(&self, components: &StructureComponents, target: &Position) -> Option<i32> {
        let src = components.position?;
        if !match components.rotation? {
            Rotation::Left | Rotation::Right => target.y == src.y,
            Rotation::Top | Rotation::Bottom => target.x == src.x,
        } {
            return None;
        }
        let dx = target.x - src.x;
        let dy = target.y - src.y;
        Some(match components.rotation? {
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

    fn draw(
        &self,
        components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 && depth != 1 {
            return Ok(());
        };
        let position = components.get_position()?;
        let rotation = components.get_rotation()?;
        match state.image_pipe.as_ref() {
            Some(img) => {
                context.save();
                context.draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    &img.bitmap,
                    match rotation {
                        Rotation::Left => 0.,
                        Rotation::Top => 1.,
                        Rotation::Right => 2.,
                        Rotation::Bottom => 3.,
                    } * TILE_SIZE,
                    TILE_SIZE * 4.,
                    TILE_SIZE,
                    TILE_SIZE,
                    position.x as f64 * TILE_SIZE,
                    position.y as f64 * TILE_SIZE,
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
        components: &StructureComponents,
        state: &FactorishState,
        gl: &GL,
        depth: i32,
        is_ghost: bool,
    ) -> Result<(), JsValue> {
        let position = components
            .position
            .ok_or_else(|| js_str!("Underground belt without Position"))?;
        let rotation = components
            .rotation
            .ok_or_else(|| js_str!("Underground belt without Rotation"))?;

        let (x, y) = (
            position.x as f32 + state.viewport.x as f32,
            position.y as f32 + state.viewport.y as f32,
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
                let sx = ((rotation.angle_4() + 2) % 4) as f32;
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
                let fluid_box = components.get_fluid_box_first()?;
                let on_cursor = state.cursor == Some([position.x, position.y]);
                if state.alt_mode && matches!(rotation, Rotation::Left | Rotation::Top) || on_cursor
                {
                    if let Some(dist) = fluid_box.connect_to[rotation.angle_4() as usize]
                        .and_then(|id| state.get_structure(id))
                        .and_then(|s| self.distance(components, &s.components.position?))
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
                        let (scale_x, scale_y) = if rotation.is_horizontal() {
                            (scales.0, scales.1)
                        } else {
                            (scales.1, scales.0)
                        };
                        let x = if rotation == Rotation::Right {
                            x - dist as f32
                        } else {
                            x
                        };
                        let y = if rotation == Rotation::Bottom {
                            y - dist as f32
                        } else {
                            y
                        };

                        gl.uniform_matrix3fv_with_f32_array(
                            shader.tex_transform_loc.as_ref(),
                            false,
                            (Matrix3::from_angle_z(Rad(rotation.angle_rad() as f32))
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
        components: &mut StructureComponents,
        state: &mut FactorishState,
        structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        let position = components.position.ok_or(())?;
        for fb in &mut components.fluid_boxes {
            fb.simulate(&position, state, structures);
        }
        Ok(FrameProcResult::None)
    }

    fn on_construction(
        &mut self,
        components: &mut StructureComponents,
        other_id: StructureId,
        other: &StructureBundle,
        others: &StructureDynIter,
        construct: bool,
    ) -> Result<(), JsValue> {
        if !construct {
            return Ok(());
        }

        let other_rotation = other.components.get_rotation()?;
        let rotation = components.get_rotation()?;
        if other_rotation != rotation.next().next() {
            return Ok(());
        }

        let underground_reach = if let Some(reach) = other.dynamic.under_pipe_reach() {
            reach.min(UNDERGROUND_REACH)
        } else {
            return Ok(());
        };

        let d = if let Some(d) = other
            .components
            .position
            .and_then(|opos| self.distance(components, &opos))
        {
            d
        } else {
            return Ok(());
        };

        if d < 1 || underground_reach < d {
            return Ok(());
        }

        let connect_index = rotation.angle_4() as usize;
        let fluid_box = components.get_fluid_box_first()?;

        // If there is already an underground belt with shorter distance, don't connect to the new one.
        if let Some(target) =
            fluid_box.connect_to[connect_index].and_then(|target| others.get(target))
        {
            let target_pos = target.components.get_position()?;
            if let Some(target_d) = self.distance(components, &target_pos) {
                if 0 < target_d && target_d < d {
                    return Ok(());
                }
            }
        }

        if let Some(fluid_box) = components.fluid_boxes.first_mut() {
            fluid_box.connect_to[connect_index] = Some(other_id);
        }

        Ok(())
    }

    fn on_construction_self(
        &mut self,
        _id: StructureId,
        components: &mut StructureComponents,
        others: &StructureDynIter,
        _construct: bool,
    ) -> Result<(), JsValue> {
        let rotation = components.get_rotation()?;
        let connect_index = rotation.angle_4() as usize;

        if let Some((id, _)) =
            others.dyn_iter_id().find(|(_, other)| {
                if other.components.rotation != Some(rotation.next().next()) {
                    return false;
                }

                let underground_reach = if let Some(reach) = other.dynamic.under_pipe_reach() {
                    reach.min(UNDERGROUND_REACH)
                } else {
                    return false;
                };

                let opos = if let Some(pos) = other.components.position {
                    pos
                } else {
                    return false;
                };
                let d = if let Some(d) = self.distance(components, &opos) {
                    d
                } else {
                    return false;
                };

                if d < 1 || underground_reach < d {
                    return false;
                }

                // If there is already an underground belt with shorter distance, don't connect to the new one.
                if let Some(target) = components.fluid_boxes.first().and_then(|fb| {
                    fb.connect_to[connect_index].and_then(|target| others.get(target))
                }) {
                    if let Some(target_d) = target
                        .components
                        .position
                        .and_then(|pos| self.distance(components, &pos))
                    {
                        if 0 < target_d && target_d < d {
                            return false;
                        }
                    }
                }

                true
            })
        {
            if let Some(fb) = components.fluid_boxes.first_mut() {
                fb.connect_to[connect_index] = Some(id);
            }
        }
        Ok(())
    }

    fn fluid_connections(&self, components: &StructureComponents) -> [bool; 4] {
        if let Some(rotation) = components.rotation {
            let mut ret = [false; 4];
            ret[rotation.angle_4() as usize] = true;
            ret
        } else {
            [true; 4]
        }
    }

    fn under_pipe_reach(&self) -> Option<i32> {
        Some(UNDERGROUND_REACH)
    }

    crate::serialize_impl!();
}
