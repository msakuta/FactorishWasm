use super::{
    burner::Burner,
    pipe::Pipe,
    serialize_impl,
    structure::{
        Energy, Structure, StructureBundle, StructureComponents, StructureDynIter, StructureId,
    },
    water_well::{FluidBox, FluidType},
    FactorishState, FrameProcResult, Inventory, ItemType, Position, Recipe, TempEnt,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

use std::collections::HashMap;

const FUEL_CAPACITY: usize = 10;

#[derive(Serialize, Deserialize)]
pub(crate) struct Boiler {
    progress: Option<f64>,
    recipe: Option<Recipe>,
}

impl Boiler {
    pub(crate) fn new(position: &Position) -> StructureBundle {
        StructureBundle {
            dynamic: Box::new(Boiler {
                progress: None,
                recipe: Some(Recipe {
                    input: hash_map!(ItemType::CoalOre => 1usize),
                    input_fluid: Some(FluidType::Water),
                    output: HashMap::new(),
                    output_fluid: Some(FluidType::Steam),
                    power_cost: 100.,
                    recipe_time: 30.,
                }),
            }),
            components: StructureComponents {
                position: Some(*position),
                rotation: None,
                burner: Some(Burner {
                    inventory: Inventory::new(),
                    capacity: FUEL_CAPACITY,
                }),
                energy: Some(Energy {
                    value: 0.,
                    max: 100.,
                }),
                factory: None,
                fluid_boxes: vec![
                    FluidBox::new(true, false, [None; 4]),
                    FluidBox::new(false, true, [None; 4]),
                ],
            },
        }
    }

    const FLUID_PER_PROGRESS: f64 = 100.;
    const COMBUSTION_EPSILON: f64 = 1e-6;

    fn combustion_rate(&self, energy: &Energy, fluid_boxes: &[FluidBox]) -> f64 {
        if let ([input_fluid_box, output_fluid_box], Some(ref recipe)) =
            (&fluid_boxes[..2], &self.recipe)
        {
            (energy.value / recipe.power_cost)
                .min(1. / recipe.recipe_time)
                .min(input_fluid_box.amount / Self::FLUID_PER_PROGRESS)
                .min(
                    (output_fluid_box.max_amount - output_fluid_box.amount)
                        / Self::FLUID_PER_PROGRESS,
                )
                .min(1.)
        } else {
            0.
        }
    }
}

impl Structure for Boiler {
    fn name(&self) -> &str {
        "Boiler"
    }

    fn draw(
        &self,
        components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_tooltip: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        Pipe::draw_int(self, components, state, context, depth, false)?;
        let (x, y) = if let Some(position) = &components.position {
            (position.x as f64 * 32., position.y as f64 * 32.)
        } else {
            (0., 0.)
        };
        match state.image_boiler.as_ref() {
            Some(img) => {
                let sx = if let Some(energy) = components.energy.as_ref() {
                    if self.progress.is_some()
                        && Self::COMBUSTION_EPSILON
                            < self.combustion_rate(energy, &components.fluid_boxes)
                    {
                        ((((state.sim_time * 5.) as isize) % 2 + 1) * 32) as f64
                    } else {
                        0.
                    }
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

    fn desc(&self, components: &StructureComponents, _state: &FactorishState) -> String {
        let (_burner, energy) =
            if let Some(burner) = components.burner.as_ref().zip(components.energy.as_ref()) {
                burner
            } else {
                return "Burner not found".to_string();
            };
        format!(
            "{}",
            if self.recipe.is_some() {
                // Progress bar
                format!("{}{}{}{}Input fluid: {}Output fluid: {}",
                    format!("Progress: {:.0}%<br>", self.progress.unwrap_or(0.) * 100.),
                    "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
                    format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                        self.progress.unwrap_or(0.) * 100.),
                    format!(r#"Power: {:.1}kJ <div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>
                    <div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>"#,
                    energy.value,
                    if 0. < energy.max { (energy.value) / energy.max * 100. } else { 0. }),
                    components.fluid_boxes.get(0).map(|f| f.desc()).unwrap_or_else(|| "".to_string()),
                    components.fluid_boxes.get(1).map(|f| f.desc()).unwrap_or_else(|| "".to_string()))
            // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
            // "Outputs: <br>" +
            // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
            } else {
                String::from("No recipe")
            },
        )
    }

    fn frame_proc(
        &mut self,
        _me: StructureId,
        components: &mut StructureComponents,
        state: &mut FactorishState,
        structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        let position = components.position.as_ref().ok_or(())?;
        let energy = components.energy.as_mut().ok_or(())?;
        if components.fluid_boxes.len() < 2 {
            return Err(());
        }
        if let Some(recipe) = &self.recipe {
            if components.fluid_boxes[0].type_ == Some(FluidType::Water) {
                self.progress = Some(0.);
            }
            let ret = FrameProcResult::None;

            if let Some(prev_progress) = self.progress {
                // Proceed only if we have sufficient energy in the buffer.
                let progress = self.combustion_rate(energy, &components.fluid_boxes);
                if state.rng.next() < progress * 10. {
                    state
                        .temp_ents
                        .push(TempEnt::new(&mut state.rng, *position));
                }
                if 1. <= prev_progress + progress {
                    self.progress = None;

                    return Ok(FrameProcResult::InventoryChanged(*position));
                } else if Self::COMBUSTION_EPSILON < progress {
                    self.progress = Some(prev_progress + progress);
                    energy.value -= progress * recipe.power_cost;
                    if let [input_fluid_box, output_fluid_box] = &mut components.fluid_boxes[..2] {
                        output_fluid_box.type_ = Some(FluidType::Steam);
                        output_fluid_box.amount += progress * Self::FLUID_PER_PROGRESS;
                        input_fluid_box.amount -= progress * Self::FLUID_PER_PROGRESS;
                    } else {
                        return Err(());
                    };
                }
            }
            return Ok(ret);
        }
        Ok(FrameProcResult::None)
    }

    fn get_selected_recipe(&self) -> Option<&Recipe> {
        self.recipe.as_ref()
    }

    serialize_impl!();
}
