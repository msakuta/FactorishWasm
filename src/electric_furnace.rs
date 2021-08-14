use super::{
    furnace::RECIPES,
    gl::{
        draw_electricity_alarm_gl,
        utils::{enable_buffer, Flatten},
    },
    items::item_to_str,
    serialize_impl,
    structure::{Structure, StructureDynIter, StructureId},
    DropItem, FactorishState, FrameProcResult, Inventory, InventoryTrait, ItemType, Position,
    Recipe, TILE_SIZE,
};
use cgmath::{Matrix3, Matrix4, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, WebGlRenderingContext as GL};

#[derive(Serialize, Deserialize)]
pub(crate) struct ElectricFurnace {
    position: Position,
    input_inventory: Inventory,
    output_inventory: Inventory,
    progress: Option<f64>,
    power: f64,
    max_power: f64,
    recipe: Option<Recipe>,
}

impl ElectricFurnace {
    pub(crate) fn new(position: &Position) -> Self {
        ElectricFurnace {
            position: *position,
            input_inventory: Inventory::new(),
            output_inventory: Inventory::new(),
            progress: None,
            power: 20.,
            max_power: 20.,
            recipe: None,
        }
    }
}

impl Structure for ElectricFurnace {
    fn name(&self) -> &str {
        "Electric Furnace"
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
        let (x, y) = (
            self.position.x as f64 * TILE_SIZE,
            self.position.y as f64 * TILE_SIZE,
        );
        match state.image_electric_furnace.as_ref() {
            Some(img) => {
                let sx = if self.progress.is_some() && 0. < self.power {
                    (((state.sim_time * 5.) as isize) % 2 + 1) as f64 * TILE_SIZE
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
        if depth == 2 {
            if !is_ghost && self.recipe.is_some() && self.power == 0. {
                draw_electricity_alarm_gl((x, y), state, gl)?;
            }
        }
        if depth != 0 {
            return Ok(());
        };
        let shader = state
            .assets
            .textured_shader
            .as_ref()
            .ok_or_else(|| js_str!("Shader not found"))?;
        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), if is_ghost { 0.5 } else { 1. });
        let texture = &state.assets.tex_electric_furnace;
        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(texture));
        let sx = if self.progress.is_some() && 0. < self.power {
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

        enable_buffer(&gl, &state.assets.screen_buffer, 2, shader.vertex_position);
        gl.uniform_matrix4fv_with_f32_array(
            shader.transform_loc.as_ref(),
            false,
            &(state.get_world_transform()?
                * Matrix4::from_scale(2.)
                * Matrix4::from_translation(Vector3::new(x, y, 0.)))
            .flatten(),
        );
        gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

        Ok(())
    }

    fn desc(&self, _state: &FactorishState) -> String {
        format!(
            "{}<br>{}{}",
            if self.recipe.is_some() {
                // Progress bar
                format!("{}{}{}{}",
                    format!("Progress: {:.0}%<br>", self.progress.unwrap_or(0.) * 100.),
                    "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
                    format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                        self.progress.unwrap_or(0.) * 100.),
                    format!(r#"Power: {:.1}kJ <div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>
                    <div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>"#,
                    self.power,
                    if 0. < self.max_power { (self.power) / self.max_power * 100. } else { 0. }),
                    )
            // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
            // "Outputs: <br>" +
            // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
            } else {
                String::from("No recipe")
            },
            format!("Input Items: <br>{}", self.input_inventory.describe()),
            format!("Output Items: <br>{}", self.output_inventory.describe())
        )
    }

