use crate::structure::Size;

use super::{
    draw_direction_arrow,
    drop_items::hit_check,
    gl::{
        draw_direction_arrow_gl,
        utils::{enable_buffer, Flatten},
    },
    inventory::{Inventory, InventoryTrait, InventoryType},
    items::ItemType,
    structure::{get_powered_progress, RotateErr, Structure, StructureDynIter, StructureId},
    DropItem, FactorishState, FrameProcResult, Position, Recipe, Rotation, TempEnt, COAL_POWER,
    TILE_SIZE,
};
use cgmath::{Matrix3, Matrix4, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, WebGlRenderingContext as GL};

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
    #[serde(skip)]
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
        let output_position = self.output_pos();
        if other.bounding_box().intersects_position(output_position) {
            self.output_structure = if construct { Some(other_id) } else { None };
        }
        Ok(())
    }

    /// Get output port position of products, in float coordinates
    fn output_pos(&self) -> Position {
        let pos = self.output_port_pos();
        let direction_delta = self.rotation.delta();
        Position {
            x: pos.x + direction_delta.0,
            y: pos.y + direction_delta.1,
        }
    }

    /// Get output port position of products, in float coordinates
    fn output_port_pos(&self) -> Position {
        let x = self.position.x;
        let y = self.position.y;
        let x = if matches!(self.rotation, Rotation::Right | Rotation::Bottom) {
            x + 1
        } else {
            x
        };
        let y = if matches!(self.rotation, Rotation::Left | Rotation::Bottom) {
            y + 1
        } else {
            y
        };
        Position { x, y }
    }
}

impl Structure for OreMine {
    fn name(&self) -> &str {
        "Ore Mine"
    }

    fn position(&self) -> &Position {
        &self.position
    }

    fn size(&self) -> Size {
        Size::new(2, 2)
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
        let source_scale = if is_toolbar { 2. } else { 1. };
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
                            TILE_SIZE * source_scale,
                            TILE_SIZE * source_scale,
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
            }
            _ => (),
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
        let (x, y) = (
            self.position.x as f32 + state.viewport.x as f32,
            self.position.y as f32 + state.viewport.y as f32,
        );
        match depth {
            0 => {
                let shader = state
                    .assets
                    .textured_shader
                    .as_ref()
                    .ok_or_else(|| js_str!("Shader not found"))?;
                gl.use_program(Some(&shader.program));
                gl.uniform1f(shader.alpha_loc.as_ref(), if is_ghost { 0.5 } else { 1. });

                enable_buffer(&gl, &state.assets.screen_buffer, 2, shader.vertex_position);

                gl.active_texture(GL::TEXTURE0);

                let draw_exit = || -> Result<(), JsValue> {
                    let port_pos = self.output_port_pos().to_f64() + state.viewport.offset_f64();
                    let port_pos_f32 = port_pos
                        .cast::<f32>()
                        .ok_or_else(|| js_str!("Failed to cast port position"))?;
                    gl.uniform_matrix4fv_with_f32_array(
                        shader.transform_loc.as_ref(),
                        false,
                        (state.get_world_transform()?
                            * Matrix4::from_scale(2.)
                            * Matrix4::from_translation(port_pos_f32.extend(0.)))
                        .flatten(),
                    );

                    gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_ore_mine_exit));
                    let sx = self.rotation.angle_4() as f32 / 4.;
                    gl.uniform_matrix3fv_with_f32_array(
                        shader.tex_transform_loc.as_ref(),
                        false,
                        (Matrix3::from_translation(Vector2::new(sx, 0.))
                            * Matrix3::from_nonuniform_scale(1. / 4., 1.))
                        .flatten(),
                    );
                    gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
                    Ok(())
                };

                if self.rotation != Rotation::Bottom {
                    draw_exit()?;
                }

