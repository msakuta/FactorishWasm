use super::{
    components::{
        burner::Burner,
        fluid_box::{FluidBox, FluidType, InputFluidBox, OutputFluidBox},
    },
    pipe::Pipe,
    serialize_impl,
    structure::{Energy, Structure, StructureComponents},
    FactorishState, FrameProcResult, Inventory, ItemType, Position, Recipe, TempEnt,
};
use serde::{Deserialize, Serialize};
use specs::{Builder, Entity, World, WorldExt};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

use std::collections::HashMap;

const FUEL_CAPACITY: usize = 10;

#[derive(Serialize, Deserialize)]
pub(crate) struct Boiler {
    progress: Option<f64>,
    recipe: Option<Recipe>,
    // input_fluid_box: FluidBox,
    // output_fluid_box: FluidBox,
}

impl Boiler {
    pub(crate) fn new(world: &mut World, position: Position) -> Entity {
        world
            .create_entity()
            .with(Box::new(Boiler {
                progress: None,
                recipe: Some(Recipe {
                    input: hash_map!(ItemType::CoalOre => 1usize),
                    input_fluid: Some(FluidType::Water),
                    output: HashMap::new(),
                    output_fluid: Some(FluidType::Steam),
                    power_cost: 100.,
                    recipe_time: 30.,
                }),
            }) as Box<dyn Structure + Send + Sync>)
            .with(position)
            .with(Burner {
                inventory: Inventory::new(),
                capacity: FUEL_CAPACITY,
            })
            .with(Energy {
                value: 0.,
                max: 100.,
            })
            .with(InputFluidBox(FluidBox::new(true, false)))
            .with(OutputFluidBox(FluidBox::new(false, true)))
            .build()
    }

    const FLUID_PER_PROGRESS: f64 = 100.;
    const COMBUSTION_EPSILON: f64 = 1e-6;

    fn combustion_rate(
        &self,
        _burner: &Burner,
        energy: &Energy,
        input_fluid_box: &FluidBox,
        output_fluid_box: &FluidBox,
    ) -> f64 {
        if let Some(ref recipe) = self.recipe {
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
        entity: Entity,
        components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_tooltip: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        Pipe::draw_int(entity, self, components, state, context, depth, false)?;
        let (x, y) = if let Some(position) = &components.position {
            (position.x as f64 * 32., position.y as f64 * 32.)
        } else {
            (0., 0.)
        };
        match state.image_boiler.as_ref() {
            Some(img) => {
                let sx = if let Some(energy) = components.energy.as_ref() {
                    let input_fluid_box = components
                        .input_fluid_box
                        .as_ref()
                        .ok_or_else(|| js_str!("Boiler without InputFluidBox component"))?;
                    let output_fluid_box = components
                        .output_fluid_box
                        .as_ref()
                        .ok_or_else(|| js_str!("Boiler without OutputFluidBox component"))?;
                    if self.progress.is_some()
                        && components
                            .burner
                            .as_ref()
                            .map(|burner| {
                                Self::COMBUSTION_EPSILON
                                    < self.combustion_rate(
                                        burner,
                                        energy,
                                        &input_fluid_box.0,
                                        &output_fluid_box.0,
                                    )
                            })
                            .unwrap_or(false)
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

    fn desc(&self, entity: Entity, state: &FactorishState) -> String {
        use specs::Join;
        let burner = state.world.read_component::<Burner>();
        let energy = state.world.read_component::<Energy>();
        let (_burner, energy) = if let Some(burner) = (&burner, &energy)
            .join()
            .get(entity, &state.world.entities())
        {
            burner
        } else {
            return "Burner not found".to_string();
        };
        format!(
            "{}",
            if self.recipe.is_some() {
                // Progress bar
                // format!("{}{}{}{}Input fluid: {}Output fluid: {}",
                format!("{}{}{}{}",
                    format!("Progress: {:.0}%<br>", self.progress.unwrap_or(0.) * 100.),
                    "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
                    format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                        self.progress.unwrap_or(0.) * 100.),
                    format!(r#"Power: {:.1}kJ <div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>
                    <div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>"#,
                    energy.value,
                    if 0. < energy.max { (energy.value) / energy.max * 100. } else { 0. }),
                    // self.input_fluid_box.desc(),
                    // self.output_fluid_box.desc()
                )
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
        _entity: Entity,
        components: &mut StructureComponents,
        state: &mut FactorishState,
    ) -> Result<FrameProcResult, ()> {
        let position = components.position.as_ref().ok_or(())?;
        let burner = components.burner.as_mut().ok_or(())?;
        let energy = components.energy.as_mut().ok_or(())?;
        let input_fluid_box = components.input_fluid_box.as_mut().ok_or(())?;
        let output_fluid_box = components.output_fluid_box.as_mut().ok_or(())?;
        if let Some(recipe) = &self.recipe {
            if input_fluid_box.0.type_ == Some(FluidType::Water) {
                self.progress = Some(0.);
            }
            let ret = FrameProcResult::None;

            if let Some(prev_progress) = self.progress {
                // Proceed only if we have sufficient energy in the buffer.
                let progress =
                    self.combustion_rate(burner, energy, &input_fluid_box.0, &output_fluid_box.0);
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
                    output_fluid_box.0.type_ = Some(FluidType::Steam);
                    output_fluid_box.0.amount += progress * Self::FLUID_PER_PROGRESS;
                    input_fluid_box.0.amount -= progress * Self::FLUID_PER_PROGRESS;
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
