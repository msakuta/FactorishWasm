use super::{
    burner::Burner,
    factory::Factory,
    items::item_to_str,
    serialize_impl,
    structure::{Energy, Structure, StructureBundle},
    DropItem, FactorishState, Inventory, InventoryTrait, ItemType, Position, Recipe,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

const FUEL_CAPACITY: usize = 10;

#[derive(Serialize, Deserialize)]
pub(crate) struct Furnace {
    position: Position,
}

impl Furnace {
    pub(crate) fn new(position: &Position) -> StructureBundle {
        StructureBundle::new(
            Box::new(Furnace {
                position: *position,
            }),
            Some(Burner {
                inventory: Inventory::new(),
                capacity: FUEL_CAPACITY,
            }),
            Some(Energy {
                value: 0.,
                max: 100.,
            }),
            Some(Factory::new()),
        )
    }
}

impl Structure for Furnace {
    fn name(&self) -> &str {
        "Furnace"
    }

    fn position(&self) -> &Position {
        &self.position
    }

    fn draw(
        &self,
        _burner: Option<&Burner>,
        energy: Option<&Energy>,
        factory: Option<&Factory>,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        is_toolbar: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };

        let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
        match state.image_furnace.as_ref() {
            Some(img) => {
                let sx = if let Some((energy, factory)) = energy.zip(factory) {
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
        if !is_toolbar {
            // crate::draw_fuel_alarm!(self, state, context);
        }

        Ok(())
    }

    fn desc(
        &self,
        _burner: Option<&Burner>,
        energy: Option<&Energy>,
        factory: Option<&Factory>,
        _state: &FactorishState,
    ) -> String {
        let (energy, factory) = if let Some(bundle) = energy.zip(factory) {
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

    fn input(&mut self, factory: Option<&mut Factory>, o: &DropItem) -> Result<(), JsValue> {
        let factory = factory.ok_or_else(|| js_str!("Furnace without Factory component"))?;
        console_log!(
            "Inputting to Furnace: {:?} recipe: {:?}",
            o.type_,
            factory.recipe.is_none()
        );
        if factory.recipe.is_none() {
            match o.type_ {
                ItemType::IronOre => {
                    factory.recipe = Some(Recipe::new(
                        hash_map!(ItemType::IronOre => 1usize),
                        hash_map!(ItemType::IronPlate => 1usize),
                        20.,
                        50.,
                    ));
                }
                ItemType::CopperOre => {
                    factory.recipe = Some(Recipe::new(
                        hash_map!(ItemType::CopperOre => 1usize),
                        hash_map!(ItemType::CopperPlate => 1usize),
                        20.,
                        50.,
                    ));
                }
                _ => {
                    return Err(JsValue::from_str(&format!(
                        "Cannot smelt {}",
                        item_to_str(&o.type_)
                    )))
                }
            }
        } else if let Some(recipe) = factory.recipe.as_ref() {
            console_log!("recipe: {:?}", recipe.input);
        }

        Err(JsValue::from_str("Recipe is not initialized"))
    }

    fn can_input(&self, item_type: &ItemType) -> bool {
        match *item_type {
            ItemType::IronOre | ItemType::CopperOre => true,
            _ => false,
        }
    }

    fn get_recipes(&self) -> Vec<Recipe> {
        vec![
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
        ]
    }

    serialize_impl!();
}
