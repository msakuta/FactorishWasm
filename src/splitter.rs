use super::structure::{BoundingBox, ItemResponse, ItemResponseResult, Size, Structure};
use super::{DropItem, FactorishState, Position, Rotation, TILE_SIZE};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

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
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        match state.image_belt.as_ref().zip(state.image_splitter.as_ref()) {
            Some((belt, splitter)) => {
                let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
                context.save();
                context.translate(x + 16., y + 16.)?;
                context.rotate(self.rotation.angle_rad())?;
                context.translate(-(x + 16.), -(y + 16.))?;
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
                context.draw_image_with_image_bitmap(
                    &splitter.bitmap,
                    self.position.x as f64 * TILE_SIZE,
                    self.position.y as f64 * TILE_SIZE,
                )?;
                context.restore();
            }
            None => return Err(JsValue::from_str("belt image not available")),
        }

        Ok(())
    }

    fn movable(&self) -> bool {
        true
    }

    fn rotate(&mut self) -> Result<(), ()> {
        self.rotation.next();
        Ok(())
    }

    fn set_rotation(&mut self, rotation: &Rotation) -> Result<(), ()> {
        self.rotation = *rotation;
        Ok(())
    }

    fn item_response(&mut self, item: &DropItem) -> Result<ItemResponseResult, ()> {
        let vx = self.rotation.delta().0;
        let vy = self.rotation.delta().1;
        let mut ax = if self.rotation.is_vertial() {
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
        if self.rotation.is_horizontal() {
            // Detect the point where the item passes over the mid point of this entity.
            if ((ax + halftilesize) / TILE_SIZE).floor()
                != ((ax + vx as f64 + halftilesize) / TILE_SIZE).floor()
            {
                ay = (ty + self.direction as i32) as f64 * TILE_SIZE + TILE_SIZE / 2.;
                postdirection = true; // Signal to switch direction
            }
        } else if ((ay + halftilesize) / TILE_SIZE).floor()
            != ((ay + vy as f64 + halftilesize) / TILE_SIZE).floor()
        {
            ax = (tx + self.direction as i32) as f64 * TILE_SIZE + TILE_SIZE / 2.;
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
