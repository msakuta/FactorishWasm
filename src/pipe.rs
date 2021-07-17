use super::{
    structure::{Structure, StructureDynIter, StructureId},
    water_well::FluidBox,
    FactorishState, FrameProcResult, Position,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

#[derive(Serialize, Deserialize)]
pub(crate) struct Pipe {
    position: Position,
    fluid_box: FluidBox,
}

impl Pipe {
    pub(crate) fn new(position: &Position) -> Self {
        Pipe {
            position: *position,
            fluid_box: FluidBox::new(true, true, [None; 4]),
        }
    }

    pub(crate) fn draw_int(
        structure: &dyn Structure,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        draw_center: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        let position = structure.position();
        let (x, y) = (position.x as f64 * 32., position.y as f64 * 32.);
        match state.image_pipe.as_ref() {
            Some(img) => {
                let connections = structure
                    .fluid_box()
                    .map(|fluid_boxes| {
                        Some(
                            fluid_boxes
                                .first()?
                                .connect_to
                                .iter()
                                .enumerate()
                                .filter(|(_, b)| b.is_some())
                                .fold(0, |acc, (i, _)| acc | (1 << i)),
                        )
                    })
                    .flatten()
                    .unwrap_or(0);
                // Skip drawing center dot? if there are no connections
                if !draw_center && connections == 0 {
                    return Ok(());
                }
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
            None => return Err(JsValue::from_str("pipe image not available")),
        }

        Ok(())
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
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        Self::draw_int(self, state, context, depth, true)
    }

    fn desc(&self, _state: &FactorishState) -> String {
        self.fluid_box.desc()
        // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
        // "Outputs: <br>" +
        // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
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

    fn fluid_box(&self) -> Option<Vec<&FluidBox>> {
        Some(vec![&self.fluid_box])
    }

    fn fluid_box_mut(&mut self) -> Option<Vec<&mut FluidBox>> {
        Some(vec![&mut self.fluid_box])
    }

    crate::serialize_impl!();
}
