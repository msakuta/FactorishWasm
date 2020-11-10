use super::structure::{ItemResponse, ItemResponseResult, Structure};
use super::{DropItem, FactorishState, Position, Rotation};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

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
                            img,
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
        let moved_x = item.x + self.rotation.delta().0;
        let moved_y = item.y + self.rotation.delta().1;
        Ok((ItemResponse::Move(moved_x, moved_y), None))
    }
}
