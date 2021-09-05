use super::{
    burner::Burner,
    factory::Factory,
    gl::utils::{enable_buffer, Flatten},
    serialize_impl,
    structure::{
        Energy, FrameProcResult, Structure, StructureBundle, StructureComponents, StructureDynIter,
        StructureId,
    },
    FactorishState, Inventory, InventoryTrait, ItemType, Position, Recipe,
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
        Recipe::new(
            hash_map!(ItemType::IronPlate => 5usize),
            hash_map!(ItemType::SteelPlate => 1usize),
            100.,
            250.,
        ),
    ]
});

#[derive(Serialize, Deserialize)]
pub(crate) struct Furnace {}

impl Furnace {
    pub(crate) fn new(position: &Position) -> StructureBundle {
        StructureBundle::new(
            Box::new(Furnace {}),
            Some(*position),
            None,
            Some(Burner {
                inventory: Inventory::new(),
                capacity: FUEL_CAPACITY,
            }),
            Some(Energy {
                value: 0.,
                max: 100.,
            }),
            Some(Factory::new()),
            vec![],
        )
    }
}

impl Structure for Furnace {
    fn name(&self) -> &'static str {
        "Furnace"
    }

    fn draw(
        &self,
        components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };

        let (x, y) = if let Some(position) = &components.position {
            (position.x as f64 * 32., position.y as f64 * 32.)
        } else {
            (0., 0.)
        };
        match state.image_furnace.as_ref() {
            Some(img) => {
                let sx = if let Some((energy, factory)) =
                    components.energy.as_ref().zip(components.factory.as_ref())
                {
                    if factory.progress.is_some() && 0. < energy.value {
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

    fn draw_gl(
        &self,
        components: &StructureComponents,
        state: &FactorishState,
        gl: &GL,
        depth: i32,
        is_ghost: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        let position = components
            .position
            .ok_or_else(|| js_str!("Furnace without Position"))?;
        let factory = components
            .factory
            .as_ref()
            .ok_or_else(|| js_str!("Furnace without Factory"))?;
        let energy = components
            .energy
            .as_ref()
            .ok_or_else(|| js_str!("Furnace without Energy"))?;
        let shader = state
            .assets
            .textured_shader
            .as_ref()
            .ok_or_else(|| js_str!("Shader not found"))?;
        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), if is_ghost { 0.5 } else { 1. });
        let (x, y) = (
            position.x as f32 + state.viewport.x as f32,
            position.y as f32 + state.viewport.y as f32,
        );
        let texture = &state.assets.tex_furnace;
        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(texture));
        let sx = if factory.progress.is_some() && 0. < energy.value {
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

        if !is_ghost {
            if factory.recipe.is_some() && energy.value == 0. {
                crate::gl::draw_fuel_alarm_gl(components, state, gl)?;
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
            if factory.recipe.is_some() {
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
            // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
            // "Outputs: <br>" +
            // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
            } else {
                String::from("No recipe")
            },
            format!("Input Items: <br>{}", factory.input_inventory.describe()),
            format!("Output Items: <br>{}", factory.output_inventory.describe())
        )
    }

    fn frame_proc(
        &mut self,
        _me: StructureId,
        components: &mut StructureComponents,
        _state: &mut FactorishState,
        _structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        let factory = components.factory.as_mut().ok_or_else(|| ())?;
        if factory.recipe.is_none() {
            factory.recipe = RECIPES
                .iter()
                .find(|recipe| {
                    recipe
                        .input
                        .iter()
                        .all(|(type_, count)| *count <= factory.input_inventory.count_item(&type_))
                })
                .cloned();
        } else if factory.progress.is_none() {
            factory.recipe = None;
        }
        Ok(FrameProcResult::None)
    }

    fn get_recipes(&self) -> std::borrow::Cow<[Recipe]> {
        std::borrow::Cow::from(&RECIPES[..])
    }

    fn auto_recipe(&self) -> bool {
        true
    }

    serialize_impl!();
}
