use super::{
    pipe::Pipe,
    structure::{Structure, StructureBundle, StructureComponents, StructureDynIter, StructureId},
    water_well::{FluidBox, FluidType},
    FactorishState, FrameProcResult, Position,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

#[derive(Serialize, Deserialize)]
pub(crate) struct OffshorePump;

impl OffshorePump {
    pub(crate) fn new(position: &Position) -> StructureBundle {
        StructureBundle::new(
            Box::new(OffshorePump),
            Some(*position),
            None,
            None,
            None,
            None,
            vec![FluidBox::new(false, true, [None; 4]).set_type(&FluidType::Water)],
        )
    }
}

impl Structure for OffshorePump {
    fn name(&self) -> &str {
        "Offshore Pump"
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
        Pipe::draw_int(self, components, state, context, depth, false)?;
        let position = components
            .position
            .ok_or_else(|| js_str!("Offshore Pump without Position"))?;
        let (x, y) = (position.x as f64 * 32., position.y as f64 * 32.);
        match state.image_offshore_pump.as_ref() {
            Some(img) => {
                context.draw_image_with_image_bitmap(&img.bitmap, x, y)?;
            }
            None => return Err(JsValue::from_str("furnace image not available")),
        }

        Ok(())
    }

    fn frame_proc(
        &mut self,
        _me: StructureId,
        components: &mut StructureComponents,
        _state: &mut FactorishState,
        _structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        let output_fluid_box = components.fluid_boxes.get_mut(0).ok_or_else(|| ())?;
        output_fluid_box.amount =
            (output_fluid_box.amount + 1.).min(output_fluid_box.max_amount);
        Ok(FrameProcResult::None)
    }

    crate::serialize_impl!();
}
