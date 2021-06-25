use super::{
    burner::Burner,
    pipe::Pipe,
    serialize_impl,
    structure::{DynIterMut, Structure, StructureBundle},
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
    position: Position,
    progress: Option<f64>,
    recipe: Option<Recipe>,
    input_fluid_box: FluidBox,
    output_fluid_box: FluidBox,
}

impl Boiler {
    pub(crate) fn new(position: &Position) -> StructureBundle {
        StructureBundle {
            dynamic: Box::new(Boiler {
                position: *position,
                progress: None,
                recipe: Some(Recipe {
                    input: hash_map!(ItemType::CoalOre => 1usize),
                    input_fluid: Some(FluidType::Water),
                    output: HashMap::new(),
                    output_fluid: Some(FluidType::Steam),
                    power_cost: 100.,
                    recipe_time: 30.,
                }),
                input_fluid_box: FluidBox::new(true, false, [false; 4]),
                output_fluid_box: FluidBox::new(false, true, [false; 4]),
            }),
            burner: Some(Burner {
                inventory: Inventory::new(),
                capacity: FUEL_CAPACITY,
                energy: 0.,
                max_energy: 100.,
            }),
        }
    }

    const FLUID_PER_PROGRESS: f64 = 100.;
    const COMBUSTION_EPSILON: f64 = 1e-6;

    fn combustion_rate(&self, burner: &Burner) -> f64 {
        if let Some(ref recipe) = self.recipe {
            (burner.energy / recipe.power_cost)
                .min(1. / recipe.recipe_time)
                .min(self.input_fluid_box.amount / Self::FLUID_PER_PROGRESS)
                .min(
                    (self.output_fluid_box.max_amount - self.output_fluid_box.amount)
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

    fn position(&self) -> &Position {
        &self.position
    }

    fn draw(
        &self,
        burner: Option<&Burner>,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_tooltip: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        Pipe::draw_int(self, state, context, depth, false)?;
        let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
        match state.image_boiler.as_ref() {
            Some(img) => {
                let sx = if self.progress.is_some()
                    && burner
                        .map(|burner| Self::COMBUSTION_EPSILON < self.combustion_rate(burner))
                        .unwrap_or(false)
                {
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

    fn desc(&self, burner: Option<&Burner>, _state: &FactorishState) -> String {
        let burner = if let Some(burner) = burner {
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
                    burner.energy,
                    if 0. < burner.max_energy { (burner.energy) / burner.max_energy * 100. } else { 0. }),
                    self.input_fluid_box.desc(),
                    self.output_fluid_box.desc())
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
        state: &mut FactorishState,
        structures: &mut dyn DynIterMut<Item = StructureBundle>,
        burner: Option<&mut Burner>,
    ) -> Result<FrameProcResult, ()> {
        let connections = self.connection(state, structures.as_dyn_iter());
        let burner = burner.ok_or(())?;
        self.output_fluid_box.connect_to = connections;
        self.input_fluid_box
            .simulate(&self.position, state, &mut structures.dyn_iter_mut());
        self.output_fluid_box
            .simulate(&self.position, state, &mut structures.dyn_iter_mut());
        if let Some(recipe) = &self.recipe {
            if self.input_fluid_box.type_ == Some(FluidType::Water) {
                self.progress = Some(0.);
            }
            let ret = FrameProcResult::None;

            if let Some(prev_progress) = self.progress {
                // Proceed only if we have sufficient energy in the buffer.
                let progress = self.combustion_rate(burner);
                if state.rng.next() < progress * 10. {
                    state
                        .temp_ents
                        .push(TempEnt::new(&mut state.rng, self.position));
                }
                if 1. <= prev_progress + progress {
                    self.progress = None;

                    return Ok(FrameProcResult::InventoryChanged(self.position));
                } else if Self::COMBUSTION_EPSILON < progress {
                    self.progress = Some(prev_progress + progress);
                    burner.energy -= progress * recipe.power_cost;
                    self.output_fluid_box.type_ = Some(FluidType::Steam);
                    self.output_fluid_box.amount += progress * Self::FLUID_PER_PROGRESS;
                    self.input_fluid_box.amount -= progress * Self::FLUID_PER_PROGRESS;
                }
            }
            return Ok(ret);
        }
        Ok(FrameProcResult::None)
    }

    fn get_selected_recipe(&self) -> Option<&Recipe> {
        self.recipe.as_ref()
    }

    fn fluid_box(&self) -> Option<Vec<&FluidBox>> {
        Some(vec![&self.input_fluid_box, &self.output_fluid_box])
    }

    fn fluid_box_mut(&mut self) -> Option<Vec<&mut FluidBox>> {
        Some(vec![&mut self.input_fluid_box, &mut self.output_fluid_box])
    }

    serialize_impl!();
}
