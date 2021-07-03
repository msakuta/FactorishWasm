use super::{
    structure::{DynIterMut, Structure, StructureBoxed, StructureBundle, StructureComponents},
    water_well::FluidBox,
    FactorishState, FrameProcResult, Position, Ref,
};
use serde::{Deserialize, Serialize};
use specs::{Builder, Entity, World, WorldExt};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

#[derive(Serialize, Deserialize)]
pub(crate) struct Pipe {
    fluid_box: FluidBox,
}

impl Pipe {
    pub(crate) fn new(world: &mut World, position: Position) -> Entity {
        world
            .create_entity()
            .with(Box::new(Pipe {
                fluid_box: FluidBox::new(true, true, [false; 4]),
            }) as StructureBoxed)
            .with(position)
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
        match state.image_pipe.as_ref() {
            Some(img) => {
                // let (front, mid) = state.structures.split_at_mut(i);
                // let (center, last) = mid
                //     .split_first_mut()
                //     .ok_or(JsValue::from_str("Structures split fail"))?;

                // We could split and chain like above, but we don't have to, as long as we deal with immutable
                // references.
                // let structures_slice: &[StructureBundle] = state.structures.as_slice();

                let connection_list = structure.connection(entity, state);
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

    fn desc(&self, _entity: Entity, _state: &FactorishState) -> String {
        self.fluid_box.desc()
        // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
        // "Outputs: <br>" +
        // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
    }

    fn frame_proc(
        &mut self,
        components: &mut StructureComponents,
        state: &mut FactorishState,
    ) -> Result<FrameProcResult, ()> {
        // self.fluid_box.connect_to = self.connection(components, state, structures.as_dyn_iter());
        // if let Some(position) = &components.position {
        //     self.fluid_box
        //         .simulate(position, state, &mut structures.dyn_iter_mut());
        // }
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
