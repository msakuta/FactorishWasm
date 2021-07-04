use super::{
    fluid_box::{FluidBox, FluidType, OutputFluidBox},
    pipe::Pipe,
    structure::{DynIterMut, Structure, StructureBoxed, StructureBundle, StructureComponents},
    FactorishState, FrameProcResult, Position,
};
use serde::{Deserialize, Serialize};
use specs::{Builder, Entity, World, WorldExt};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

#[derive(Serialize, Deserialize)]
pub(crate) struct WaterWell;

impl WaterWell {
    pub(crate) fn new(world: &mut World, position: Position) -> Entity {
        world
            .create_entity()
            .with(Box::new(Self) as StructureBoxed)
            .with(position)
            .with(OutputFluidBox(FluidBox::new(false, true)))
            .build()
    }
}

impl Structure for WaterWell {
    fn name(&self) -> &str {
        "Water Well"
    }

    fn draw(
        &self,
        entity: Entity,
        components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        Pipe::draw_int(entity, self, components, state, context, depth, false)?;
        let (x, y) = if let Some(position) = components.position {
            (position.x as f64 * 32., position.y as f64 * 32.)
        } else {
            (0., 0.)
        };
        match state.image_water_well.as_ref() {
            Some(img) => {
                context.draw_image_with_image_bitmap(&img.bitmap, x, y)?;
            }
            None => return Err(JsValue::from_str("furnace image not available")),
        }

        Ok(())
    }

    fn desc(&self, _entity: Entity, _state: &FactorishState) -> String {
        format!(
            "<br>{}",
            // self.output_fluid_box.desc(),
            "Outputs: Water<br>",
        )
    }

    fn frame_proc(
        &mut self,
        _entity: Entity,
        components: &mut StructureComponents,
        state: &mut FactorishState,
    ) -> Result<FrameProcResult, ()> {
        if let Some(ofb) = &mut components.output_fluid_box {
            ofb.0.amount = (ofb.0.amount + 1.).min(ofb.0.max_amount);
        }
        let connections = [false; 4]; //self.connection(components, state, structures.as_dyn_iter());
                                      // self.output_fluid_box.connect_to = connections;
                                      // if let Some(position) = components.position.as_ref() {
                                      //     self.output_fluid_box
                                      //         .simulate(position, state, &mut structures.dyn_iter_mut());
                                      // }
        Ok(FrameProcResult::None)
    }

    crate::serialize_impl!();
}
