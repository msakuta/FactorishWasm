use super::{
    burner::Burner,
    draw_direction_arrow,
    items::ItemType,
    structure::{
        Energy, Structure, StructureBundle, StructureComponents, StructureDynIter, StructureId,
    },
    DropItem, FactorishState, FrameProcResult, Inventory, Position, Recipe, RotateErr, Rotation,
    TempEnt, TILE_SIZE, TILE_SIZE_I,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

const FUEL_CAPACITY: usize = 10;

#[derive(Serialize, Deserialize)]
pub(crate) struct OreMine {
    progress: f64,
    recipe: Option<Recipe>,
    output_structure: Option<StructureId>,
    #[serde(skip)]
    digging: bool,
}

impl OreMine {
    pub(crate) fn new(x: i32, y: i32, rotation: Rotation) -> StructureBundle {
        StructureBundle::new(
            Box::new(OreMine {
                progress: 0.,
                recipe: None,
                output_structure: None,
                digging: false,
            }),
            Some(Position { x, y }),
            Some(rotation),
            Some(Burner {
                inventory: Inventory::new(),
                capacity: FUEL_CAPACITY,
            }),
            Some(Energy {
                value: 25.,
                max: 100.,
            }),
            None,
            vec![],
        )
    }

    fn on_construction_common(
        &mut self,
        components: &StructureComponents,
        other_id: StructureId,
        other: &StructureBundle,
        construct: bool,
    ) -> Result<(), JsValue> {
        let position = components
            .position
            .ok_or_else(|| js_str!("OreMine without Position"))?;
        let rotation = components
            .rotation
            .ok_or_else(|| js_str!("OreMine without Rotation"))?;
        let output_position = position.add(rotation.delta());
        let other_position = other
            .components
            .position
            .ok_or_else(|| js_str!("Other without Position"))?;
        if other_position == output_position {
            self.output_structure = if construct { Some(other_id) } else { None };
            console_log!(
                "OreMine{:?}: {} output_structure {:?}",
                position,
                if construct { "set" } else { "unset" },
                other_id
            );
        }
        Ok(())
    }
}

impl Structure for OreMine {
    fn name(&self) -> &str {
        "Ore Mine"
    }

    fn draw(
        &self,
        components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        let (x, y) = if let Some(position) = components.position.as_ref() {
            (position.x as f64 * TILE_SIZE, position.y as f64 * TILE_SIZE)
        } else {
            (0., 0.)
        };
        match depth {
            0 => match state.image_mine.as_ref() {
                Some(img) => {
                    let sx = if self.digging {
                        (((state.sim_time * 5.) as isize) % 2 + 1) as f64 * TILE_SIZE
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
                None => return Err(JsValue::from_str("mine image not available")),
            },
            2 => {
                draw_direction_arrow(
                    (x, y),
                    &components.rotation.unwrap_or(Rotation::Left),
                    state,
                    context,
                )?;
            }
            _ => (),
        }

        Ok(())
    }

    fn desc(&self, components: &StructureComponents, state: &FactorishState) -> String {
        let (position, energy) =
            if let Some(energy) = components.position.as_ref().zip(components.energy.as_ref()) {
                energy
            } else {
                return "Position or Energy not found".to_string();
            };
        let tile = &state.board[position.x as usize + position.y as usize * state.width as usize];
        if let Some(_recipe) = &self.recipe {
            // Progress bar
            format!("{}{}{}{}{}",
                format!("Progress: {:.0}%<br>", self.progress * 100.),
                "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
                format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                    self.progress * 100.),
                format!(r#"Power: {:.1}kJ <div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>
                 <div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>"#,
                    energy.value,
                    if 0. < energy.max { (energy.value) / energy.max * 100. } else { 0. }),
                format!("Expected output: {}", tile.ore.map(|ore| ore.1).unwrap_or(0)))
        // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
        // "Outputs: <br>" +
        // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
        } else {
            String::from("Empty")
        }
    }

    fn frame_proc(
        &mut self,
        _me: StructureId,
        components: &mut StructureComponents,
        state: &mut FactorishState,
        structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        let position = components.position.as_ref().ok_or(())?;
        let rotation = components.rotation.as_ref().ok_or(())?;
        let energy = components.energy.as_mut().ok_or(())?;

        let otile = &state.tile_at(position);
        if otile.is_none() {
            return Ok(FrameProcResult::None);
        }
        let tile = otile.unwrap();

        let ret = FrameProcResult::None;

        if self.recipe.is_none() {
            if let Some(item_type) = tile.get_ore_type() {
                self.recipe = Some(Recipe::new(
                    HashMap::new(),
                    hash_map!(item_type => 1usize),
                    8.,
                    80.,
                ));
            }
        }
        if let Some(recipe) = &self.recipe {
            // First, check if we need to refill the energy buffer in order to continue the current work.
            // if("Coal Ore" in this.inventory){
            //     var coalPower = 100;
            //     // Refill the energy from the fuel
            //     if(this.power < this.recipe.powerCost){
            //         this.power += coalPower;
            //         this.maxPower = this.power;
            //         this.removeItem("Coal Ore");
            //     }
            // }

            let output = |state: &mut FactorishState, _item, position: &Position| {
                let tile = state.tile_at_mut(&position).ok_or(())?;
                let ore = tile.ore.as_mut().ok_or(())?;
                let val = &mut ore.1;
                if 0 < *val {
                    *val -= 1;
                    let ret = *val;
                    if ret == 0 {
                        tile.ore = None;
                    }
                    Ok(ret)
                } else {
                    Err(())
                }
            };

            // Proceed only if we have sufficient energy in the buffer.
            let progress = (energy.value / recipe.power_cost)
                .min(1. / recipe.recipe_time)
                .min(1. - self.progress);
            if 1. <= self.progress + progress {
                let output_position = position.add(rotation.delta());
                if let Some(structure) = self
                    .output_structure
                    .map(|id| structures.get_mut(id))
                    .flatten()
                {
                    let mut it = recipe.output.iter();
                    if let Some(item) = it.next() {
                        // Check whether we can input first
                        if structure.can_input(item.0) {
                            if let Ok(val) = output(state, *item.0, position) {
                                structure
                                    .input(&DropItem {
                                        id: 0,
                                        type_: *item.0,
                                        x: output_position.x,
                                        y: output_position.y,
                                    })
                                    .map_err(|_| ())?;
                                if val == 0 {
                                    self.recipe = None;
                                }
                                self.progress = 0.;
                                return Ok(FrameProcResult::InventoryChanged(output_position));
                            } else {
                                self.recipe = None;
                                return Err(());
                            };
                        }
                    }
                    if !structure.dynamic.movable() {
                        self.digging = false;
                        return Ok(FrameProcResult::None);
                    }
                }
                let drop_x = output_position.x * TILE_SIZE_I + TILE_SIZE_I / 2;
                let drop_y = output_position.y * TILE_SIZE_I + TILE_SIZE_I / 2;
                if !state.hit_check(drop_x, drop_y, None)
                    && state
                        .tile_at(&output_position)
                        .map(|cell| !cell.water)
                        .unwrap_or(false)
                {
                    // let dest_tile = state.board[dx as usize + dy as usize * state.width as usize];
                    let mut it = recipe.output.iter();
                    if let Some(item) = it.next() {
                        assert!(it.next().is_none());
                        if let Err(_code) = state.new_object(&output_position, *item.0) {
                            // console_log!("Failed to create object: {:?}", code);
                        } else if let Ok(val) = output(state, *item.0, position) {
                            if val == 0 {
                                self.recipe = None;
                            }
                            self.progress = 0.;
                        }
                    } else {
                        return Err(());
                    }
                    self.progress = 0.;
                } else {
                    // Output is blocked
                    self.digging = false;
                    return Ok(FrameProcResult::None);
                }
            } else {
                self.progress += progress;
                energy.value -= progress * recipe.power_cost;
                self.digging = 0. < progress;
            }

            // Show smoke if there was some progress
            if state.rng.next() < progress * 5. {
                state
                    .temp_ents
                    .push(TempEnt::new(&mut state.rng, *position));
            }
        } else {
            self.digging = false;
        }
        Ok(ret)
    }

    fn rotate(
        &mut self,
        components: &mut StructureComponents,
        others: &StructureDynIter,
    ) -> Result<(), RotateErr> {
        if let Some(ref mut rotation) = components.rotation {
            *rotation = rotation.next();
            self.output_structure = None;
            for (id, s) in others.dyn_iter_id() {
                self.on_construction_common(components, id, s, true)
                    .map_err(|e| RotateErr::Other(e))?;
            }
        }
        Ok(())
    }

    fn set_rotation(
        &mut self,
        components: &mut StructureComponents,
        rotation: &Rotation,
    ) -> Result<(), ()> {
        if let Some(ref mut self_rotation) = components.rotation {
            *self_rotation = *rotation;
            Ok(())
        } else {
            Err(())
        }
    }

    fn on_construction(
        &mut self,
        components: &mut StructureComponents,
        other_id: StructureId,
        other: &StructureBundle,
        construct: bool,
    ) -> Result<(), JsValue> {
        self.on_construction_common(components, other_id, other, construct)
    }

    fn on_construction_self(
        &mut self,
        _self_id: StructureId,
        components: &mut StructureComponents,
        others: &StructureDynIter,
        construct: bool,
    ) -> Result<(), JsValue> {
        for (id, s) in others.dyn_iter_id() {
            self.on_construction_common(components, id, s, construct)?;
        }
        Ok(())
    }

    crate::serialize_impl!();
}
