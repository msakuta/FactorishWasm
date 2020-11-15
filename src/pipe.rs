use super::items::item_to_str;
use super::structure::{DynIterMut, Structure};
use super::water_well::FluidBox;
use super::{
    log, DropItem, FactorishState, FrameProcResult, Inventory, InventoryTrait, ItemType, Position,
    Recipe, Rotation, COAL_POWER,
};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

use std::collections::HashMap;

pub(crate) struct Pipe {
    position: Position,
    fluid_box: FluidBox,
}

impl Pipe {
    pub(crate) fn new(position: &Position) -> Self {
        Pipe {
            position: *position,
            fluid_box: FluidBox::new(true, false, [false; 4]),
        }
    }
}

impl Structure for Pipe {
    fn name(&self) -> &str {
        "Pipe"
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
        let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
        match state.image_pipe.as_ref() {
            Some(img) => {
                // let (front, mid) = state.structures.split_at_mut(i);
                // let (center, last) = mid
                //     .split_first_mut()
                //     .ok_or(JsValue::from_str("Structures split fail"))?;
                let connections = self.connection(state, &mut state.structures.iter());
                let sx = (connections % 4 * 32) as f64;
                let sy = ((connections / 4) * 32) as f64;
                context.draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    &img.bitmap,
                    sx,
                    sy,
                    32.,
                    32.,
                    x,
                    y,
                    32.,
                    32.,
                )?;
            }
            None => return Err(JsValue::from_str("furnace image not available")),
        }

        Ok(())
    }

    fn desc(&self, _state: &FactorishState) -> String {
        let amount_ratio = self.fluid_box.amount / self.fluid_box.max_amount * 100.;
        // Progress bar
        format!("{}{}{}",
            format!("Water: {:.0}%<br>", amount_ratio),
            "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
            format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                amount_ratio),
            )
        // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
        // "Outputs: <br>" +
        // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
    }

    fn frame_proc(
        &mut self,
        _state: &mut FactorishState,
        _structures: &mut dyn DynIterMut<Item = Box<dyn Structure>>,
    ) -> Result<FrameProcResult, ()> {
        Ok(FrameProcResult::None)
    }

    fn fluid_box(&self) -> Option<&FluidBox> {
        Some(&self.fluid_box)
    }

    fn fluid_box_mut(&mut self) -> Option<&mut FluidBox> {
        Some(&mut self.fluid_box)
    }
}
