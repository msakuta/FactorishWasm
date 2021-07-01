use super::structure::{
    BoundingBox, ItemResponse, ItemResponseResult, Size, Structure, StructureBundle,
    StructureComponents,
};
use super::{DropItem, FactorishState, Position, Rotation, TILE_SIZE};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;
use specs::{World, WorldExt, Entity, Builder};

#[derive(Serialize, Deserialize)]
pub(crate) struct Splitter {
    direction: i8,
}

impl Splitter {
    pub(crate) fn new(world: &World, position: Position, rotation: Rotation) -> Entity {
        world.create_entity()
            .with(Box::new(Splitter { direction: 0 }) as Box<dyn Structure + Send + Sync>)
            .with(position)
            .with(rotation)
            .build()
    }
}

impl Structure for Splitter {
    fn name(&self) -> &str {
        "Splitter"
    }

    fn size(&self) -> Size {
        Size {
            width: 1,
            height: 2,
        }
    }

    fn bounding_box(&self, components: &StructureComponents) -> Option<BoundingBox> {
        let (position, rotation) = if let StructureComponents {
            position: Some(position),
            rotation: Some(rotation),
            ..
        } = components
        {
            (position, rotation)
        } else {
            return None;
        };
        Some(match *rotation {
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
        })
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
        }
        let mut ret = Ok(());
        let (x, y) = if let Some(position) = &components.position {
            (position.x as f64 * 32., position.y as f64 * 32.)
        } else {
            (0., 0.)
        };
        context.save();
        context.translate(x + 16., y + 16.)?;
        context.rotate(components.rotation.map(|r| r.angle_rad()).unwrap_or(0.))?;
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
                                x,
                                y + n as f64 * TILE_SIZE,
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
                    for ix in 0..2 {
                        context
                            .draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                                &splitter.bitmap,
                                0.,
                                (if self.direction == 0 { 1 - ix } else { ix }) as f64 * TILE_SIZE,
                                TILE_SIZE,
                                TILE_SIZE,
                                x,
                                y + ix as f64 * TILE_SIZE,
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

    fn movable(&self) -> bool {
        true
    }

    fn rotate(&mut self, components: &mut StructureComponents) -> Result<(), ()> {
        let rotation = components.rotation.as_mut().ok_or(())?;
        let position = components.position.as_mut().ok_or(())?;
        *position = position.add(rotation.next().delta());
        *rotation = rotation.next().next();
        Ok(())
    }

    fn item_response(
        &mut self,
        components: &mut StructureComponents,
        item: &DropItem,
    ) -> Result<ItemResponseResult, JsValue> {
        let rotation = components
            .rotation
            .as_ref()
            .ok_or_else(|| js_str!("Splitter without Rotation component"))?;
        let vx = rotation.delta().0;
        let vy = rotation.delta().1;
        let mut ax = if rotation.is_vertial() {
            (item.x as f64 / TILE_SIZE).floor() * TILE_SIZE + TILE_SIZE / 2.
        } else {
            item.x as f64
        };
        let mut ay = if rotation.is_horizontal() {
            (item.y as f64 / TILE_SIZE).floor() * TILE_SIZE + TILE_SIZE / 2.
        } else {
            item.y as f64
        };
        let Position { x: tx, y: ty } = components
            .position
            .ok_or_else(|| js_str!("Splitter without Position component"))?;
        let halftilesize = TILE_SIZE / 2.;
        let mut postdirection = false;
        let shift_direction = rotation.clone().next().delta();
        if rotation.is_horizontal() {
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

        let moved_x = ax as i32 + vx;
        let moved_y = ay as i32 + vy;
        Ok((ItemResponse::Move(moved_x, moved_y), None))
    }

    crate::serialize_impl!();
}
