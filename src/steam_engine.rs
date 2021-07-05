use super::{
    components::fluid_box::{FluidBox, FluidType, InputFluidBox},
    pipe::Pipe,
    serialize_impl,
    structure::{Rotation, Structure, StructureBoxed, StructureComponents},
    FactorishState, FrameProcResult, Position, Recipe,
};
use serde::{Deserialize, Serialize};
use specs::{Builder, Entity, World, WorldExt};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

use std::collections::HashMap;

#[derive(Serialize, Deserialize)]
pub(crate) struct SteamEngine {
    progress: Option<f64>,
    power: f64,
    max_power: f64,
    recipe: Option<Recipe>,
}

impl SteamEngine {
    pub(crate) fn new(world: &mut World, position: Position) -> Entity {
        world
            .create_entity()
            .with(Box::new(SteamEngine {
                progress: None,
                power: 0.,
                max_power: 100.,
                recipe: Some(Recipe {
                    input: HashMap::new(),
                    input_fluid: Some(FluidType::Steam),
                    output: HashMap::new(),
                    output_fluid: None,
                    power_cost: -100.,
                    recipe_time: 100.,
                }),
            }) as StructureBoxed)
            .with(position)
            .with(InputFluidBox(FluidBox::new(true, false)))
            .build()
    }

    const FLUID_PER_PROGRESS: f64 = 100.;
    const COMBUSTION_EPSILON: f64 = 1e-6;

    fn combustion_rate(&self, input_fluid_box: &InputFluidBox) -> f64 {
        if let Some(ref recipe) = self.recipe {
            ((self.max_power - self.power) / recipe.power_cost.abs())
                .min(1. / recipe.recipe_time)
                .min(input_fluid_box.0.amount / Self::FLUID_PER_PROGRESS)
                .min(1.)
                .max(0.)
        } else {
            0.
        }
    }

    fn draw_int(
        x: f64,
        y: f64,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        working: bool,
    ) -> Result<(), JsValue> {
        match state.image_steam_engine.as_ref() {
            Some(img) => {
                let sx = if working {
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
                )
            }
            None => return Err(JsValue::from_str("furnace image not available")),
        }
    }

    pub(crate) fn draw_static(
        x: f64,
        y: f64,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
    ) -> Result<(), JsValue> {
        Self::draw_int(x, y, state, context, false)?;
        Ok(())
    }
}

impl Structure for SteamEngine {
    fn name(&self) -> &str {
        "Steam Engine"
    }

    fn draw(
        &self,
        entity: Entity,
        components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        Pipe::draw_int(components, state, context, depth, false)?;
        let (x, y) = if let Some(position) = &components.position {
            (position.x as f64 * 32., position.y as f64 * 32.)
        } else {
            (0., 0.)
        };
        let input_fluid_box = components
            .input_fluid_box
            .as_ref()
            .ok_or_else(|| js_str!("SteamEngine without InputFluidBox component"))?;
        let working = self.progress.is_some()
            && Self::COMBUSTION_EPSILON < self.combustion_rate(&input_fluid_box);
        SteamEngine::draw_int(x, y, state, context, working)
    }

    fn desc(&self, entity: Entity, state: &FactorishState) -> String {
        let input_fluid_box_storage = state.world.read_component::<InputFluidBox>();
        let input_fluid_box = if let Some(ifb) = input_fluid_box_storage.get(entity) {
            ifb
        } else {
            return "SteamEngine without InputFluidBox component".to_string();
        };
        if self.recipe.is_some() {
            // Progress bar
            format!("{}{}{}{}{}",
                format!("Progress: {:.0}%<br>", self.progress.unwrap_or(0.) * 100.),
                "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
                format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                    self.progress.unwrap_or(0.) * 100.),
                format!(r#"Power: {:.1}kJ <div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>
                <div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>"#,
                self.power,
                if 0. < self.max_power { (self.power) / self.max_power * 100. } else { 0. }),
                format!("<div>Combustion rate: {:.1}</div>", self.combustion_rate(input_fluid_box)),
                )
        // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
        // "Outputs: <br>" +
        // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
        } else {
            String::from("No recipe")
        }
    }

    fn frame_proc(
        &mut self,
        _entity: Entity,
        components: &mut StructureComponents,
        state: &mut FactorishState,
    ) -> Result<FrameProcResult, ()> {
        let position = components.position.as_ref().ok_or(())?;
        let input_fluid_box = components.input_fluid_box.as_mut().ok_or(())?;
        if let Some(recipe) = &self.recipe {
            if input_fluid_box.0.type_ == recipe.input_fluid {
                self.progress = Some(0.);
            }
            let ret = FrameProcResult::None;

            if let Some(prev_progress) = self.progress {
                // Proceed only if we have sufficient energy in the buffer.
                let progress = self.combustion_rate(&input_fluid_box);
                if 1. <= prev_progress + progress {
                    self.progress = None;
                    return Ok(FrameProcResult::InventoryChanged(*position));
                } else if Self::COMBUSTION_EPSILON < progress {
                    self.progress = Some(prev_progress + progress);
                    self.power -= progress * recipe.power_cost;
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

    fn power_source(&self) -> bool {
        true
    }

    fn power_outlet(&mut self, demand: f64) -> Option<f64> {
        let energy = demand.min(self.power);
        self.power -= energy;
        Some(energy)
    }

    serialize_impl!();
}
