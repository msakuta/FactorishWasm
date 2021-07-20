use super::{
    structure::{Position, Structure, StructureBundle, StructureComponents},
    water_well::FluidBox,
    FactorishState,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

#[derive(Serialize, Deserialize)]
pub(crate) struct Pipe;

impl Pipe {
    pub(crate) fn new(position: Position) -> StructureBundle {
        StructureBundle::new(
            Box::new(Pipe),
            Some(position),
            None,
            None,
            None,
            None,
            vec![FluidBox::new(true, true)],
        )
    }

    pub(crate) fn draw_int(
        _structure: &dyn Structure,
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
                let connections = components
                    .fluid_boxes
                    .first()
                    .map(|fluid_box| {
                        fluid_box
                            .connect_to
                            .iter()
                            .enumerate()
                            .filter(|(_, b)| b.is_some())
                            .fold(0, |acc, (i, _)| acc | (1 << i))
                    })
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

    fn draw(
        &self,
        components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        Self::draw_int(self, components, state, context, depth, true)
    }

    fn desc(&self, components: &StructureComponents, _state: &FactorishState) -> String {
        components
            .fluid_boxes
            .iter()
            .map(|p| p.desc())
            .fold("".to_string(), |acc, s| acc + &s)
        // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
        // "Outputs: <br>" +
        // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
    }

    crate::serialize_impl!();
}
