use super::{
    structure::{
        ItemResponse, ItemResponseResult, Structure, StructureBundle, StructureComponents, StructureDynIter
    },
    DropItem, FactorishState, Position, Rotation, TILE_SIZE,
    RotateErr
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

#[derive(Serialize, Deserialize)]
pub(crate) struct TransportBelt {}

impl TransportBelt {
    pub(crate) fn new(position: Position, rotation: Rotation) -> StructureBundle {
        StructureBundle {
            dynamic: Box::new(TransportBelt {}),
            components: StructureComponents::new_with_position_and_rotation(position, rotation),
        }
    }
}

impl Structure for TransportBelt {
    fn name(&self) -> &str {
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

    fn movable(&self) -> bool {
        true
    }

    fn rotate(&mut self, components: &mut StructureComponents, _others: &StructureDynIter) -> Result<(), RotateErr> {
        if let Some(ref mut rotation) = components.rotation {
            *rotation = rotation.next();
            Ok(())
        } else {
            Err(RotateErr::NotSupported)
        }
    }

    fn set_rotation(&mut self, components: &mut StructureComponents, rotation: &Rotation) -> Result<(), ()> {
        if let Some(ref mut self_rotation) = components.rotation {
            *self_rotation = *rotation;
        }
        Ok(())
    }

    fn item_response(&mut self, components: &mut StructureComponents,item: &DropItem) -> Result<ItemResponseResult, JsValue> {
        let rotation = components
            .rotation
            .as_ref()
            .ok_or_else(|| js_str!("TransportBelt without Rotation component"))?;
        let vx = rotation.delta().0;
        let vy = rotation.delta().1;
        let ax = if rotation.is_vertial() {
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

    crate::serialize_impl!();
}