                gl.uniform_matrix4fv_with_f32_array(
                    shader.transform_loc.as_ref(),
                    false,
                    (state.get_world_transform()?
                        * Matrix4::from_scale(2.)
                        * Matrix4::from_translation(Vector3::new(x, y, 0.))
                        * Matrix4::from_scale(2.))
                    .flatten(),
                );
                gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_ore_mine));
                let sx = if self.digging {
                    (((state.sim_time * 5.) as isize) % 2 + 1) as f32 / 3.
                } else {
                    0.
                };
                gl.uniform_matrix3fv_with_f32_array(
                    shader.tex_transform_loc.as_ref(),
                    false,
                    (Matrix3::from_translation(Vector2::new(sx, 0.))
                        * Matrix3::from_nonuniform_scale(1. / 3., 1.))
                    .flatten(),
                );

                gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

                if self.rotation == Rotation::Bottom {
                    draw_exit()?;
                }
            }
            2 => {
                if state.alt_mode {
                    let output_pos = self.output_port_pos().to_f64() + state.viewport.offset_f64();
                    let output_pos_f32 = output_pos
                        .cast::<f32>()
                        .ok_or_else(|| js_str!("Failed to cast port position"))?;
                    draw_direction_arrow_gl(output_pos_f32, &self.rotation, state, gl)?;
                }
                if !is_ghost {
                    crate::draw_fuel_alarm_gl_impl!(self, state, gl);
                }
            }
            _ => (),
        }
        Ok(())
    }

    fn desc(&self, state: &FactorishState) -> String {
        let Some(_recipe) = &self.recipe else {
            return String::from("Empty");
        };

        let expected_outputs = self
            .bounding_box()
            .iter_tiles()
            .filter_map(|p| state.tile_at(&p).and_then(|t| t.ore))
            .fold(
                BTreeMap::new(),
                |mut acc: BTreeMap<crate::Ore, u32>, cur| {
                    *acc.entry(cur.0).or_default() += cur.1;
                    acc
                },
            );

        let expected_output_fmt = expected_outputs
            .iter()
            .map(|(ore, amount)| format!("&nbsp;&nbsp;{:?}: {}<br>", ore, amount))
            .fold("".to_string(), |acc, cur| acc + &cur);

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
            format!("Expected output:<br>{}", if expected_output_fmt.is_empty() { "None" } else { &expected_output_fmt }))
        // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
        // "Outputs: <br>" +
        // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
    }

    fn frame_proc(
        &mut self,
        _me: StructureId,
        state: &mut FactorishState,
        structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        let mut ret = FrameProcResult::None;

        if self.recipe.is_none() {
            for tile in self
                .bounding_box()
                .iter_tiles()
                .filter_map(|p| state.tile_at(&p))
            {
                if let Some(item_type) = tile.get_ore_type() {
                    self.recipe = Some(Recipe::new(
                        HashMap::new(),
                        hash_map!(item_type => 1usize),
                        8.,
                        80.,
                    ));
                }
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

            let recipe_ore = recipe.output.iter().next().map(|(ore, _)| ore).ok_or(())?;
            let bbox = self.bounding_box();

            let remove_ore_from_tile = |state: &mut FactorishState, pos: &Position| {
                let tile = state.tile_at_mut(&pos)?;
                let ore = tile.ore.as_mut()?;
                if ore.0.to_item_type() != *recipe_ore {
                    return None;
                }
                let val = &mut ore.1;
                if 0 < *val {
                    *val -= 1;
                    let ret = *val;
                    if ret == 0 {
                        tile.ore = None;
                    }
                    Some(ret)
                } else {
                    None
                }
            };

            let remove_ore_from_tiles = |state: &mut FactorishState| {
                for pos in bbox.iter_tiles() {
                    if let Some(val) = remove_ore_from_tile(state, &pos) {
                        return Some(val);
                    }
                }
                None
            };

            // Proceed only if we have sufficient energy in the buffer.
            let progress = get_powered_progress(self.power, self.progress, recipe);
            if 1. <= self.progress + progress {
                let output_position = self.output_pos();
                let output_pixels = output_position.to_pixels();
                if let Some(structure) = self.output_structure.and_then(|id| structures.get_mut(id))
                {
                    let mut it = recipe.output.iter();
                    if let Some(item) = it.next() {
                        // Check whether we can input first
                        if structure.can_input(item.0) {
                            if let Some(val) = remove_ore_from_tiles(state) {
                                structure
                                    .input(&DropItem {
                                        type_: *item.0,
                                        x: output_pixels.x as f64,
                                        y: output_pixels.y as f64,
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
                    if !structure.movable() {
                        self.digging = false;
                        return Ok(FrameProcResult::None);
                    }
                }
                let drop_x = output_pixels.x as f64 + TILE_SIZE / 2.;
                let drop_y = output_pixels.y as f64 + TILE_SIZE / 2.;
                if !hit_check(&state.drop_items, drop_x, drop_y, None)
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
                        } else if let Some(val) = remove_ore_from_tiles(state) {
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
                self.power -= progress * recipe.power_cost;
                self.digging = 0. < progress;
            }

            // Show smoke if there was some progress
            if state.rng.next() < progress * 5. {
                state.temp_ents.push(TempEnt::new_float(
                    &mut state.rng,
                    (self.position.x as f64 + 1., self.position.y as f64 + 0.5),
                ));
            }
        } else {
            self.digging = false;
        }
        Ok(ret)
    }

    fn rotate(
        &mut self,
        _state: &mut FactorishState,
        others: &StructureDynIter,
    ) -> Result<(), RotateErr> {
        self.rotation = self.rotation.next();
        self.output_structure = None;
        for (id, s) in others.dyn_iter_id() {
            self.on_construction_common(id, s, true)
                .map_err(RotateErr::Other)?;
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
        _others: &StructureDynIter,
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

    fn add_inventory(
        &mut self,
        inventory_type: InventoryType,
        item_type: &ItemType,
        amount: isize,
    ) -> isize {
        if inventory_type != InventoryType::Burner {
            return 0;
        }
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

    fn inventory(&self, invtype: InventoryType) -> Option<&Inventory> {
        Some(match invtype {
            InventoryType::Burner => &self.input_inventory,
            _ => return None,
        })
    }

    fn inventory_mut(&mut self, invtype: InventoryType) -> Option<&mut Inventory> {
        Some(match invtype {
            InventoryType::Burner => &mut self.input_inventory,
            _ => return None,
        })
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
