use super::{
    drop_items::DROP_ITEM_SIZE_I,
    gl::utils::{enable_buffer, Flatten},
    inventory::InventoryTrait,
    items::ItemType,
    structure::{
        ItemResponse, ItemResponseResult, Structure, StructureBundle, StructureComponents,
        StructureDynIter, StructureId,
    },
    transport_belt::TransportBelt,
    window, DropItem, FactorishState, FrameProcResult, Inventory, Position, RotateErr, Rotation,
    TILE_SIZE, TILE_SIZE_I,
};
use cgmath::{Deg, Matrix3, Matrix4, Rad, Vector2, Vector3};
use rotate_enum::RotateEnum;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, WebGlRenderingContext as GL};

const UNDERGROUND_REACH: i32 = 4;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, RotateEnum)]
pub(crate) enum UnderDirection {
    ToGround,
    ToSurface,
}

use UnderDirection::*;

#[derive(Serialize, Deserialize)]
pub(crate) struct UndergroundBelt {
    direction: UnderDirection,
    target: Option<StructureId>,

    /// Items in the underground belt. First value is the absolute position in the underground belt
    /// from the entrance.
    items: VecDeque<(i32, ItemType)>,
}

impl UndergroundBelt {
    pub(crate) fn new(
        position: Position,
        rotation: Rotation,
        direction: UnderDirection,
    ) -> StructureBundle {
        StructureBundle {
            dynamic: Box::new(Self {
                direction,
                target: None,
                items: VecDeque::new(),
            }),
            components: StructureComponents::new_with_position_and_rotation(position, rotation),
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
            Rotation::Left => -dx,
            Rotation::Right => dx,
            Rotation::Top => -dy,
            Rotation::Bottom => dy,
        })
    }
}

impl Structure for UndergroundBelt {
    fn name(&self) -> &str {
        "Underground Belt"
    }

    fn under_direction(&self) -> Option<self::UnderDirection> {
        Some(self.direction)
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
        let position = components
            .position
            .ok_or_else(|| js_str!("Underground belt without Position"))?;
        let rotation = components
            .rotation
            .ok_or_else(|| js_str!("Underground belt without Rotation"))?;
        match state.image_underground_belt.as_ref() {
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
                    match self.direction {
                        ToGround => 0.,
                        ToSurface => 64.,
                    } + depth as f64 * 128.,
                    TILE_SIZE,
                    TILE_SIZE * 2.,
                    position.x as f64 * TILE_SIZE,
                    position.y as f64 * TILE_SIZE - TILE_SIZE,
                    TILE_SIZE,
                    TILE_SIZE * 2.,
                )?;
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
            0 | 1 => {
                let shader = state
                    .assets
                    .textured_shader
                    .as_ref()
                    .ok_or_else(|| js_str!("Shader not found"))?;
                gl.use_program(Some(&shader.program));
                gl.uniform1f(shader.alpha_loc.as_ref(), if is_ghost { 0.5 } else { 1. });
                gl.active_texture(GL::TEXTURE0);
                gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_underground_belt));
                let sx = ((rotation.angle_4() + 2) % 4) as f32;
                gl.uniform_matrix3fv_with_f32_array(
                    shader.tex_transform_loc.as_ref(),
                    false,
                    (Matrix3::from_nonuniform_scale(1. / 4., 1. / 4.)
                        * Matrix3::from_translation(Vector2::new(
                            sx,
                            match self.direction {
                                ToGround => 0.,
                                ToSurface => 1.,
                            } + depth as f32 * 2.,
                        )))
                    .flatten(),
                );

