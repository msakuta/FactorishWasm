use super::{
    fluid_box::{BufferFluidBox, FluidBox},
    structure::{Structure, StructureBoxed, StructureComponents},
    FactorishState, Position,
};
use serde::{Deserialize, Serialize};
use specs::{Builder, Entity, World, WorldExt};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

#[derive(Serialize, Deserialize)]
pub(crate) struct Pipe;

impl Pipe {
    pub(crate) fn new(world: &mut World, position: Position) -> Entity {
        world
            .create_entity()
            .with(Box::new(Pipe) as StructureBoxed)
            .with(position)
            .with(BufferFluidBox(FluidBox::new(true, true)))
            .build()
    }

    pub(crate) fn draw_int(
        entity: Entity,
        structure: &dyn Structure,
        components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        draw_center: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        let (x, y) = if let Some(position) = components.position.as_ref() {
            (position.x as f64 * 32., position.y as f64 * 32.)
        } else {
            (0., 0.)
        };
        let input_fluid_box = components.input_fluid_box.as_ref();
        let output_fluid_box = components.output_fluid_box.as_ref();
        let buffer_fluid_box = components.buffer_fluid_box.as_ref();
        match state.image_pipe.as_ref() {
            Some(img) => {
                // let (front, mid) = state.structures.split_at_mut(i);
                // let (center, last) = mid
                //     .split_first_mut()
                //     .ok_or(JsValue::from_str("Structures split fail"))?;

                // We could split and chain like above, but we don't have to, as long as we deal with immutable
                // references.
                // let structures_slice: &[StructureBundle] = state.structures.as_slice();

                let mut connection_list = [false; 4]; //structure.connection(entity, state);
                let mut update_fluid_box = |fluid_box: Option<&FluidBox>| {
                    if let Some(fb) = fluid_box {
                        for (i, connect) in fb.connect_to.iter().enumerate() {
                            if connect.is_some() {
                                connection_list[i] = true;
                            }
                        }
                    }
                };
                update_fluid_box(input_fluid_box.map(|i| &i.0));
                update_fluid_box(output_fluid_box.map(|i| &i.0));
                update_fluid_box(buffer_fluid_box.map(|i| &i.0));
                let connections = connection_list
                    .iter()
                    .enumerate()
                    .fold(0, |acc, (i, b)| acc | ((*b as u32) << i));
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

    fn draw(
        &self,
        entity: Entity,
        components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        Self::draw_int(entity, self, components, state, context, depth, true)
    }

    crate::serialize_impl!();
}
