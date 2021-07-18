use super::{
    draw_direction_arrow,
    items::ItemType,
    structure::{RotateErr, Structure, StructureDynIter, StructureId},
    DropItem, FactorishState, FrameProcResult, Inventory, InventoryTrait, Position, Recipe,
    Rotation, TempEnt, COAL_POWER, TILE_SIZE,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

const FUEL_CAPACITY: usize = 10;

#[derive(Serialize, Deserialize)]
pub(crate) struct OreMine {
    position: Position,
    rotation: Rotation,
    progress: f64,
    power: f64,
    max_power: f64,
    recipe: Option<Recipe>,
    input_inventory: Inventory,
    output_structure: Option<StructureId>,
    #[serde(skip)]
    digging: bool,
}

impl OreMine {
    pub(crate) fn new(x: i32, y: i32, rotation: Rotation) -> Self {
        OreMine {
            position: Position { x, y },
            rotation,
            progress: 0.,
            power: 25., // TODO: Have some initial energy for debugging, should be zero
            max_power: 25.,
            recipe: None,
            input_inventory: Inventory::new(),
            output_structure: None,
            digging: false,
        }
    }

    fn on_construction_common(
        &mut self,
        other_id: StructureId,
        other: &dyn Structure,
        construct: bool,
    ) -> Result<(), JsValue> {
        let output_position = self.position.add(self.rotation.delta());
        if *other.position() == output_position {
            self.output_structure = if construct { Some(other_id) } else { None };
            console_log!(
                "OreMine{:?}: {} output_structure {:?}",
                self.position,
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

    fn position(&self) -> &Position {
        &self.position
    }

    fn draw(
        &self,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        is_toolbar: bool,
    ) -> Result<(), JsValue> {
        let (x, y) = (
            self.position.x as f64 * TILE_SIZE,
            self.position.y as f64 * TILE_SIZE,
        );
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
                draw_direction_arrow((x, y), &self.rotation, state, context)?;
                if !is_toolbar {
                    crate::draw_fuel_alarm!(self, state, context);
                }
            }
            _ => (),
        }

        Ok(())
    }

    fn desc(&self, state: &FactorishState) -> String {
        let tile = &state.board
            [self.position.x as usize + self.position.y as usize * state.width as usize];
        if let Some(_recipe) = &self.recipe {
            // Progress bar
            format!("{}{}{}{}{}",
                format!("Progress: {:.0}%<br>", self.progress * 100.),
                "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
                format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                    self.progress * 100.),
                format!(r#"Power: {:.1}kJ <div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>
                 <div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>"#,
                    self.power,
                    if 0. < self.max_power { (self.power) / self.max_power * 100. } else { 0. }),
                format!("Expected output: {}", if 0 < tile.iron_ore { tile.iron_ore } else if 0 < tile.coal_ore { tile.coal_ore } else { tile.copper_ore }))
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
        state: &mut FactorishState,
        structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        let otile = &state.tile_at(&self.position);
        if otile.is_none() {
            return Ok(FrameProcResult::None);
        }
        let tile = otile.unwrap();

        let mut ret = FrameProcResult::None;

        if self.recipe.is_none() {
            if 0 < tile.iron_ore {
                self.recipe = Some(Recipe::new(
                    HashMap::new(),
                    hash_map!(ItemType::IronOre => 1usize),
                    8.,
                    80.,
                ));
            } else if 0 < tile.coal_ore {
                self.recipe = Some(Recipe::new(
                    HashMap::new(),
                    hash_map!(ItemType::CoalOre => 1usize),
                    8.,
                    80.,
                ));
            } else if 0 < tile.copper_ore {
                self.recipe = Some(Recipe::new(
                    HashMap::new(),
                    hash_map!(ItemType::CopperOre => 1usize),
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
            if let Some(amount) = self.input_inventory.get_mut(&ItemType::CoalOre) {
                if 0 < *amount && self.power == 0. {
                    self.input_inventory.remove_item(&ItemType::CoalOre);
                    self.power += COAL_POWER;
                    self.max_power = self.max_power.max(self.power);
                    ret = FrameProcResult::InventoryChanged(self.position);
                }
            }

            let output = |state: &mut FactorishState, item, position: &Position| {
                if let Ok(val) = if let Some(tile) = state.tile_at_mut(&position) {
                    match item {
                        ItemType::IronOre => Ok(&mut tile.iron_ore),
                        ItemType::CoalOre => Ok(&mut tile.coal_ore),
                        ItemType::CopperOre => Ok(&mut tile.copper_ore),
                        _ => Err(()),
                    }
                } else {
                    Err(())
                } {
                    if 0 < *val {
                        *val -= 1;
                        Ok(*val)
                    } else {
                        Err(())
                    }
                } else {
                    Err(())
                }
            };

            // Proceed only if we have sufficient energy in the buffer.
            let progress = (self.power / recipe.power_cost)
                .min(1. / recipe.recipe_time)
                .min(1. - self.progress);
            if state.rng.next() < progress * 5. {
                state
                    .temp_ents
                    .push(TempEnt::new(&mut state.rng, self.position));
            }
            if 1. <= self.progress + progress {
                let output_position = self.position.add(self.rotation.delta());
                if let Some(structure) = self
                    .output_structure
                    .map(|id| structures.get_mut(id))
                    .flatten()
                {
                    let mut it = recipe.output.iter();
                    if let Some(item) = it.next() {
                        // Check whether we can input first
                        if structure.can_input(item.0) {
                            if let Ok(val) = output(state, *item.0, &self.position) {
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
                        } else {
                            // Output is blocked, wait
                            self.digging = false;
                        }
                    }
                    if !structure.movable() {
                        return Ok(FrameProcResult::None);
                    }
                }
                if !state.hit_check(output_position.x, output_position.y, None) {
                    // let dest_tile = state.board[dx as usize + dy as usize * state.width as usize];
                    let mut it = recipe.output.iter();
                    if let Some(item) = it.next() {
                        assert!(it.next().is_none());
                        if let Err(_code) =
                            state.new_object(output_position.x, output_position.y, *item.0)
                        {
                            // console_log!("Failed to create object: {:?}", code);
                        } else if let Ok(val) = output(state, *item.0, &self.position) {
                            if val == 0 {
                                self.recipe = None;
                            }
                            self.progress = 0.;
                        }
                    } else {
                        return Err(());
                    }
                    self.progress = 0.;
                }
            } else {
                self.progress += progress;
                self.power -= progress * recipe.power_cost;
                self.digging = 0. < progress;
            }
        } else {
            self.digging = false;
        }
        Ok(ret)
    }

    fn rotate(&mut self, others: &StructureDynIter) -> Result<(), RotateErr> {
        self.rotation = self.rotation.next();
        for (id, s) in others.dyn_iter_id() {
            self.on_construction_common(id, s, true)
                .map_err(|e| RotateErr::Other(e))?;
        }
        Ok(())
    }

    fn set_rotation(&mut self, rotation: &Rotation) -> Result<(), ()> {
        self.rotation = *rotation;
        Ok(())
    }

    fn input(&mut self, item: &DropItem) -> Result<(), JsValue> {
        // Fuels are always welcome.
        if item.type_ == ItemType::CoalOre
            && self.input_inventory.count_item(&ItemType::CoalOre) < FUEL_CAPACITY
        {
            self.input_inventory.add_item(&ItemType::CoalOre);
            return Ok(());
        }
        Err(JsValue::from_str("not inputtable to ore mine"))
    }

    fn can_input(&self, item_type: &ItemType) -> bool {
        *item_type == ItemType::CoalOre
            && self.input_inventory.count_item(&ItemType::CoalOre) < FUEL_CAPACITY
    }

    fn on_construction(
        &mut self,
        other_id: StructureId,
        other: &dyn Structure,
        construct: bool,
    ) -> Result<(), JsValue> {
        self.on_construction_common(other_id, other, construct)
    }

    fn on_construction_self(
        &mut self,
        _self_id: StructureId,
        others: &StructureDynIter,
        construct: bool,
    ) -> Result<(), JsValue> {
        for (id, s) in others.dyn_iter_id() {
            self.on_construction_common(id, s, construct)?;
        }
        Ok(())
    }

    fn burner_inventory(&self) -> Option<&Inventory> {
        Some(&self.input_inventory)
    }

    fn add_burner_inventory(&mut self, item_type: &ItemType, amount: isize) -> isize {
        if amount < 0 {
            let existing = self.input_inventory.count_item(item_type);
            let removed = existing.min((-amount) as usize);
            self.input_inventory.remove_items(item_type, removed);
            -(removed as isize)
        } else if *item_type == ItemType::CoalOre {
            let add_amount = amount.min(
                (FUEL_CAPACITY - self.input_inventory.count_item(&ItemType::CoalOre)) as isize,
            );
            self.input_inventory
                .add_items(item_type, add_amount as usize);
            add_amount
        } else {
            0
        }
    }

    fn burner_energy(&self) -> Option<(f64, f64)> {
        Some((self.power, self.max_power))
    }

    fn destroy_inventory(&mut self) -> Inventory {
        // Return the ingredients if it was in the middle of processing a recipe.
        if let Some(recipe) = self.recipe.take() {
            if 0. < self.progress {
                let mut ret = std::mem::take(&mut self.input_inventory);
                ret.merge(recipe.input);
                return ret;
            }
        }
        std::mem::take(&mut self.input_inventory)
    }

    crate::serialize_impl!();
}