                enable_buffer(&gl, &state.assets.screen_buffer, 2, shader.vertex_position);
                gl.uniform_matrix4fv_with_f32_array(
                    shader.transform_loc.as_ref(),
                    false,
                    (state.get_world_transform()?
                        * Matrix4::from_scale(2.)
                        * Matrix4::from_translation(Vector3::new(x, y - 1., 0.))
                        * Matrix4::from_nonuniform_scale(1., 2., 1.))
                    .flatten(),
                );
                gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
            }
            2 => {
                let on_cursor = state.cursor == Some([position.x, position.y]);
                if state.alt_mode && self.direction == UnderDirection::ToGround || on_cursor {
                    if let Some(dist) = self
                        .target
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

                        let scale = (dist + 1) as f32;
                        let (scale_x, scale_y) = if rotation.is_horizontal() {
                            (scale, 1.)
                        } else {
                            (1., scale)
                        };
                        let x = if rotation == Rotation::Left {
                            x - dist as f32
                        } else {
                            x
                        };
                        let y = if rotation == Rotation::Top {
                            y - dist as f32
                        } else {
                            y
                        };

                        let mut arrow_rotation = rotation;
                        if self.direction == UnderDirection::ToGround {
                            arrow_rotation = arrow_rotation.next().next();
                        }

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

                        gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_sparse_direction));

                        gl.uniform_matrix3fv_with_f32_array(
                            shader.tex_transform_loc.as_ref(),
                            false,
                            (Matrix3::from_nonuniform_scale(scale * 2., 1.)
                                * Matrix3::from_angle_z(Rad(-arrow_rotation.angle_rad() as f32)))
                            .flatten(),
                        );

                        let (x, y, scale_x, scale_y) = if rotation.is_horizontal() {
                            (x, y + 0.25, scale, 0.5)
                        } else {
                            (x + 0.25, y, 0.5, scale)
                        };

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
        if self.direction == ToSurface {
            return Ok(FrameProcResult::None);
        }
        if let Some((target, distance)) =
            self.target
                .and_then(|id| structures.get(id))
                .and_then(|target| {
                    // If the direction of the other side of the underground belt does not align with us
                    // (ToGround vs. ToGround), we don't want to run the underground belt
                    // which will send items back and forth like ping-pong.
                    // Note that we don't have to worry about ToSurface vs. ToSurface because ToSurface will
                    // not run this branch, but this logic will also disable motion if we did.
                    if target
                        .dynamic
                        .under_direction()
                        .map(|d| d == self.direction)
                        .unwrap_or(true)
                    {
                        None
                    } else {
                        let tpos = target.components.position?;
                        Some((tpos, self.distance(components, &tpos)?))
                    }
                })
        {
            // Because we have ordered queue, we only need to remember the last index and pop out the rest.
            let mut delete_index = None;
            for i in 0..self.items.len() {
                let next_pos = if i + 1 < self.items.len() {
                    self.items[i + 1].0
                } else {
                    (distance + 1) * TILE_SIZE_I
                };
                let item = &mut self.items[i];
                if distance * TILE_SIZE_I < item.0 {
                    delete_index = Some(i);
                    break;
                } else if item.0 + DROP_ITEM_SIZE_I < next_pos {
                    item.0 += 1;
                }
            }
            if let Some(delete_index) = delete_index {
                for i in (delete_index..self.items.len()).rev() {
                    if let Ok(()) = state.new_object(&target, self.items[i].1) {
                        self.items.pop_back().ok_or(())?;
                    }
                }
            }
        }
        Ok(FrameProcResult::None)
    }

    fn movable(&self) -> bool {
        true
    }

    fn rotate(
        &mut self,
        _components: &mut StructureComponents,
        state: &mut FactorishState,
        _others: &StructureDynIter,
    ) -> Result<(), RotateErr> {
        self.direction = self.direction.next();
        if self.direction == ToSurface {
            state.player.inventory.merge(self.destroy_inventory());
            state
                .on_player_update
                .call1(
                    &window(),
                    &JsValue::from(state.get_player_inventory().map_err(RotateErr::Other)?),
                )
                .unwrap_or_else(|_| JsValue::from(true));
        }
        Ok(())
    }

    fn set_rotation(
        &mut self,
        components: &mut StructureComponents,
        rotation: &Rotation,
    ) -> Result<(), ()> {
        if let Some(ref mut my_rotation) = components.rotation {
            *my_rotation = *rotation;
        }
        Ok(())
    }

    fn item_response(
        &mut self,
        components: &mut StructureComponents,
        item: &DropItem,
    ) -> Result<ItemResponseResult, JsValue> {
        if self.direction == ToGround {
            if self.target.is_some() {
                if let Some(first_item) = self.items.front() {
                    // Do not insert if the underground buffer is full
                    if first_item.0 < DROP_ITEM_SIZE_I {
                        return Err(js_str!("Item stuck"));
                    }
                }
                self.items.push_front((0, item.type_));
                Ok((ItemResponse::Consume, None))
            } else {
                Err(js_str!("Underground belt not connected"))
            }
        } else {
            TransportBelt::transport_item(
                components
                    .rotation
                    .ok_or_else(|| js_str!("Transport belt without rotation"))?
                    .next()
                    .next(),
                item,
            )
        }
    }

    fn can_input(&self, _components: &StructureComponents, _item_type: &ItemType) -> bool {
        self.direction == ToGround
    }

    fn can_output(
        &self,
        components: &StructureComponents,
        structures: &StructureDynIter,
    ) -> Inventory {
        if self.direction == ToSurface {
            if let Some(distance) = self
                .target
                .and_then(|id| structures.get(id))
                .and_then(|target| target.components.position)
                .and_then(|tpos| self.distance(components, &tpos))
            {
                return self
                    .items
                    .iter()
                    .filter_map(|item| {
                        if distance * TILE_SIZE_I < item.0 {
                            Some((item.1, 1))
                        } else {
                            None
                        }
                    })
                    .collect();
            }
        }
        Inventory::new()
    }

    fn desc(&self, _components: &StructureComponents, _state: &FactorishState) -> String {
        format!("Connection: {:?}<br>Items: {:?}", self.target, self.items)
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
            // This resetting is not strictly necessary with generational id
            if self.target == Some(other_id) {
                self.target = None;
            }
            return Ok(());
        }
        let rotation = components
            .rotation
            .ok_or_else(|| js_str!("UndergroundBelt without Rotation"))?;
        if other.dynamic.name() != self.name()
            || other.components.rotation != Some(rotation.next().next())
        {
            return Ok(());
        }
        let opos = other
            .components
            .position
            .ok_or_else(|| js_str!("UndergroundBelt target without Position"))?;
        let d = if let Some(d) = self.distance(components, &opos) {
            d
        } else {
            return Ok(());
        };

        if d < 1 || UNDERGROUND_REACH < d {
            return Ok(());
        }

        // If there is already an underground belt with shorter distance, don't connect to the new one.
        if let Some(target) = self.target.and_then(|target| others.get(target)) {
            if let Some(target_d) = target
                .components
                .position
                .and_then(|tpos| self.distance(components, &tpos))
            {
                if target_d < d {
                    return Ok(());
                }
            }
        }

        self.target = Some(other_id);

        Ok(())
    }

    fn on_construction_self(
        &mut self,
        _id: StructureId,
        components: &mut StructureComponents,
        others: &StructureDynIter,
        _construct: bool,
    ) -> Result<(), JsValue> {
        let rotation = components
            .rotation
            .ok_or_else(|| js_str!("UndergroundBelt without Rotation"))?;
        if let Some((id, _)) = others.dyn_iter_id().find(|(_, other)| {
            if other.dynamic.name() != self.name()
                || other.components.rotation != Some(rotation.next().next())
            {
                return false;
            }

            let opos = if let Some(opos) = other.components.position {
                opos
            } else {
                return false;
            };
            let d = if let Some(d) = self.distance(components, &opos) {
                d
            } else {
                return false;
            };

            if d < 1 || UNDERGROUND_REACH < d {
                return false;
            }

            // If there is already an underground belt with shorter distance, don't connect to the new one.
            if let Some(target) = self.target.and_then(|target| others.get(target)) {
                if let Some(target_d) = target
                    .components
                    .position
                    .and_then(|pos| self.distance(components, &pos))
                {
                    if target_d < d {
                        return false;
                    }
                }
            }

            true
        }) {
            self.target = Some(id);
        }
        Ok(())
    }

    fn destroy_inventory(&mut self) -> Inventory {
        let mut ret = Inventory::new();
        for (_, item) in std::mem::take(&mut self.items) {
            *ret.entry(item).or_default() += 1;
        }
        ret
    }

    crate::serialize_impl!();
}
