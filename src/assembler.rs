use super::{
    burner::Burner,
    factory::Factory,
    items::get_item_image_url,
    serialize_impl,
    structure::{DynIterMut, Energy, Structure, StructureBundle, StructureComponents},
    FactorishState, FrameProcResult, InventoryTrait, ItemType, Position, PowerWire, Recipe,
    TILE_SIZE,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

fn generate_item_image(item_image: &str, icon_size: bool, count: usize) -> String {
    let size = 32;
    format!("<div style=\"background-image: url('{}'); width: {}px; height: {}px; display: inline-block\"     draggable='false'>{}</div>",
        item_image, size, size,
    if icon_size && 0 < count {
        format!("<span class='overlay noselect' style='position: relative; display: inline-block; width: {}px; height: {}px'>{}</span>", size, size, count)
    } else {
        "".to_string()
    })
}

fn _recipe_html(state: &FactorishState, recipe: &Recipe) -> String {
    let mut ret = String::from("");
    ret += "<div class='recipe-box'>";
    ret += &format!(
        "<span style='display: inline-block; margin: 1px'>{}</span>",
        &generate_item_image("time", true, recipe.recipe_time as usize)
    );
    ret += "<span style='display: inline-block; width: 50%'>";
    for (key, value) in &recipe.input {
        ret += &generate_item_image(get_item_image_url(state, &key), true, *value);
    }
    ret += "</span><img src='img/rightarrow.png' style='width: 20px; height: 32px'><span style='display: inline-block; width: 10%'>";
    for (key, value) in &recipe.output {
        ret += &generate_item_image(get_item_image_url(state, &key), true, *value);
    }
    ret += "</span></div>";
    ret
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Assembler {
    position: Position,
}

impl Assembler {
    pub(crate) fn new(position: &Position) -> StructureBundle {
        StructureBundle::new(
            Box::new(Assembler {
                position: *position,
            }),
            Some(*position),
            None,
            Some(Energy {
                value: 0.,
                max: 100.,
            }),
            Some(Factory::new()),
        )
    }

    /// Find all power sources that are connected to this structure through wires.
    fn find_power_sources(
        &self,
        state: &mut FactorishState,
        structures: &mut dyn DynIterMut<Item = StructureBundle>,
    ) -> Vec<Position> {
        let mut checked = HashMap::<PowerWire, ()>::new();
        let mut expand_list = HashMap::<Position, Vec<PowerWire>>::new();
        let mut ret = vec![];
        let mut check_struct = |position: &Position| {
            if structures.dyn_iter().any(|structure| {
                structure.dynamic.power_source() && *structure.dynamic.position() == *position
            }) {
                ret.push(*position);
            }
        };
        for wire in &state.power_wires {
            if wire.0 == self.position {
                expand_list.insert(wire.1, vec![*wire]);
                checked.insert(*wire, ());
                check_struct(&wire.1);
            } else if wire.1 == self.position {
                expand_list.insert(wire.0, vec![*wire]);
                checked.insert(*wire, ());
                check_struct(&wire.0);
            }
        }
        // Simple Dijkstra
        while !expand_list.is_empty() {
            let mut next_expand = HashMap::<Position, Vec<PowerWire>>::new();
            for check in &expand_list {
                for wire in &state.power_wires {
                    if checked.get(wire).is_some() {
                        continue;
                    }
                    if wire.0 == *check.0 {
                        next_expand.insert(wire.1, vec![*wire]);
                        checked.insert(*wire, ());
                        check_struct(&wire.1);
                    } else if wire.1 == *check.0 {
                        next_expand.insert(wire.0, vec![*wire]);
                        checked.insert(*wire, ());
                        check_struct(&wire.1);
                    }
                }
            }
            expand_list = next_expand;
        }
        // console_log!("Assember power sources: {:?}", ret);
        ret
    }
}

impl Structure for Assembler {
    fn name(&self) -> &str {
        "Assembler"
    }

    fn position(&self) -> &Position {
        &self.position
    }

    fn draw(
        &self,
        components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        is_toolbar: bool,
    ) -> Result<(), JsValue> {
        if depth == 0 {
            let (x, y) = (
                self.position.x as f64 * TILE_SIZE,
                self.position.y as f64 * TILE_SIZE,
            );
            match state.image_assembler.as_ref() {
                Some(img) => {
                    let sx = if let Some((energy, factory)) =
                        components.energy.as_ref().zip(components.factory.as_ref())
                    {
                        if factory.progress.is_some() && 0. < energy.value {
                            ((((state.sim_time * 5.) as isize) % 4) * 32) as f64
                        } else {
                            0.
                        }
                    } else {
                        0.
                    };
                    context
                        .draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                            &img.bitmap,
                            sx,
                            0.,
                            TILE_SIZE,
                            TILE_SIZE,
                            x,
                            y,
                            TILE_SIZE,
                            TILE_SIZE,
                        )?;
                }
                None => return Err(JsValue::from_str("assembler image not available")),
            }
            return Ok(());
        }
        if let Some((
            energy,
            Factory {
                recipe: Some(_recipe),
                ..
            },
        )) = components.energy.as_ref().zip(components.factory.as_ref())
        {
            if !is_toolbar && energy.value == 0. && state.sim_time % 1. < 0.5 {
                if let Some(img) = state.image_electricity_alarm.as_ref() {
                    let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
                    context.draw_image_with_image_bitmap(&img.bitmap, x, y)?;
                } else {
                    return js_err!("electricity alarm image not available");
                }
            }
        }

        Ok(())
    }

    fn desc(&self, components: &StructureComponents, _state: &FactorishState) -> String {
        let (energy, factory) =
            if let Some(bundle) = components.energy.as_ref().zip(components.factory.as_ref()) {
                bundle
            } else {
                return "Energy or Factory component not found".to_string();
            };
        format!(
            "{}<br>{}{}",
            if let Some(recipe) = &factory.recipe {
                // Progress bar
                format!("{}{}{}{}",
                    format!("Progress: {:.0}%<br>", factory.progress.unwrap_or(0.) * 100.),
                    "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
                    format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                        factory.progress.unwrap_or(0.) * 100.),
                    format!(r#"Power: {:.1}kJ <div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>
                    <div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>"#,
                    energy.value,
                    if 0. < energy.max { energy.value / energy.max * 100. } else { 0. }),
                    )
                + &generate_item_image(&_state.image_time.as_ref().unwrap().url, true, recipe.recipe_time as usize) + "<br>" +
                "Outputs: <br>" +
                &recipe.output.iter()
                    .map(|item| format!("{}<br>", &generate_item_image(get_item_image_url(_state, &item.0), true, *item.1)))
                    .fold::<String, _>("".to_string(), |a, s| a + &s)
            } else {
                String::from("No recipe")
            },
            format!("Input Items: <br>{}", factory.input_inventory.describe()),
            format!("Output Items: <br>{}", factory.output_inventory.describe())
        )
    }

    fn frame_proc(
        &mut self,
        _burner: Option<&mut Burner>,
        energy: Option<&mut super::structure::Energy>,
        factory: Option<&mut Factory>,
        _state: &mut FactorishState,
        structures: &mut dyn DynIterMut<Item = StructureBundle>,
    ) -> Result<FrameProcResult, ()> {
        if let Some((
            energy,
            Factory {
                recipe: Some(recipe),
                ..
            },
        )) = energy.zip(factory)
        {
            let mut ret = FrameProcResult::None;
            // First, check if we need to refill the energy buffer in order to continue the current work.
            // Refill the energy from the fuel
            if energy.value < recipe.power_cost {
                let mut accumulated = 0.;
                for position in self.find_power_sources(_state, structures) {
                    if let Some(structure) = structures
                        .dyn_iter_mut()
                        .find(|structure| *structure.dynamic.position() == position)
                    {
                        let demand = energy.max - energy.value - accumulated;
                        if let Some(energy) = structure.dynamic.power_outlet(demand) {
                            accumulated += energy;
                            // console_log!("draining {:?}kJ of energy with {:?} demand, from {:?}, accumulated {:?}", energy, demand, structure.name(), accumulated);
                        }
                    }
                }
                energy.value += accumulated;
                ret = FrameProcResult::InventoryChanged(self.position);
            }

            return Ok(ret);
        }
        Ok(FrameProcResult::None)
    }

    fn get_recipes(&self) -> Vec<Recipe> {
        vec![
            Recipe::new(
                hash_map!(ItemType::IronPlate => 2usize),
                hash_map!(ItemType::Gear => 1usize),
                20.,
                50.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 1usize, ItemType::Gear => 1usize),
                hash_map!(ItemType::TransportBelt => 1usize),
                20.,
                50.,
            ),
            Recipe::new(
                hash_map!(ItemType::TransportBelt => 2, ItemType::Gear => 2),
                hash_map!(ItemType::Splitter => 1),
                25.,
                40.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 5usize),
                hash_map!(ItemType::Chest => 1usize),
                20.,
                50.,
            ),
            Recipe::new(
                hash_map!(ItemType::CopperPlate => 1usize),
                hash_map!(ItemType::CopperWire => 2usize),
                20.,
                20.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 1, ItemType::CopperWire => 3usize),
                hash_map!(ItemType::Circuit => 1usize),
                20.,
                50.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 5, ItemType::Gear => 5, ItemType::Circuit => 3),
                hash_map!(ItemType::Assembler => 1),
                20.,
                120.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 1, ItemType::Gear => 1, ItemType::Circuit => 1),
                hash_map!(ItemType::Inserter => 1),
                20.,
                20.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 1, ItemType::Gear => 5, ItemType::Circuit => 3),
                hash_map!(ItemType::OreMine => 1),
                100.,
                100.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 2),
                hash_map!(ItemType::Pipe => 1),
                20.,
                20.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 2, ItemType::CopperPlate => 3),
                hash_map!(ItemType::WaterWell => 1),
                100.,
                100.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 5, ItemType::CopperPlate => 5),
                hash_map!(ItemType::Boiler => 1),
                100.,
                100.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 5, ItemType::Gear => 5, ItemType::CopperPlate => 5),
                hash_map!(ItemType::SteamEngine => 1),
                200.,
                200.,
            ),
            Recipe::new(
                hash_map!(ItemType::IronPlate => 2, ItemType::CopperWire => 2),
                hash_map!(ItemType::ElectPole => 1),
                20.,
                20.,
            ),
        ]
    }

    fn select_recipe(&mut self, factory: &mut Factory, index: usize) -> Result<bool, JsValue> {
        factory.recipe = Some(
            self.get_recipes()
                .get(index)
                .ok_or_else(|| js_str!("recipes index out of bound {:?}", index))?
                .clone(),
        );
        Ok(true)
    }

    fn power_sink(&self) -> bool {
        true
    }

    serialize_impl!();
}
