use super::{
    pipe::Pipe,
    structure::{Structure, StructureDynIter},
    water_well::{FluidBox, FluidType},
    FactorishState, FrameProcResult, Position,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

#[derive(Serialize, Deserialize)]
pub(crate) struct OffshorePump {
    position: Position,
    output_fluid_box: FluidBox,
}

impl OffshorePump {
    pub(crate) fn new(position: &Position) -> Self {
        OffshorePump {
            position: *position,
            output_fluid_box: FluidBox::new(false, true, [None; 4]).set_type(&FluidType::Water),
        }
    }
}

impl Structure for OffshorePump {
    fn name(&self) -> &str {
        "Offshore Pump"
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
        if depth != 0 {
            return Ok(());
        };
        Pipe::draw_int(self, state, context, depth, false)?;
        let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
        match state.image_offshore_pump.as_ref() {
            Some(img) => {
                context.draw_image_with_image_bitmap(&img.bitmap, x, y)?;
            }
            None => return Err(JsValue::from_str("furnace image not available")),
        }

        Ok(())
    }

    fn desc(&self, _state: &FactorishState) -> String {
        format!(
            "{}<br>{}",
            self.output_fluid_box.desc(),
            "Outputs: Water<br>",
        )
    }

    fn frame_proc(
        &mut self,
        _state: &mut FactorishState,
        structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        self.output_fluid_box.amount =
            (self.output_fluid_box.amount + 1.).min(self.output_fluid_box.max_amount);
        self.output_fluid_box.simulate(structures);
        Ok(FrameProcResult::None)
    }

    fn fluid_box(&self) -> Option<Vec<&FluidBox>> {
        Some(vec![&self.output_fluid_box])
    }

    fn fluid_box_mut(&mut self) -> Option<Vec<&mut FluidBox>> {
        Some(vec![&mut self.output_fluid_box])
    }

    crate::serialize_impl!();
}
