use super::{
    gl::utils::{enable_buffer, Flatten},
    pipe::Pipe,
    structure::{Structure, StructureDynIter, StructureId},
    FactorishState, FrameProcResult, Position,
};
use cgmath::{Matrix3, Matrix4, Vector3};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, WebGlRenderingContext as GL};

use std::cmp::Eq;

const FLOW_PER_PRESSURE: f64 = 0.1 / 0.05 / 60.;

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
    #[serde(skip)]
    pub flow: [f64; 4],
}

impl FluidBox {
    pub(crate) fn new(input_enable: bool, output_enable: bool) -> Self {
        Self {
            type_: None,
            amount: 0.,
            max_amount: 100.,
            input_enable,
            output_enable,
            connect_to: [None; 4],
            filter: None,
            flow: [0.; 4],
        }
    }

    pub(crate) fn new_with_filter(
        input_enable: bool,
        output_enable: bool,
        filter: Option<FluidType>,
    ) -> Self {
        Self {
            type_: None,
            amount: 0.,
            max_amount: 100.,
            input_enable,
            output_enable,
            connect_to: [None; 4],
            filter,
            flow: [0.; 4],
        }
    }

    pub(crate) fn set_type(mut self, type_: &FluidType) -> Self {
        self.type_ = Some(*type_);
        self
    }

    pub(crate) fn desc(&self) -> String {
        let amount_ratio = self.amount / self.max_amount * 100.;
        // Progress bar
        format!("{}{}{}connect: {:?}<br>flow: {:?}",
            format!("{}: {:.0}%<br>", self.type_.map(|v| format!("{:?}", v)).unwrap_or_else(|| "None".to_string()), amount_ratio),
            "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
            format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                amount_ratio),
            self.connect_to,
            self.flow,
            )
    }

    pub(crate) fn simulate(&mut self, structures: &mut StructureDynIter) {
        let mut _biggest_flow_idx = -1;
        let mut biggest_flow_amount = 1e-3; // At least this amount of flow is required for displaying flow direction
                                            // In an unlikely event, a fluid box without either input or output ports has nothing to do
        if self.amount == 0. || !self.input_enable && !self.output_enable {
            return;
        }
        self.flow = [0.; 4];
        let connect_list = self
            .connect_to
            .iter()
            .zip(self.flow.iter_mut())
            .enumerate()
            .filter_map(|(i, (c, f))| Some((i, ((*c)?, f))));
        for (i, (id, flow)) in connect_list {
            if let Some(fluid_boxes) = structures.get_mut(id).map(|s| s.fluid_box_mut()).flatten() {
                for fluid_box in fluid_boxes {
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
                    let flow_amount = pressure * FLOW_PER_PRESSURE;
                    // Check input/output valve state
                    if if flow_amount < 0. {
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
                    fluid_box.amount -= flow_amount;
                    self.amount += flow_amount;
                    if flow_amount < 0. {
                        fluid_box.type_ = self.type_;
                    } else {
                        self.type_ = fluid_box.type_;
                    }
                    if biggest_flow_amount < flow_amount.abs() {
                        biggest_flow_amount = flow_amount;
                        _biggest_flow_idx = i as isize;
                    }
                    *flow = flow_amount;
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
            output_fluid_box: FluidBox::new(false, true).set_type(&FluidType::Water),
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

    fn draw_gl(
        &self,
        state: &FactorishState,
        gl: &GL,
        depth: i32,
        is_ghost: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        Pipe::draw_gl_int(self, state, gl, depth, false, is_ghost)?;
        let (x, y) = (
            self.position.x as f32 + state.viewport.x as f32,
            self.position.y as f32 + state.viewport.y as f32,
        );
        let shader = state
            .assets
            .textured_shader
            .as_ref()
            .ok_or_else(|| js_str!("Shader not found"))?;
        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), if is_ghost { 0.5 } else { 1. });
        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_water_well));
        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::from_scale(1.).flatten(),
        );

        enable_buffer(&gl, &state.assets.screen_buffer, 2, shader.vertex_position);
        gl.uniform_matrix4fv_with_f32_array(
            shader.transform_loc.as_ref(),
            false,
            (state.get_world_transform()?
                * Matrix4::from_scale(2.)
                * Matrix4::from_translation(Vector3::new(x, y, 0.)))
            .flatten(),
        );
        gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
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
        _me: StructureId,
        _state: &mut FactorishState,
        structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        structures.get_mut(StructureId { id: 0, gen: 0 });
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
