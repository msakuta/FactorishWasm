use super::{
    structure::{ItemResponse, ItemResponseResult, Structure, StructureComponents},
    DropItem, FactorishState, Position, Rotation, TILE_SIZE,
};
use serde::{Deserialize, Serialize};
use specs::{Builder, Entity, World, WorldExt};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

#[derive(Serialize, Deserialize)]
pub(crate) struct TransportBelt {}

impl TransportBelt {
    pub(crate) fn new(world: &mut World, position: Position, rotation: Rotation) -> Entity {
        world
            .create_entity()
            .with(Box::new(TransportBelt {}) as Box<dyn Structure + Send + Sync>)
            .with(position)
            .with(rotation)
            .with(crate::structure::Movable)
            .build()
    }

    pub(crate) fn draw_static(
        x: f64,
        y: f64,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        rotation: &Rotation,
    ) -> Result<(), JsValue> {
        match state.image_belt.as_ref() {
            Some(img) => {
                context.save();
                context.translate(x + 16., y + 16.)?;
                context.rotate(rotation.angle_rad())?;
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
}

impl Structure for TransportBelt {
    fn name(&self) -> &str {
        "Transport Belt"
    }

    fn draw(
        &self,
        _entity: Entity,
        components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        let (x, y) = if let Some(position) = components.position.as_ref() {
            (position.x as f64 * 32., position.y as f64 * 32.)
        } else {
            (0., 0.)
        };
        TransportBelt::draw_static(
            x,
            y,
            state,
            context,
            &components.rotation.unwrap_or(Rotation::Left),
        )
    }

    fn item_response(
        &mut self,
        entity: Entity,
        state: &FactorishState,
        item: &DropItem,
    ) -> Result<ItemResponseResult, JsValue> {
        let rotation = state
            .world
            .read_component::<Rotation>()
            .get(entity)
            .copied()
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
