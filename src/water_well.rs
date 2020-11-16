use super::structure::{DynIterMut, Structure};
use super::{log, FactorishState, FrameProcResult, Inventory, ItemType, Position, Recipe};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

use std::cmp::Eq;
use std::collections::HashMap;

#[derive(Eq, PartialEq, Clone, Copy)]
pub(crate) enum FluidType {
    Water,
    // Steam,
}

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

    fn simulate(
        &mut self,
        position: &Position,
        state: &mut FactorishState,
        structures: &mut dyn Iterator<Item = &mut Box<dyn Structure>>,
    ) {
        let mut _biggest_flow_idx = -1;
        let mut biggest_flow_amount = 1e-3; // At least this amount of flow is required for displaying flow direction
                                            // In an unlikely event, a fluid box without either input or output ports has nothing to do
        if self.amount == 0. || !self.input_enable && !self.output_enable {
            return;
        }
        let rel_dir = [[-1, 0], [0, -1], [1, 0], [0, 1]];
        for (i, _connect) in self.connect_to.iter().enumerate().filter(|(_, c)| **c) {
            let dir_idx = i % 4;
            let pos = Position {
                x: position.x + rel_dir[dir_idx][0],
                y: position.y + rel_dir[dir_idx][1],
            };
            if pos.x < 0 || state.width <= pos.x as u32 || pos.y < 0 || state.height <= pos.y as u32
            {
                continue;
            }
            console_log!("FluidBox checking struct at {:?}", pos);
            if let Some(structure) = structures.map(|s| s).find(|s| *s.position() == pos) {
                console_log!("FluidBox found struct at {:?}", pos);
                if let Some(fluid_box) = structure.fluid_box_mut() {
                    // Different types of fluids won't mix
                    if 0. < fluid_box.amount && fluid_box.type_ != self.type_ {
                        continue;
                    }
                    let pressure = fluid_box.amount - self.amount;
                    let flow = pressure * 0.1;
                    // Check input/output valve state
                    if if flow < 0. {
                        !self.output_enable
                            || !fluid_box.input_enable
                            || fluid_box.filter != self.type_
                    } else {
                        !self.input_enable
                            || !fluid_box.output_enable && self.filter != fluid_box.type_
                    } {
                        continue;
                    }
                    fluid_box.amount -= flow;
                    self.amount += flow;
                    fluid_box.type_ = self.type_;
                    if biggest_flow_amount < flow.abs() {
                        biggest_flow_amount = flow;
                        _biggest_flow_idx = i as isize;
                    }
                }
            }
        }
    }
}

pub(crate) struct WaterWell {
    position: Position,
    inventory: Inventory,
    progress: Option<f64>,
    power: f64,
    recipe: Option<Recipe>,
    output_fluid_box: FluidBox,
}

impl WaterWell {
    pub(crate) fn new(position: &Position) -> Self {
        WaterWell {
            position: *position,
            inventory: Inventory::new(),
            progress: None,
            power: 0.,
            recipe: Some(Recipe {
                input: hash_map!(ItemType::CoalOre => 1usize),
                output: HashMap::new(),
                power_cost: 0.,
                recipe_time: 30.,
            }),
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
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
        match state.image_water_well.as_ref() {
            Some(img) => {
                let sx = if self.progress.is_some() && 0. < self.power {
                    ((((state.sim_time * 5.) as isize) % 2 + 1) * 32) as f64
                } else {
                    0.
                };
                context.draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    &img.bitmap,
                    sx,
                    0.,
                    32.,
                    32.,
                    x,
                    y,
                    32.,
                    32.,
                )?;
            }
            None => return Err(JsValue::from_str("furnace image not available")),
        }

        Ok(())
    }

    fn desc(&self, _state: &FactorishState) -> String {
        format!(
            "{}<br>{}",
            if self.recipe.is_some() {
                let amount_ratio =
                    self.output_fluid_box.amount / self.output_fluid_box.max_amount * 100.;
                // Progress bar
                format!("{}{}{}",
                    format!("Water: {:.0}%<br>", amount_ratio),
                    "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
                    format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                        amount_ratio),
                    )
            // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
            // "Outputs: <br>" +
            // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
            } else {
                String::from("No recipe")
            },
            format!(
                "Items: \n{}",
                self.inventory
                    .iter()
                    .map(|item| format!("{:?}: {}<br>", item.0, item.1))
                    .fold(String::from(""), |accum, item| accum + &item)
            )
        )
    }

    fn frame_proc(
        &mut self,
        state: &mut FactorishState,
        structures: &mut dyn DynIterMut<Item = Box<dyn Structure>>,
    ) -> Result<FrameProcResult, ()> {
        self.output_fluid_box.amount =
            (self.output_fluid_box.amount + 1.).min(self.output_fluid_box.max_amount);
        let connections = self.connection(state, structures.as_dyn_iter());
        self.output_fluid_box.connect_to = [
            connections & 1 != 0,
            connections & 2 != 0,
            connections & 4 != 0,
            connections & 8 != 0,
        ];
        console_log!(
            "WaterWell: conn: {}, {:?}",
            connections,
            self.output_fluid_box.connect_to
        );
        self.output_fluid_box
            .simulate(&self.position, state, &mut structures.dyn_iter_mut());
        Ok(FrameProcResult::None)
    }

    fn get_selected_recipe(&self) -> Option<&Recipe> {
        self.recipe.as_ref()
    }

    fn fluid_box(&self) -> Option<&FluidBox> {
        Some(&self.output_fluid_box)
    }
}
