use super::{
    burner::Burner,
    pipe::Pipe,
    structure::{DynIterMut, Structure, StructureBundle},
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
    pub connect_to: [bool; 4],
    pub filter: Option<FluidType>, // permits undefined
}

impl FluidBox {
    pub(crate) fn new(input_enable: bool, output_enable: bool, connect_to: [bool; 4]) -> Self {
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

    fn set_type(mut self, type_: &FluidType) -> Self {
        self.type_ = Some(*type_);
        self
    }

    pub(crate) fn desc(&self) -> String {
        let amount_ratio = self.amount / self.max_amount * 100.;
        // Progress bar
        format!("{}{}{}",
            format!("{}: {:.0}%<br>", self.type_.map(|v| format!("{:?}", v)).unwrap_or_else(|| "None".to_string()), amount_ratio),
            "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
            format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                amount_ratio),
            )
    }

    pub(crate) fn simulate(
        &mut self,
        position: &Position,
        state: &mut FactorishState,
        structures: &mut dyn Iterator<Item = &mut StructureBundle>,
    ) {
        let mut _biggest_flow_idx = -1;
        let mut biggest_flow_amount = 1e-3; // At least this amount of flow is required for displaying flow direction
                                            // In an unlikely event, a fluid box without either input or output ports has nothing to do
        if self.amount == 0. || !self.input_enable && !self.output_enable {
            return;
        }
        let rel_dir = [[-1, 0], [0, -1], [1, 0], [0, 1]];
        let connect_list = self
            .connect_to
            .iter()
            .enumerate()
            .map(|(i, c)| (i, *c))
            .filter(|(_, c)| *c)
            .collect::<Vec<_>>();
        for (i, _connect) in connect_list {
            let dir_idx = i % 4;
            let pos = Position {
                x: position.x + rel_dir[dir_idx][0],
                y: position.y + rel_dir[dir_idx][1],
            };
            if pos.x < 0 || state.width <= pos.x as u32 || pos.y < 0 || state.height <= pos.y as u32
            {
                continue;
            }
            if let Some(structure) = structures.map(|s| s).find(|s| *s.dynamic.position() == pos) {
                let mut process_fluid_box = |self_box: &mut FluidBox, fluid_box: &mut FluidBox| {
                    // Different types of fluids won't mix
                    if 0. < fluid_box.amount
                        && fluid_box.type_ != self_box.type_
                        && fluid_box.type_.is_some()
                    {
                        return;
                    }
                    let pressure = fluid_box.amount - self_box.amount;
                    let flow = pressure * 0.1;
                    // Check input/output valve state
                    if if flow < 0. {
                        !self_box.output_enable
                            || !fluid_box.input_enable
                            || fluid_box.filter.is_some() && fluid_box.filter != self_box.type_
                    } else {
                        !self_box.input_enable
                            || !fluid_box.output_enable
                            || self_box.filter.is_some() && self_box.filter != fluid_box.type_
                    } {
                        return;
                    }
                    fluid_box.amount -= flow;
                    self_box.amount += flow;
                    if flow < 0. {
                        fluid_box.type_ = self_box.type_;
                    } else {
                        self_box.type_ = fluid_box.type_;
                    }
                    if biggest_flow_amount < flow.abs() {
                        biggest_flow_amount = flow;
                        _biggest_flow_idx = i as isize;
                    }
                };
                if let Some(fluid_boxes) = structure.dynamic.fluid_box_mut() {
                    for fluid_box in fluid_boxes {
                        process_fluid_box(self, fluid_box);
                    }
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct WaterWell {
    position: Position,
    output_fluid_box: FluidBox,
}

impl WaterWell {
    pub(crate) fn new(position: &Position) -> Self {
        WaterWell {
            position: *position,
            output_fluid_box: FluidBox::new(false, true, [false; 4]).set_type(&FluidType::Water),
        }
    }
}

impl Structure for WaterWell {
    fn name(&self) -> &str {
        "Water Well"
    }

    fn position(&self) -> &Position {
        &self.position
    }

    fn draw(
        &self,
        _burner: Option<&Burner>,
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
        match state.image_water_well.as_ref() {
            Some(img) => {
                context.draw_image_with_image_bitmap(&img.bitmap, x, y)?;
            }
            None => return Err(JsValue::from_str("furnace image not available")),
        }

        Ok(())
    }

    fn desc(&self, _burner: Option<&Burner>, _state: &FactorishState) -> String {
        format!(
            "{}<br>{}",
            self.output_fluid_box.desc(),
            "Outputs: Water<br>",
        )
    }

    fn frame_proc(
        &mut self,
        state: &mut FactorishState,
        structures: &mut dyn DynIterMut<Item = StructureBundle>,
        _burner: Option<&mut Burner>,
    ) -> Result<FrameProcResult, ()> {
        self.output_fluid_box.amount =
            (self.output_fluid_box.amount + 1.).min(self.output_fluid_box.max_amount);
        let connections = self.connection(state, structures.as_dyn_iter());
        self.output_fluid_box.connect_to = connections;
        self.output_fluid_box
            .simulate(&self.position, state, &mut structures.dyn_iter_mut());
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
