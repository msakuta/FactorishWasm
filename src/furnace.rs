use super::{
    gl::utils::{enable_buffer, Flatten},
    inventory::InventoryType,
    items::item_to_str,
    research::TechnologyTag,
    structure::{
        default_add_inventory, get_powered_progress, Size, Structure, StructureDynIter,
        StructureId, RECIPE_CAPACITY_MULTIPLIER,
    },
    DropItem, FactorishState, FrameProcResult, Inventory, InventoryTrait, ItemType, Position,
    Recipe, TempEnt, COAL_POWER, TILE_SIZE,
};
use cgmath::{Matrix3, Matrix4, Vector2, Vector3};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, WebGlRenderingContext as GL};

const FUEL_CAPACITY: usize = 10;

/// A list of fixed recipes, because dynamic get_recipes() can only return a Vec.
pub(crate) static RECIPES: Lazy<[Recipe; 3]> = Lazy::new(|| {
    [
        Recipe::new(
            hash_map!(ItemType::IronOre => 1usize),
            hash_map!(ItemType::IronPlate => 1usize),
            20.,
            50.,
        ),
        Recipe::new(
            hash_map!(ItemType::CopperOre => 1usize),
            hash_map!(ItemType::CopperPlate => 1usize),
            20.,
            50.,
        ),
        Recipe::new_with_requires(
            hash_map!(ItemType::IronPlate => 5usize),
            hash_map!(ItemType::SteelPlate => 1usize),
            100.,
            250.,
            hash_set!(TechnologyTag::SteelWorks),
        ),
    ]
});

#[derive(Serialize, Deserialize)]
pub(crate) struct Furnace {
    position: Position,
    #[serde(default)]
    burner_inventory: Inventory,
    input_inventory: Inventory,
    output_inventory: Inventory,
    progress: Option<f64>,
    power: f64,
    max_power: f64,
    recipe: Option<Recipe>,
}

impl Furnace {
    pub(crate) fn new(position: &Position) -> Self {
        Furnace {
            position: *position,
            burner_inventory: Inventory::new(),
            input_inventory: Inventory::new(),
            output_inventory: Inventory::new(),
            progress: None,
            power: 20.,
            max_power: 20.,
            recipe: None,
        }
    }
}

impl Structure for Furnace {
    fn name(&self) -> &str {
        "Furnace"
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
        if depth != 0 {
            return Ok(());
        };
        let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
        let source_scale = if is_toolbar { 2. } else { 1. };
        match state.image_furnace.as_ref() {
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
                    TILE_SIZE * source_scale,
                    TILE_SIZE * source_scale,
                    x,
                    y,
                    TILE_SIZE,
                    TILE_SIZE,
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
        let (x, y) = (
            self.position.x as f32 + state.viewport.x as f32,
            self.position.y as f32 + state.viewport.y as f32,
        );
        let texture = &state.assets.tex_furnace;
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
            (state.get_world_transform()?
                * Matrix4::from_scale(2.)
                * Matrix4::from_translation(Vector3::new(x, y, 0.))
                * Matrix4::from_scale(2.))
            .flatten(),
        );
        gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

        if !is_ghost {
            crate::draw_fuel_alarm_gl_impl!(self, state, gl);
        }

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
        _me: StructureId,
        state: &mut FactorishState,
        _structures: &mut StructureDynIter,
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
            let mut ret = FrameProcResult::None;
            // First, check if we need to refill the energy buffer in order to continue the current work.
            if self.burner_inventory.get(&ItemType::CoalOre).is_some() {
                // Refill the energy from the fuel
                if self.power < recipe.power_cost {
                    self.power += COAL_POWER;
                    self.max_power = self.power;
                    self.burner_inventory.remove_item(&ItemType::CoalOre);
                    ret = FrameProcResult::InventoryChanged(self.position);
                }
            }

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
                let progress = get_powered_progress(self.power, prev_progress, recipe);
                if state.rng.next() < progress * 10. {
                    let position = self.position.to_f64();
                    state.temp_ents.push(TempEnt::new_float(
                        &mut state.rng,
                        (position.x + 1.4, position.y),
                    ));
                }
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
        // Fuels are always welcome.
        if o.type_ == ItemType::CoalOre
            && self.burner_inventory.count_item(&ItemType::CoalOre) < FUEL_CAPACITY
        {
            self.burner_inventory.add_item(&ItemType::CoalOre);
            return Ok(());
        }

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

        if 0 < default_add_inventory(self, InventoryType::Input, &o.type_, 1) {
            Ok(())
        } else {
            Err(JsValue::from_str("Item is not part of recipe"))
        }
    }

    fn can_input(&self, item_type: &ItemType) -> bool {
        if *item_type == ItemType::CoalOre
            && self.burner_inventory.count_item(item_type) < FUEL_CAPACITY
        {
            return true;
        }
        if let Some(recipe) = &self.recipe {
            recipe
                .input
                .get(item_type)
                .map(|count| {
                    self.input_inventory.count_item(item_type) < *count * RECIPE_CAPACITY_MULTIPLIER
                })
                .unwrap_or(false)
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

    fn add_inventory(
        &mut self,
        inventory_type: InventoryType,
        item_type: &ItemType,
        count: isize,
    ) -> isize {
        if inventory_type != InventoryType::Burner {
            return default_add_inventory(self, inventory_type, item_type, count);
        }
        if count < 0 {
            let existing = self.burner_inventory.count_item(item_type);
            let removed = existing.min((-count) as usize);
            self.burner_inventory.remove_items(item_type, removed);
            -(removed as isize)
        } else if *item_type == ItemType::CoalOre {
            let add_amount = count.min(
                (FUEL_CAPACITY - self.burner_inventory.count_item(&ItemType::CoalOre)) as isize,
            );
            self.burner_inventory
                .add_items(item_type, add_amount as usize);
            add_amount as isize
        } else {
            0
        }
    }

    fn burner_energy(&self) -> Option<(f64, f64)> {
        Some((self.power, self.max_power))
    }

    fn inventory(&self, invtype: InventoryType) -> Option<&Inventory> {
        Some(match invtype {
            InventoryType::Burner => &self.burner_inventory,
            InventoryType::Input => &self.input_inventory,
            InventoryType::Output => &self.output_inventory,
            _ => return None,
        })
    }

    fn inventory_mut(&mut self, invtype: InventoryType) -> Option<&mut Inventory> {
        Some(match invtype {
            InventoryType::Burner => &mut self.burner_inventory,
            InventoryType::Input => &mut self.input_inventory,
            InventoryType::Output => &mut self.output_inventory,
            _ => return None,
        })
    }

    fn destroy_inventory(&mut self) -> Inventory {
        let mut ret = std::mem::take(&mut self.input_inventory);
        ret.merge(std::mem::take(&mut self.output_inventory));
        ret.merge(std::mem::take(&mut self.burner_inventory));
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

    fn auto_recipe(&self) -> bool {
        true
    }

    fn get_selected_recipe(&self) -> Option<&Recipe> {
        self.recipe.as_ref()
    }

    fn get_progress(&self) -> Option<f64> {
        self.progress
    }

    fn serialize(&self) -> serde_json::Result<serde_json::Value> {
        serde_json::to_value(self)
    }
}
