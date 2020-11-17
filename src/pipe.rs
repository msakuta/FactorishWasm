use super::structure::{DynIterMut, Structure};
use super::water_well::FluidBox;
use super::{FactorishState, FrameProcResult, Position, Ref};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

pub(crate) struct Pipe {
    position: Position,
    fluid_box: FluidBox,
}

impl Pipe {
    pub(crate) fn new(position: &Position) -> Self {
        Pipe {
            position: *position,
            fluid_box: FluidBox::new(true, true, [false; 4]),
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

                // We could split and chain like above, but we don't have to, as long as we deal with immutable
                // references.
                let structures_slice: &[Box<dyn Structure>] = state.structures.as_slice();

                let connections = self.connection(state, &mut Ref(structures_slice));
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
        self.fluid_box.desc()
        // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
        // "Outputs: <br>" +
        // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
    }

    fn frame_proc(
        &mut self,
        state: &mut FactorishState,
        structures: &mut dyn DynIterMut<Item = Box<dyn Structure>>,
    ) -> Result<FrameProcResult, ()> {
        let connections = self.connection(state, structures.as_dyn_iter());
        self.fluid_box.connect_to = [
            connections & 1 != 0,
            connections & 2 != 0,
            connections & 4 != 0,
            connections & 8 != 0,
        ];
        self.fluid_box
            .simulate(&self.position, state, &mut structures.dyn_iter_mut());
        Ok(FrameProcResult::None)
    }

    fn fluid_box(&self) -> Option<&FluidBox> {
        Some(&self.fluid_box)
    }

    fn fluid_box_mut(&mut self) -> Option<&mut FluidBox> {
        Some(&mut self.fluid_box)
    }
}