    fn frame_proc(
        &mut self,
        me: StructureId,
        state: &mut FactorishState,
        structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        if self.recipe.is_none() {
            self.recipe = RECIPES
                .iter()
                .find(|recipe| {
                    recipe
                        .input
                        .iter()
                        .all(|(type_, count)| *count <= self.input_inventory.count_item(&type_))
                })
                .cloned();
        }
        if let Some(recipe) = &self.recipe {
            if self.power < recipe.power_cost {
                let mut accumulated = 0.;
                for network in &state
                    .power_networks
                    .iter()
                    .find(|network| network.sinks.contains(&me))
                {
                    for id in network.sources.iter() {
                        if let Some(source) = structures.get_mut(*id) {
                            let demand = self.max_power - self.power - accumulated;
                            if let Some(energy) = source.power_outlet(demand) {
                                accumulated += energy;
                                // console_log!("draining {:?}kJ of energy with {:?} demand, from {:?}, accumulated {:?}", energy, demand, structure.name(), accumulated);
                            }
                        }
                    }
                }
                self.power += accumulated;
            }

            let mut ret = FrameProcResult::None;

            if self.progress.is_none() {
                // First, check if we have enough ingredients to finish this recipe.
                // If we do, consume the ingredients and start the progress timer.
                // We can't start as soon as the recipe is set because we may not have enough ingredients
                // at the point we set the recipe.
                if recipe
                    .input
                    .iter()
                    .map(|(item, count)| count <= &self.input_inventory.count_item(item))
                    .all(|b| b)
                {
                    for (item, count) in &recipe.input {
                        self.input_inventory.remove_items(item, *count);
                    }
                    self.progress = Some(0.);
                    ret = FrameProcResult::InventoryChanged(self.position);
                } else {
                    self.recipe = None;
                    return Ok(FrameProcResult::None); // Return here to avoid borrow checker
                }
            }

            if let Some(prev_progress) = self.progress {
                // Proceed only if we have sufficient energy in the buffer.
                let progress = (self.power / recipe.power_cost)
                    .min(1. / recipe.recipe_time)
                    .min(1.);
                if 1. <= prev_progress + progress {
                    self.progress = None;

                    // Produce outputs into inventory
                    for output_item in &recipe.output {
                        self.output_inventory.add_item(&output_item.0);
                    }
                    return Ok(FrameProcResult::InventoryChanged(self.position));
                } else {
                    self.progress = Some(prev_progress + progress);
                    self.power -= progress * recipe.power_cost;
                }
            }
            return Ok(ret);
        }
        Ok(FrameProcResult::None)
    }

    fn input(&mut self, o: &DropItem) -> Result<(), JsValue> {
        if self.recipe.is_none() {
            if let Some(recipe) = RECIPES
                .iter()
                .find(|recipe| recipe.input.contains_key(&o.type_))
            {
                self.recipe = Some(recipe.clone());
            } else {
                return Err(JsValue::from_str(&format!(
                    "Cannot smelt {}",
                    item_to_str(&o.type_)
                )));
            }
        }

        if let Some(recipe) = &self.recipe {
            if 0 < recipe.input.count_item(&o.type_) || 0 < recipe.output.count_item(&o.type_) {
                self.input_inventory.add_item(&o.type_);
                return Ok(());
            } else {
                return Err(JsValue::from_str("Item is not part of recipe"));
            }
        }
        Err(JsValue::from_str("Recipe is not initialized"))
    }

    fn can_input(&self, item_type: &ItemType) -> bool {
        if let Some(recipe) = &self.recipe {
            recipe.input.get(item_type).is_some()
        } else {
            RECIPES
                .iter()
                .any(|recipe| recipe.input.contains_key(item_type))
        }
    }

    fn can_output(&self, _structures: &StructureDynIter) -> Inventory {
        self.output_inventory.clone()
    }

    fn output(&mut self, _state: &mut FactorishState, item_type: &ItemType) -> Result<(), ()> {
        if self.output_inventory.remove_item(item_type) {
            Ok(())
        } else {
            Err(())
        }
    }

    fn inventory(&self, is_input: bool) -> Option<&Inventory> {
        Some(if is_input {
            &self.input_inventory
        } else {
            &self.output_inventory
        })
    }

    fn inventory_mut(&mut self, is_input: bool) -> Option<&mut Inventory> {
        Some(if is_input {
            &mut self.input_inventory
        } else {
            &mut self.output_inventory
        })
    }

    fn destroy_inventory(&mut self) -> Inventory {
        let mut ret = std::mem::take(&mut self.input_inventory);
        ret.merge(std::mem::take(&mut self.output_inventory));
        // Return the ingredients if it was in the middle of processing a recipe.
        if let Some(mut recipe) = self.recipe.take() {
            if self.progress.is_some() {
                ret.merge(std::mem::take(&mut recipe.input));
            }
        }
        ret
    }

    fn get_recipes(&self) -> std::borrow::Cow<[Recipe]> {
        std::borrow::Cow::from(&RECIPES[..])
    }

    fn get_selected_recipe(&self) -> Option<&Recipe> {
        self.recipe.as_ref()
    }

    fn power_sink(&self) -> bool {
        true
    }

    serialize_impl!();
}
