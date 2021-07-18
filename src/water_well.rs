use super::{
    pipe::Pipe,
    structure::{Structure, StructureDynIter, StructureBundle, StructureId, StructureComponents},
    FactorishState, FrameProcResult, Position,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

use std::cmp::Eq;

#[derive(Eq, PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) enum FluidType {
    Water,
    Steam,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct FluidBox {
    pub type_: Option<FluidType>,
    pub amount: f64,
    pub max_amount: f64,
    pub input_enable: bool,
    pub output_enable: bool,
    #[serde(skip)]
    pub connect_to: [Option<StructureId>; 4],
    pub filter: Option<FluidType>, // permits undefined
}

impl FluidBox {
    pub(crate) fn new(
        input_enable: bool,
        output_enable: bool,
        connect_to: [Option<StructureId>; 4],
    ) -> Self {
        Self {
            type_: None,
            amount: 0.,
            max_amount: 100.,
            input_enable,
            output_enable,
            connect_to,
            filter: None,
        }
    }

    pub(crate) fn set_type(mut self, type_: &FluidType) -> Self {
        self.type_ = Some(*type_);
        self
    }

    pub(crate) fn desc(&self) -> String {
        let amount_ratio = self.amount / self.max_amount * 100.;
        // Progress bar
        format!("{}{}{}{:?}",
            format!("{}: {:.0}%<br>", self.type_.map(|v| format!("{:?}", v)).unwrap_or_else(|| "None".to_string()), amount_ratio),
            "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
            format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                amount_ratio),
            self.connect_to,
            )
    }

    pub(crate) fn simulate(
        &mut self,
        position: &Position,
        state: &mut FactorishState,
        structures: &mut StructureDynIter,
    ) {
        let mut _biggest_flow_idx = -1;
        let mut biggest_flow_amount = 1e-3; // At least this amount of flow is required for displaying flow direction
                                            // In an unlikely event, a fluid box without either input or output ports has nothing to do
        if self.amount == 0. || !self.input_enable && !self.output_enable {
            return;
        }
        let connect_list = self
            .connect_to
            .iter()
            .enumerate()
            .filter_map(|(i, c)| Some((i, (*c)?)));
        for (i, id) in connect_list {
            if let Some(bundle) = structures.get_mut(id) {
                for fluid_box in &mut bundle.components.fluid_boxes {
                    // Different types of fluids won't mix
                    if 0. < fluid_box.amount
                        && 0. < self.amount
                        && fluid_box.type_ != self.type_
                        && fluid_box.type_.is_some()
                    {
                        continue;
                    }
                    let pressure = fluid_box.amount - self.amount;
                    if 0. < pressure {
                        continue;
                    }
                    let flow = pressure * 0.1;
                    // Check input/output valve state
                    if if flow < 0. {
                        !self.output_enable
                            || !fluid_box.input_enable
                            || fluid_box.filter.is_some() && fluid_box.filter != self.type_
                    } else {
                        !self.input_enable
                            || !fluid_box.output_enable
                            || self.filter.is_some() && self.filter != fluid_box.type_
                    } {
                        continue;
                    }
                    fluid_box.amount -= flow;
                    self.amount += flow;
                    if flow < 0. {
                        fluid_box.type_ = self.type_;
                    } else {
                        self.type_ = fluid_box.type_;
                    }
                    if biggest_flow_amount < flow.abs() {
                        biggest_flow_amount = flow;
                        _biggest_flow_idx = i as isize;
                    }
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct WaterWell;

impl WaterWell {
    pub(crate) fn new(position: Position) -> StructureBundle {
        StructureBundle::new(
            Box::new(WaterWell),
            Some(position),
            None,
            None,
            None,
            None,
            vec![FluidBox::new(false, true, [None; 4]).set_type(&FluidType::Water)],
        )
    }
}

impl Structure for WaterWell {
    fn name(&self) -> &str {
        "Water Well"
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

    fn desc(&self, components: &StructureComponents, _state: &FactorishState) -> String {
        format!(
            "{}<br>{}",
            components
                .fluid_boxes
                .first()
                .map(|fb| fb.desc())
                .unwrap_or("".to_string()),
            "Outputs: Water<br>",
        )
    }

    fn frame_proc(
        &mut self,
        _me: StructureId,
        components: &mut StructureComponents,
        state: &mut FactorishState,
        structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        assert!(components.fluid_boxes.len() > 0);
        let output_fluid_box = &mut components.fluid_boxes[0];
        output_fluid_box.amount = (output_fluid_box.amount + 1.).min(output_fluid_box.max_amount);
        Ok(FrameProcResult::None)
    }

    crate::serialize_impl!();
}
