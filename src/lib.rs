#![allow(non_upper_case_globals)]
mod perlin_noise;
mod utils;

use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlDivElement, ImageBitmap};

use perlin_noise::perlin_noise_pixel;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

macro_rules! console_log {
    ($fmt:expr, $($arg1:expr),*) => {
        log(&format!($fmt, $($arg1),+))
    };
    ($fmt:expr) => {
        log($fmt)
    }
}

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, factorish-js!");
}

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

#[allow(dead_code)]
fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

#[allow(dead_code)]
fn document() -> web_sys::Document {
    window()
        .document()
        .expect("should have a document on window")
}

#[allow(dead_code)]
fn body() -> web_sys::HtmlElement {
    document().body().expect("document should have a body")
}

const COAL_POWER: f64 = 100.; // kilojoules

#[derive(Copy, Clone)]
struct Cell {
    iron_ore: u32,
    coal_ore: u32,
    copper_ore: u32,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
struct Position {
    x: i32,
    y: i32,
}

impl Position {
    fn add(&self, o: (i32, i32)) -> Position {
        Self {
            x: self.x + o.0,
            y: self.y + o.1,
        }
    }
}

impl From<&[i32; 2]> for Position {
    fn from(xy: &[i32; 2]) -> Self {
        Self { x: xy[0], y: xy[1] }
    }
}

#[derive(Copy, Clone)]
enum Rotation {
    Left,
    Top,
    Right,
    Bottom,
}

impl Rotation {
    fn delta(&self) -> (i32, i32) {
        match self {
            Rotation::Left => (-1, 0),
            Rotation::Top => (0, -1),
            Rotation::Right => (1, 0),
            Rotation::Bottom => (0, 1),
        }
    }

    fn delta_inv(&self) -> (i32, i32) {
        let delta = self.delta();
        (-delta.0, -delta.1)
    }

    fn next(&mut self) {
        *self = match self {
            Rotation::Left => Rotation::Top,
            Rotation::Top => Rotation::Right,
            Rotation::Right => Rotation::Bottom,
            Rotation::Bottom => Rotation::Left,
        }
    }

    fn angle_deg(&self) -> i32 {
        self.angle_4() * 90
    }

    fn angle_4(&self) -> i32 {
        match self {
            Rotation::Left => 2,
            Rotation::Top => 3,
            Rotation::Right => 0,
            Rotation::Bottom => 1,
        }
    }

    fn angle_rad(&self) -> f64 {
        self.angle_deg() as f64 * std::f64::consts::PI / 180.
    }
}

enum FrameProcResult {
    None,
    InventoryChanged(Position),
}

enum ItemResponse {
    Move(i32, i32),
    Consume,
}

type ItemResponseResult = (ItemResponse, Option<FrameProcResult>);

type Inventory = HashMap<ItemType, usize>;

trait InventoryTrait {
    fn remove_item(&mut self, item: &ItemType) -> bool {
        self.remove_items(item, 1)
    }
    fn remove_items(&mut self, item: &ItemType, count: usize) -> bool;
    fn add_item(&mut self, item: &ItemType) {
        self.add_items(item, 1);
    }
    fn add_items(&mut self, item: &ItemType, count: usize);
}

impl InventoryTrait for Inventory {
    fn remove_items(&mut self, item: &ItemType, count: usize) -> bool {
        if let Some(entry) = self.get_mut(item) {
            if *entry <= count {
                self.remove(item);
            } else {
                *entry -= count;
            }
            true
        } else {
            false
        }
    }

    fn add_items(&mut self, item: &ItemType, count: usize) {
        if let Some(entry) = self.get_mut(item) {
            *entry += count;
        } else {
            self.insert(*item, count);
        }
    }
}

trait Structure {
    fn name(&self) -> &str;
    fn position(&self) -> &Position;
    fn draw(
        &self,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
    ) -> Result<(), JsValue>;
    fn desc(&self, _state: &FactorishState) -> String {
        String::from("")
    }
    fn frame_proc(
        &mut self,
        _state: &mut FactorishState,
        _structures: &mut dyn Iterator<Item = &mut Box<dyn Structure>>,
    ) -> Result<FrameProcResult, ()> {
        Ok(FrameProcResult::None)
    }
    fn movable(&self) -> bool {
        false
    }
    fn rotate(&mut self) -> Result<(), ()> {
        Err(())
    }
    fn set_rotation(&mut self, _rotation: &Rotation) -> Result<(), ()> {
        Err(())
    }
    /// Called every frame for each item that is on this structure.
    fn item_response(&mut self, _item: &DropItem) -> Result<ItemResponseResult, ()> {
        Err(())
    }
    fn input(&mut self, _o: &DropItem) -> Result<(), JsValue> {
        Err(JsValue::from_str("Not supported"))
    }
    fn output<'a>(
        &'a mut self,
        _state: &mut FactorishState,
        _position: &Position,
    ) -> Result<(DropItem, Box<dyn FnOnce(&DropItem) + 'a>), ()> {
        Err(())
    }
    fn inventory(&self) -> Option<&Inventory> {
        None
    }
    fn inventory_mut(&mut self) -> Option<&mut Inventory> {
        None
    }
}

const tilesize: i32 = 32;
struct ToolDef {
    item_type: ItemType,
    image: &'static str,
}
const tool_defs: [ToolDef; 5] = [
    ToolDef {
        item_type: ItemType::TransportBelt,
        image: "img/transport.png",
    },
    ToolDef {
        item_type: ItemType::Inserter,
        image: "img/inserter-base.png",
    },
    ToolDef {
        item_type: ItemType::OreMine,
        image: "img/mine.png",
    },
    ToolDef {
        item_type: ItemType::Chest,
        image: "img/chest.png",
    },
    ToolDef {
        item_type: ItemType::Furnace,
        image: "img/furnace.png",
    },
];

struct TransportBelt {
    position: Position,
    rotation: Rotation,
}

impl TransportBelt {
    fn new(x: i32, y: i32, rotation: Rotation) -> Self {
        TransportBelt {
            position: Position { x, y },
            rotation,
        }
    }
}

impl Structure for TransportBelt {
    fn name(&self) -> &str {
        "Transport Belt"
    }

    fn position(&self) -> &Position {
        &self.position
    }

    fn draw(
        &self,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
    ) -> Result<(), JsValue> {
        match state.image_belt.as_ref() {
            Some(img) => {
                let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
                context.save();
                context.translate(x + 16., y + 16.)?;
                context.rotate(self.rotation.angle_rad())?;
                context.translate(-(x + 16.), -(y + 16.))?;
                for i in 0..2 {
                    context
                        .draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                            img,
                            i as f64 * 32. - (state.sim_time * 16.) % 32.,
                            0.,
                            32.,
                            32.,
                            self.position.x as f64 * 32.,
                            self.position.y as f64 * 32.,
                            32.,
                            32.,
                        )?;
                }
                context.restore();
            }
            None => return Err(JsValue::from_str("belt image not available")),
        }

        Ok(())
    }

    fn movable(&self) -> bool {
        true
    }

    fn rotate(&mut self) -> Result<(), ()> {
        self.rotation.next();
        Ok(())
    }

    fn set_rotation(&mut self, rotation: &Rotation) -> Result<(), ()> {
        self.rotation = *rotation;
        Ok(())
    }

    fn item_response(&mut self, item: &DropItem) -> Result<ItemResponseResult, ()> {
        let moved_x = item.x + self.rotation.delta().0;
        let moved_y = item.y + self.rotation.delta().1;
        Ok((ItemResponse::Move(moved_x, moved_y), None))
    }
}

fn draw_direction_arrow(
    (x, y): (f64, f64),
    rotation: &Rotation,
    state: &FactorishState,
    context: &CanvasRenderingContext2d,
) -> Result<(), JsValue> {
    match state.image_direction.as_ref() {
        Some(img) => {
            context.save();
            context.translate(x + 16., y + 16.)?;
            context.rotate(rotation.angle_rad() + std::f64::consts::PI)?;
            context.translate(-(x + 16. + 4.) + 16., -(y + 16. + 8.) + 16.)?;
            context.draw_image_with_image_bitmap(img, x, y)?;
            context.restore();
        }
        None => return Err(JsValue::from_str("direction image not available")),
    };
    Ok(())
}

struct Inserter {
    position: Position,
    rotation: Rotation,
    cooldown: f64,
}

impl Inserter {
    fn new(x: i32, y: i32, rotation: Rotation) -> Self {
        Inserter {
            position: Position { x, y },
            rotation,
            cooldown: 0.,
        }
    }
}

impl Structure for Inserter {
    fn name(&self) -> &str {
        "Inserter"
    }

    fn position(&self) -> &Position {
        &self.position
    }

    fn draw(
        &self,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
    ) -> Result<(), JsValue> {
        let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
        match state.image_inserter.as_ref() {
            Some(img) => {
                context.draw_image_with_image_bitmap(img, x, y)?;
            }
            None => return Err(JsValue::from_str("inserter image not available")),
        }

        draw_direction_arrow((x, y), &self.rotation, state, context)?;

        Ok(())
    }

    fn frame_proc(
        &mut self,
        state: &mut FactorishState,
        structures: &mut dyn Iterator<Item = &mut Box<dyn Structure>>,
    ) -> Result<FrameProcResult, ()> {
        if self.cooldown <= 1. {
            self.cooldown = 0.;
            let input_position = self.position.add(self.rotation.delta_inv());
            let output_position = self.position.add(self.rotation.delta());
            let mut ret = FrameProcResult::None;

            let mut try_output = |state: &mut FactorishState,
                                  structures: &mut dyn Iterator<Item = &mut Box<dyn Structure>>,
                                  type_|
             -> bool {
                if let Some((_structure_idx, structure)) = structures
                    .enumerate()
                    .find(|(_idx, structure)| *structure.position() == output_position)
                {
                    // console_log!(
                    //     "found structure to output[{}]: {}, {}, {}",
                    //     structure_idx,
                    //     structure.name(),
                    //     output_position.x,
                    //     output_position.y
                    // );
                    if structure
                        .input(&DropItem::new(
                            &mut state.serial_no,
                            type_,
                            output_position.x,
                            output_position.y,
                        ))
                        .is_ok()
                    {
                        ret = FrameProcResult::InventoryChanged(output_position);
                        true
                    } else {
                        false
                    }
                } else if let Ok(()) = state.new_object(output_position.x, output_position.y, type_)
                {
                    self.cooldown += 20.;
                    true
                } else {
                    false
                }
            };

            if let Some(&DropItem { type_, id, .. }) = state.find_item(&input_position) {
                if try_output(state, structures, type_) {
                    state.remove_item(id);
                } else {
                    // console_log!("fail output_object: {:?}", type_);
                }
            } else if let Some((_, structure)) = structures.enumerate().find(|(_, s)| {
                s.position().x == input_position.x && s.position().y == input_position.y
            }) {
                // console_log!("outputting from a structure at {:?}", structure.position());
                if let Ok((item, callback)) = structure.output(state, &output_position) {
                    if try_output(state, structures, item.type_) {
                        callback(&item);
                        if let Some(pos) = state.selected_structure_inventory {
                            if pos == input_position {
                                return Ok(FrameProcResult::InventoryChanged(input_position));
                                // if let Err(e) = state.on_show_inventory.call2(&window(), &JsValue::from(output_position.x), &JsValue::from(output_position.y)) {
                                //     console_log!("on_show_inventory fail: {:?}", e);
                                // }
                            }
                        }
                        // console_log!("output succeeded: {:?}", item.type_);
                    }
                } else {
                    // console_log!("output failed");
                }
            }
            return Ok(ret);
        } else {
            self.cooldown -= 1.;
        }
        Ok(FrameProcResult::None)
    }

    fn rotate(&mut self) -> Result<(), ()> {
        self.rotation.next();
        Ok(())
    }

    fn set_rotation(&mut self, rotation: &Rotation) -> Result<(), ()> {
        self.rotation = *rotation;
        Ok(())
    }
}

const CHEST_CAPACITY: usize = 100;

struct Chest {
    position: Position,
    inventory: Inventory,
}

impl Chest {
    fn new(position: &Position) -> Self {
        Chest {
            position: *position,
            inventory: Inventory::new(),
        }
    }
}

fn item_to_str(type_: &ItemType) -> String {
    match type_ {
        ItemType::IronOre => "Iron Ore".to_string(),
        ItemType::CoalOre => "Coal Ore".to_string(),
        ItemType::CopperOre => "Copper Ore".to_string(),
        ItemType::IronPlate => "Iron Plate".to_string(),
        ItemType::CopperPlate => "Copper Plate".to_string(),

        ItemType::TransportBelt => "Transport Belt".to_string(),
        ItemType::Chest => "Chest".to_string(),
        ItemType::Inserter => "Inserter".to_string(),
        ItemType::OreMine => "Ore Mine".to_string(),
        ItemType::Furnace => "Furnace".to_string(),
    }
}

fn str_to_item(name: &str) -> Option<ItemType> {
    match name {
        "Iron Ore" => Some(ItemType::IronOre),
        "Coal Ore" => Some(ItemType::CoalOre),
        "Copper Ore" => Some(ItemType::CopperOre),
        "Iron Plate" => Some(ItemType::IronPlate),
        "Copper Plate" => Some(ItemType::CopperPlate),

        "Transport Belt" => Some(ItemType::TransportBelt),
        "Chest" => Some(ItemType::Chest),
        "Inserter" => Some(ItemType::Inserter),
        "Ore Mine" => Some(ItemType::OreMine),
        "Furnace" => Some(ItemType::Furnace),

        _ => None,
    }
}

impl Structure for Chest {
    fn name(&self) -> &'static str {
        "Chest"
    }

    fn position(&self) -> &Position {
        &self.position
    }

    fn draw(
        &self,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
    ) -> Result<(), JsValue> {
        let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
        match state.image_chest.as_ref() {
            Some(img) => {
                context.draw_image_with_image_bitmap(img, x, y)?;
                Ok(())
            }
            None => Err(JsValue::from_str("chest image not available")),
        }
    }

    fn desc(&self, _state: &FactorishState) -> String {
        format!(
            "Items: \n{}",
            self.inventory
                .iter()
                .map(|item| format!("{:?}: {}<br>", item.0, item.1))
                .fold(String::from(""), |accum, item| accum + &item)
        )
    }

    fn item_response(&mut self, _item: &DropItem) -> Result<ItemResponseResult, ()> {
        if self.inventory.len() < CHEST_CAPACITY {
            self.inventory.add_item(&_item.type_);
            Ok((
                ItemResponse::Consume,
                Some(FrameProcResult::InventoryChanged(self.position)),
            ))
        } else {
            Err(())
        }
    }

    fn input(&mut self, o: &DropItem) -> Result<(), JsValue> {
        self.item_response(o)
            .map(|_| ())
            .map_err(|_| JsValue::from_str("ItemResponse failed"))
    }

    fn output<'a>(
        &'a mut self,
        state: &mut FactorishState,
        position: &Position,
    ) -> Result<(DropItem, Box<dyn FnOnce(&DropItem) + 'a>), ()> {
        if let Some(ref mut item) = self.inventory.iter_mut().next() {
            if 0 < *item.1 {
                let item_type = item.0.clone();
                Ok((
                    DropItem {
                        id: state.serial_no,
                        type_: *item.0,
                        x: position.x * 32,
                        y: position.y * 32,
                    },
                    Box::new(move |_| {
                        self.inventory.remove_item(&item_type);
                    }),
                ))
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    fn inventory(&self) -> Option<&Inventory> {
        Some(&self.inventory)
    }

    fn inventory_mut(&mut self) -> Option<&mut Inventory> {
        Some(&mut self.inventory)
    }
}

type ItemSet = HashMap<ItemType, usize>;

struct Recipe {
    input: ItemSet,
    output: ItemSet,
    power_cost: f64,
    recipe_time: f64,
}

struct OreMine {
    position: Position,
    rotation: Rotation,
    cooldown: f64,
    power: f64,
    max_power: f64,
    recipe: Option<Recipe>,
}

impl OreMine {
    fn new(x: i32, y: i32, rotation: Rotation) -> Self {
        OreMine {
            position: Position { x, y },
            rotation,
            cooldown: 0.,
            power: 20.,
            max_power: 20.,
            recipe: None,
        }
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
    ) -> Result<(), JsValue> {
        let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
        match state.image_mine.as_ref() {
            Some(img) => {
                context.draw_image_with_image_bitmap(img, x, y)?;
            }
            None => return Err(JsValue::from_str("mine image not available")),
        }

        draw_direction_arrow((x, y), &self.rotation, state, context)?;

        Ok(())
    }

    fn desc(&self, state: &FactorishState) -> String {
        let tile = &state.board
            [self.position.x as usize + self.position.y as usize * state.width as usize];
        if let Some(_recipe) = &self.recipe {
            let recipe_time = 80.;
            // Progress bar
            format!("{}{}{}{}{}",
                format!("Progress: {:.0}%<br>", (recipe_time - self.cooldown) / recipe_time * 100.),
                "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
                format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                    (recipe_time - self.cooldown) / recipe_time * 100.),
                format!(r#"Power: <div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>
                 <div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>"#,
                  if 0. < self.max_power { (self.power) / self.max_power * 100. } else { 0. }),
                format!("Expected output: {}", if 0 < tile.iron_ore { tile.iron_ore } else { tile.coal_ore }))
        // getHTML(generateItemImage("time", true, this.recipe.time), true) + "<br>" +
        // "Outputs: <br>" +
        // getHTML(generateItemImage(this.recipe.output, true, 1), true) + "<br>";
        } else {
            String::from("Empty")
        }
    }

    fn frame_proc(
        &mut self,
        state: &mut FactorishState,
        _structures: &mut dyn Iterator<Item = &mut Box<dyn Structure>>,
    ) -> Result<FrameProcResult, ()> {
        let otile = &state.tile_at(&[self.position.x, self.position.y]);
        if otile.is_none() {
            return Ok(FrameProcResult::None);
        }
        let tile = otile.unwrap();

        if self.recipe.is_none() {
            if 0 < tile.iron_ore {
                self.recipe = Some(Recipe {
                    input: HashMap::new(),
                    output: [(ItemType::IronOre, 1usize)]
                        .iter()
                        .map(|(k, v)| (*k, *v))
                        .collect(),
                    power_cost: 0.1,
                    recipe_time: 80.,
                });
            } else if 0 < tile.coal_ore {
                self.recipe = Some(Recipe {
                    input: HashMap::new(),
                    output: [(ItemType::CoalOre, 1usize)]
                        .iter()
                        .map(|(k, v)| (*k, *v))
                        .collect(),
                    power_cost: 0.1,
                    recipe_time: 80.,
                });
            } else if 0 < tile.copper_ore {
                self.recipe = Some(Recipe {
                    input: HashMap::new(),
                    output: [(ItemType::CopperOre, 1usize)]
                        .iter()
                        .map(|(k, v)| (*k, *v))
                        .collect(),
                    power_cost: 0.1,
                    recipe_time: 80.,
                });
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

            // Proceed only if we have sufficient energy in the buffer.
            let progress = (self.power / recipe.power_cost).min(1.);
            if self.cooldown < progress {
                self.cooldown = 0.;
                let output_position = self.position.add(self.rotation.delta());
                if !state.hit_check(output_position.x, output_position.y, None) {
                    // let dest_tile = state.board[dx as usize + dy as usize * state.width as usize];
                    let mut it = recipe.output.iter();
                    if let Some(item) = it.next() {
                        if let Err(_code) =
                            state.new_object(output_position.x, output_position.y, *item.0)
                        {
                            // console_log!("Failed to create object: {:?}", code);
                        } else {
                            if let Some(tile) =
                                state.tile_at_mut(&[self.position.x, self.position.y])
                            {
                                self.cooldown = recipe.recipe_time;
                                match *item.0 {
                                    ItemType::IronOre => tile.iron_ore -= 1,
                                    ItemType::CoalOre => tile.coal_ore -= 1,
                                    ItemType::CopperOre => tile.copper_ore -= 1,
                                    _ => (),
                                }
                            }
                        }
                        assert!(it.next().is_none());
                    } else {
                        return Err(());
                    }
                }
            } else {
                self.cooldown -= progress;
                self.power -= progress * recipe.power_cost;
            }
        }
        Ok(FrameProcResult::None)
    }

    fn rotate(&mut self) -> Result<(), ()> {
        self.rotation.next();
        Ok(())
    }

    fn set_rotation(&mut self, rotation: &Rotation) -> Result<(), ()> {
        self.rotation = *rotation;
        Ok(())
    }

    fn item_response(&mut self, item: &DropItem) -> Result<ItemResponseResult, ()> {
        if item.type_ == ItemType::CoalOre && self.power == 0. {
            self.max_power = 100.;
            self.power = 100.;
            Ok((ItemResponse::Consume, None))
        } else {
            Err(())
        }
    }
}

struct Furnace {
    position: Position,
    inventory: Inventory,
    progress: f64,
    power: f64,
    max_power: f64,
    recipe: Option<Recipe>,
}

impl Furnace {
    fn new(position: &Position) -> Self {
        Furnace {
            position: *position,
            inventory: Inventory::new(),
            progress: 0.,
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

    fn draw(
        &self,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
    ) -> Result<(), JsValue> {
        let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
        match state.image_furnace.as_ref() {
            Some(img) => {
                let sx = if 0. < self.progress && 0. < self.power {
                    ((((state.sim_time * 5.) as isize) % 2 + 1) * 32) as f64
                } else {
                    0.
                };
                context.draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    img, sx, 0., 32., 32., x, y, 32., 32.,
                )?;
            }
            None => return Err(JsValue::from_str("furnace image not available")),
        }

        Ok(())
    }

    fn desc(&self, _state: &FactorishState) -> String {
        format!(
            "{}<br>{}",
            if self.recipe.is_some() {
                // Progress bar
                format!("{}{}{}{}",
                    format!("Progress: {:.0}%<br>", self.progress * 100.),
                    "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
                    format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                        self.progress * 100.),
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
            format!(
                "Items: \n{}",
                self.inventory
                    .iter()
                    .map(|item| format!("{:?}: {}<br>", item.0, item.1))
                    .fold(String::from(""), |accum, item| accum + &item)
            )
        )
    }

    fn frame_proc(
        &mut self,
        _state: &mut FactorishState,
        _structures: &mut dyn Iterator<Item = &mut Box<dyn Structure>>,
    ) -> Result<FrameProcResult, ()> {
        if let Some(recipe) = &self.recipe {
            let mut ret = FrameProcResult::None;
            // First, check if we need to refill the energy buffer in order to continue the current work.
            if self.inventory.get(&ItemType::CoalOre).is_some() {
                // Refill the energy from the fuel
                if self.power < recipe.power_cost {
                    self.power += COAL_POWER;
                    self.max_power = self.power;
                    self.inventory.remove_item(&ItemType::CoalOre);
                    ret = FrameProcResult::InventoryChanged(self.position);
                }
            }

            // Proceed only if we have sufficient energy in the buffer.
            let progress = (self.power / recipe.power_cost)
                .min(1. / recipe.recipe_time)
                .min(1.);
            if 1. <= self.progress + progress {
                self.progress = 0.;
                let has_ingredients = recipe
                    .input
                    .iter()
                    .map(|consume_item| {
                        if let Some(entry) = self.inventory.get(&consume_item.0) {
                            *consume_item.1 <= *entry
                        } else {
                            false
                        }
                    })
                    .all(|v| v);

                // First, check if we have enough ingredients to finish this recipe.
                if !has_ingredients {
                    self.recipe = None;
                    return Ok(FrameProcResult::None);
                }

                // Consume inputs from inventory
                for consume_item in &recipe.input {
                    self.inventory.remove_item(&consume_item.0);
                }

                // Produce outputs into inventory
                for output_item in &recipe.output {
                    self.inventory.add_item(&output_item.0);
                }
                return Ok(FrameProcResult::InventoryChanged(self.position));
            } else {
                self.progress += progress;
                self.power -= progress * recipe.power_cost;
            }
            return Ok(ret);
        }
        Ok(FrameProcResult::None)
    }

    fn inventory(&self) -> Option<&Inventory> {
        Some(&self.inventory)
    }

    fn inventory_mut(&mut self) -> Option<&mut Inventory> {
        Some(&mut self.inventory)
    }

    fn input(&mut self, o: &DropItem) -> Result<(), JsValue> {
        if self.recipe.is_none() {
            match o.type_ {
                ItemType::IronOre => {
                    self.recipe = Some(Recipe {
                        input: [(ItemType::IronOre, 1usize)]
                            .iter()
                            .map(|(k, v)| (*k, *v))
                            .collect(),
                        output: [(ItemType::IronPlate, 1usize)]
                            .iter()
                            .map(|(k, v)| (*k, *v))
                            .collect(),
                        power_cost: 20.,
                        recipe_time: 50.,
                    });
                }
                ItemType::CopperOre => {
                    self.recipe = Some(Recipe {
                        input: [(ItemType::CopperOre, 1usize)]
                            .iter()
                            .map(|(k, v)| (*k, *v))
                            .collect(),
                        output: [(ItemType::CopperPlate, 1usize)]
                            .iter()
                            .map(|(k, v)| (*k, *v))
                            .collect(),
                        power_cost: 20.,
                        recipe_time: 50.,
                    });
                }
                _ => {
                    return Err(JsValue::from_str(&format!(
                        "Cannot smelt {}",
                        item_to_str(&o.type_)
                    )))
                }
            }
        }

        // Fuels are always welcome.
        if o.type_ == ItemType::CoalOre {
            self.inventory.add_item(&ItemType::CoalOre);
            return Ok(());
        }

        if let Some(recipe) = &self.recipe {
            if recipe
                .input
                .iter()
                .find(|item| *item.0 == o.type_)
                .is_some()
                || recipe
                    .output
                    .iter()
                    .find(|item| *item.0 == o.type_)
                    .is_some()
            {
                self.inventory.add_item(&o.type_);
                return Ok(());
            } else {
                return Err(JsValue::from_str("Item is not part of recipe"));
            }
        }
        Err(JsValue::from_str("Recipe is not initialized"))
    }

    fn output<'a>(
        &'a mut self,
        state: &mut FactorishState,
        position: &Position,
    ) -> Result<(DropItem, Box<dyn FnOnce(&DropItem) + 'a>), ()> {
        if let Some(ref mut item) = self.inventory.iter_mut().next() {
            if 0 < *item.1 {
                let item_type = *item.0;
                Ok((
                    DropItem {
                        id: state.serial_no,
                        type_: *item.0,
                        x: position.x * 32,
                        y: position.y * 32,
                    },
                    Box::new(move |_| {
                        self.inventory.remove_item(&item_type);
                    }),
                ))
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
enum ItemType {
    IronOre,
    CoalOre,
    CopperOre,
    IronPlate,
    CopperPlate,

    TransportBelt,
    Chest,
    Inserter,
    OreMine,
    Furnace,
}

const objsize: i32 = 8;

struct DropItem {
    id: u32,
    type_: ItemType,
    x: i32,
    y: i32,
}

impl DropItem {
    fn new(serial_no: &mut u32, type_: ItemType, c: i32, r: i32) -> Self {
        let itilesize = tilesize as i32;
        let ret = DropItem {
            id: *serial_no,
            type_,
            x: c * itilesize + itilesize / 2,
            y: r * itilesize + itilesize / 2,
        };
        *serial_no += 1;
        ret
    }
}

struct Player {
    inventory: Inventory,
    selected_item: Option<ItemType>,
}

impl Player {
    fn add_item(&mut self, name: &ItemType, count: usize) {
        self.inventory.add_items(name, count);
    }

    fn select_item(&mut self, name: &ItemType) -> Result<(), JsValue> {
        if self.inventory.get(name).is_some() {
            self.selected_item = Some(*name);
            Ok(())
        } else {
            self.selected_item = None;
            Err(JsValue::from_str("item not found"))
        }
    }
}

#[wasm_bindgen]
pub struct FactorishState {
    delta_time: f64,
    sim_time: f64,
    width: u32,
    height: u32,
    viewport_width: f64,
    viewport_height: f64,
    board: Vec<Cell>,
    structures: Vec<Box<dyn Structure>>,
    selected_structure_inventory: Option<Position>,
    selected_structure_item: Option<ItemType>,
    drop_items: Vec<DropItem>,
    serial_no: u32,
    selected_tool: Option<usize>,
    tool_rotation: Rotation,
    player: Player,

    // rendering states
    cursor: Option<[i32; 2]>,
    info_elem: Option<HtmlDivElement>,
    on_player_update: js_sys::Function,
    // on_show_inventory: js_sys::Function,
    image_dirt: Option<ImageBitmap>,
    image_ore: Option<ImageBitmap>,
    image_coal: Option<ImageBitmap>,
    image_copper: Option<ImageBitmap>,
    image_belt: Option<ImageBitmap>,
    image_chest: Option<ImageBitmap>,
    image_mine: Option<ImageBitmap>,
    image_furnace: Option<ImageBitmap>,
    image_inserter: Option<ImageBitmap>,
    image_direction: Option<ImageBitmap>,
    image_iron_ore: Option<ImageBitmap>,
    image_coal_ore: Option<ImageBitmap>,
    image_copper_ore: Option<ImageBitmap>,
    image_iron_plate: Option<ImageBitmap>,
    image_copper_plate: Option<ImageBitmap>,
}

#[derive(Debug)]
enum NewObjectErr {
    BlockedByStructure,
    BlockedByItem,
    OutOfMap,
}

#[derive(Debug)]
enum RotateErr {
    NotFound,
    NotSupported,
}

#[wasm_bindgen]
impl FactorishState {
    #[wasm_bindgen(constructor)]
    pub fn new(
        on_player_update: js_sys::Function,
        // on_show_inventory: js_sys::Function,
    ) -> Result<FactorishState, JsValue> {
        console_log!("FactorishState constructor");

        let width = 64;
        let height = 64;

        Ok(FactorishState {
            delta_time: 0.1,
            sim_time: 0.0,
            width,
            height,
            viewport_height: 0.,
            viewport_width: 0.,
            cursor: None,
            selected_tool: None,
            tool_rotation: Rotation::Left,
            player: Player {
                inventory: [
                    (ItemType::TransportBelt, 10usize),
                    (ItemType::Inserter, 5usize),
                    (ItemType::OreMine, 5usize),
                    (ItemType::Chest, 3usize),
                    (ItemType::Furnace, 3usize),
                ]
                .iter()
                .map(|v| *v)
                .collect(),
                selected_item: None,
            },
            info_elem: None,
            image_dirt: None,
            image_ore: None,
            image_coal: None,
            image_copper: None,
            image_belt: None,
            image_chest: None,
            image_mine: None,
            image_furnace: None,
            image_inserter: None,
            image_direction: None,
            image_iron_ore: None,
            image_coal_ore: None,
            image_copper_ore: None,
            image_iron_plate: None,
            image_copper_plate: None,
            board: {
                let mut ret = vec![
                    Cell {
                        iron_ore: 0,
                        coal_ore: 0,
                        copper_ore: 0,
                    };
                    (width * height) as usize
                ];
                for y in 0..height {
                    for x in 0..width {
                        let [fx, fy] = [x as f64, y as f64];
                        let iron = (perlin_noise_pixel(fx, fy, 8) * 4000. - 3000.).max(0.) as u32;
                        let copper = (perlin_noise_pixel(fx, fy, 9) * 4000. - 3000.).max(0.) as u32;
                        let coal = (perlin_noise_pixel(fx, fy, 10) * 2000. - 1500.).max(0.) as u32;

                        match [iron, copper, coal].iter().enumerate().max_by_key(|v| v.1) {
                            Some((0, _)) => ret[(x + y * width) as usize].coal_ore = coal,
                            Some((1, _)) => ret[(x + y * width) as usize].copper_ore = copper,
                            Some((2, _)) => ret[(x + y * width) as usize].iron_ore = iron,
                            _ => (),
                        }
                    }
                }
                ret
            },
            structures: vec![
                Box::new(TransportBelt::new(10, 3, Rotation::Left)),
                Box::new(TransportBelt::new(11, 3, Rotation::Left)),
                Box::new(TransportBelt::new(12, 3, Rotation::Left)),
                Box::new(OreMine::new(12, 2, Rotation::Bottom)),
            ],
            selected_structure_inventory: None,
            selected_structure_item: None,
            drop_items: vec![],
            serial_no: 0,
            on_player_update,
            // on_show_inventory,
        })
    }

    #[wasm_bindgen]
    pub fn simulate(&mut self, delta_time: f64) -> Result<js_sys::Array, JsValue> {
        // console_log!("simulating delta_time {}, {}", delta_time, self.sim_time);
        self.delta_time = delta_time;
        self.sim_time += delta_time;

        // Since we cannot use callbacks to report events to the JavaScript environment,
        // we need to accumulate events during simulation and return them as an array.
        let mut events = vec![];

        let mut frame_proc_result_to_event = |result: Result<FrameProcResult, ()>| {
            if let Ok(FrameProcResult::InventoryChanged(pos)) = result {
                events.push(js_sys::Array::of3(
                    &JsValue::from_str("updateStructureInventory"),
                    &JsValue::from(pos.x),
                    &JsValue::from(pos.y),
                ))
            }
        };

        // This is silly way to avoid borrow checker that temporarily move the structures
        // away from self so that they do not claim mutable borrow twice, but it works.
        let mut structures = std::mem::take(&mut self.structures);
        for i in 0..structures.len() {
            if 0 < i {
                let (front, mid) = structures.split_at_mut(i);
                let (center, last) = mid
                    .split_first_mut()
                    .ok_or(JsValue::from_str("Structures split fail"))?;
                frame_proc_result_to_event(
                    center.frame_proc(self, &mut front.into_iter().chain(last.into_iter())),
                );
            } else {
                let (center, last) = structures
                    .split_first_mut()
                    .ok_or(JsValue::from_str("Structures split fail"))?;
                frame_proc_result_to_event(center.frame_proc(self, &mut last.iter_mut()));
            }
        }
        // for structure in &mut structures {
        //     structure.frame_proc(self, &mut structures);
        // }
        // let mut drop_items = std::mem::take(&mut self.drop_items);
        let mut to_remove = vec![];
        for i in 0..self.drop_items.len() {
            let item = &self.drop_items[i];
            if 0 < item.x
                && item.x < self.width as i32 * tilesize
                && 0 < item.y
                && item.y < self.height as i32 * tilesize
            {
                if let Some(item_response_result) = structures
                    .iter_mut()
                    .find(|s| s.position().x == item.x / 32 && s.position().y == item.y / 32)
                    .and_then(|structure| structure.item_response(item).ok())
                {
                    match item_response_result.0 {
                        ItemResponse::Move(moved_x, moved_y) => {
                            if self.hit_check(moved_x, moved_y, Some(item.id)) {
                                continue;
                            }
                            if let Some(s) = structures.iter().find(|s| {
                                s.position()
                                    == &Position {
                                        x: moved_x / 32,
                                        y: moved_y / 32,
                                    }
                            }) {
                                if !s.movable() {
                                    continue;
                                }
                            } else {
                                continue;
                            }
                            let item = &mut self.drop_items[i];
                            item.x = moved_x;
                            item.y = moved_y;
                        }
                        ItemResponse::Consume => {
                            to_remove.push(item.id);
                        }
                    }
                    if let Some(result) = item_response_result.1 {
                        frame_proc_result_to_event(Ok(result));
                    }
                }
            }
        }

        for id in to_remove {
            self.remove_item(id);
        }

        self.structures = structures;
        // self.drop_items = drop_items;
        self.update_info();
        Ok(events.iter().collect())
    }

    fn tile_at(&self, tile: &[i32]) -> Option<Cell> {
        if 0 <= tile[0]
            && tile[0] < self.width as i32
            && 0 <= tile[1]
            && tile[1] < self.height as i32
        {
            Some(self.board[tile[0] as usize + tile[1] as usize * self.width as usize])
        } else {
            None
        }
    }

    fn tile_at_mut(&mut self, tile: &[i32]) -> Option<&mut Cell> {
        if 0 <= tile[0]
            && tile[0] < self.width as i32
            && 0 <= tile[1]
            && tile[1] < self.height as i32
        {
            Some(&mut self.board[tile[0] as usize + tile[1] as usize * self.width as usize])
        } else {
            None
        }
    }

    /// Look up a structure at a given tile coordinates
    fn find_structure_tile(&self, tile: &[i32]) -> Option<&dyn Structure> {
        self.structures
            .iter()
            .find(|s| s.position().x == tile[0] && s.position().y == tile[1])
            .map(|s| s.as_ref())
    }

    /// Dirty hack to enable modifying a structure in an array.
    /// Instead of returning mutable reference, return an index into the array, so the
    /// caller can directly reference the structure from array `self.structures[idx]`.
    ///
    /// Because mutable version of find_structure_tile doesn't work.
    fn find_structure_tile_idx(&self, tile: &[i32]) -> Option<usize> {
        self.structures
            .iter()
            .enumerate()
            .find(|(_, s)| s.position().x == tile[0] && s.position().y == tile[1])
            .map(|(idx, _)| idx)
    }

    // fn find_structure_tile_mut<'a>(&'a mut self, tile: &[i32]) -> Option<&'a mut dyn Structure> {
    //     self.structures
    //         .iter_mut()
    //         .find(|s| s.position().x == tile[0] && s.position().y == tile[1])
    //         .map(|s| s.as_mut())
    // }

    fn _find_structure(&self, pos: &[f64]) -> Option<&dyn Structure> {
        self.find_structure_tile(&[(pos[0] / 32.) as i32, (pos[1] / 32.) as i32])
    }

    fn find_item(&self, pos: &Position) -> Option<&DropItem> {
        self.drop_items
            .iter()
            .find(|item| item.x / 32 == pos.x && item.y / 32 == pos.y)
    }

    fn remove_item(&mut self, id: u32) -> Option<DropItem> {
        if let Some((i, _)) = self
            .drop_items
            .iter()
            .enumerate()
            .find(|item| item.1.id == id)
        {
            let ret = Some(self.drop_items.remove(i));
            ret
        } else {
            None
        }
    }

    fn _remove_item_pos(&mut self, pos: &Position) -> Option<DropItem> {
        if let Some((i, _)) = self
            .drop_items
            .iter()
            .enumerate()
            .find(|item| item.1.x / 32 == pos.x && item.1.y / 32 == pos.y)
        {
            Some(self.drop_items.remove(i))
        } else {
            None
        }
    }

    fn update_info(&self) {
        if let Some(cursor) = self.cursor {
            if let Some(ref elem) = self.info_elem {
                if cursor[0] < self.width as i32 && cursor[1] < self.height as i32 {
                    elem.set_inner_html(
                        &if let Some(structure) = self.find_structure_tile(&cursor) {
                            format!(r#"Type: {}<br>{}"#, structure.name(), structure.desc(&self))
                        } else {
                            let cell = self.board
                                [cursor[0] as usize + cursor[1] as usize * self.width as usize];
                            format!(
                                r#"Empty tile<br>
                                Iron Ore: {}<br>
                                Coal Ore: {}<br>
                                Copper Ore: {}"#,
                                cell.iron_ore, cell.coal_ore, cell.copper_ore
                            )
                        },
                    );
                } else {
                    elem.set_inner_html("");
                }
            }
        }
    }

    /// Check whether given coordinates hits some object
    fn hit_check(&self, x: i32, y: i32, ignore: Option<u32>) -> bool {
        for item in &self.drop_items {
            if let Some(ignore_id) = ignore {
                if ignore_id == item.id {
                    continue;
                }
            }
            if (x - item.x).abs() < objsize && (y - item.y).abs() < objsize {
                return true;
            }
        }
        false
    }

    fn rotate(&mut self) -> Result<bool, RotateErr> {
        if let Some(_selected_tool) = self.selected_tool {
            self.tool_rotation.next();
            Ok(true)
        } else {
            if let Some(ref cursor) = self.cursor {
                if let Some(idx) = self.find_structure_tile_idx(cursor) {
                    return Ok(self.structures[idx]
                        .rotate()
                        .map_err(|()| RotateErr::NotSupported)
                        .map(|_| false)?);
                }
            }
            Err(RotateErr::NotFound)
        }
    }

    /// Insert an object on the board.  It could fail if there's already some object at the position.
    fn new_object(&mut self, c: i32, r: i32, type_: ItemType) -> Result<(), NewObjectErr> {
        let obj = DropItem::new(&mut self.serial_no, type_, c, r);
        if 0 <= c && c < self.width as i32 && 0 <= r && r < self.height as i32 {
            if let Some(stru) = self.find_structure_tile(&[c, r]) {
                if !stru.movable() {
                    return Err(NewObjectErr::BlockedByStructure);
                }
            }
            // return board[c + r * ysize].structure.input(obj);
            if self.hit_check(obj.x, obj.y, Some(obj.id)) {
                return Err(NewObjectErr::BlockedByItem);
            }
            // obj.addElem();
            self.drop_items.push(obj);
            return Ok(());
        }
        Err(NewObjectErr::OutOfMap)
    }

    fn harvest(&mut self, position: &Position) -> Result<bool, JsValue> {
        if let Some((index, structure)) = self
            .structures
            .iter()
            .enumerate()
            .find(|(_, structure)| structure.position() == position)
        {
            self.player
                .inventory
                .add_item(&str_to_item(&structure.name()).ok_or_else(|| {
                    JsValue::from_str(&format!("wrong structure name: {:?}", structure.name()))
                })?);
            if let Some(inventory) = structure.inventory() {
                for (name, &count) in inventory {
                    self.player.add_item(name, count)
                }
            }
            self.structures.remove(index);
            self.on_player_update
                .call1(&window(), &JsValue::from(self.get_player_inventory()?))
                .unwrap_or(JsValue::from(true));
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn get_inventory(
        &self,
        inventory: &Inventory,
        selected_item: &Option<ItemType>,
    ) -> Result<js_sys::Array, JsValue> {
        Ok(js_sys::Array::of2(
            &JsValue::from(
                inventory
                    .iter()
                    .map(|pair| {
                        js_sys::Array::of2(
                            &JsValue::from_str(&item_to_str(&pair.0)),
                            &JsValue::from_f64(*pair.1 as f64),
                        )
                    })
                    .collect::<js_sys::Array>(),
            ),
            &JsValue::from_str(
                &selected_item
                    .as_ref()
                    .map(|s| item_to_str(s))
                    .unwrap_or("".to_string()),
            ),
        ))
    }

    /// Returns [[itemName, itemCount]*, selectedItemName]
    pub fn get_player_inventory(&self) -> Result<js_sys::Array, JsValue> {
        self.get_inventory(&self.player.inventory, &self.player.selected_item)
    }

    pub fn select_player_inventory(&mut self, name: &str) -> Result<(), JsValue> {
        self.player.select_item(
            &str_to_item(name).ok_or_else(|| JsValue::from_str("Item name not identified"))?,
        )
    }

    pub fn open_structure_inventory(&mut self, c: i32, r: i32) -> Result<(), JsValue> {
        let pos = Position { x: c, y: r };
        if self.find_structure_tile(&[pos.x, pos.y]).is_some() {
            self.selected_structure_inventory = Some(pos);
            Ok(())
        } else {
            Err(JsValue::from_str("structure not found"))
        }
    }

    pub fn get_selected_inventory(&self) -> Result<JsValue, JsValue> {
        if let Some(pos) = self.selected_structure_inventory {
            return Ok(JsValue::from(js_sys::Array::of2(
                &JsValue::from(pos.x),
                &JsValue::from(pos.y),
            )));
        }
        Ok(JsValue::null())
    }

    pub fn get_structure_inventory(&self, c: i32, r: i32) -> Result<js_sys::Array, JsValue> {
        if let Some(structure) = self.find_structure_tile(&[c, r]) {
            if let Some(inventory) = structure.inventory() {
                return self.get_inventory(inventory, &self.selected_structure_item);
            }
        }
        Err(JsValue::from_str(
            "structure is not found or doesn't have inventory",
        ))
    }

    pub fn select_structure_inventory(&mut self, name: &str) -> Result<(), JsValue> {
        self.selected_structure_item =
            Some(str_to_item(name).ok_or_else(|| JsValue::from("Item name not valid"))?);
        Ok(())
    }

    fn move_inventory_item(src: &mut Inventory, dst: &mut Inventory, item_type: &ItemType) -> bool {
        if let Some(src_item) = src.remove(item_type) {
            dst.add_items(item_type, src_item);
            true
        } else {
            false
        }
    }

    pub fn move_selected_inventory_item(&mut self, to_player: bool) -> Result<bool, JsValue> {
        if let Some(pos) = self.selected_structure_inventory {
            if let Some(idx) = self.find_structure_tile_idx(&[pos.x, pos.y]) {
                if let Some(inventory) = self.structures[idx].inventory_mut() {
                    let (src, dst, item_name) = if to_player {
                        (
                            inventory,
                            &mut self.player.inventory,
                            &self.selected_structure_item,
                        )
                    } else {
                        (
                            &mut self.player.inventory,
                            inventory,
                            &self.player.selected_item,
                        )
                    };
                    console_log!("moving {:?}", item_name);
                    if let Some(item_name) = item_name {
                        if FactorishState::move_inventory_item(src, dst, item_name) {
                            self.on_player_update
                                .call1(&window(), &JsValue::from(self.get_player_inventory()?))?;
                            return Ok(true);
                        }
                    }
                }
            }
        }
        Ok(false)
    }

    fn new_structure(
        &self,
        tool_index: usize,
        cursor: &Position,
    ) -> Result<Box<dyn Structure>, JsValue> {
        Ok(match tool_index {
            0 => Box::new(TransportBelt::new(cursor.x, cursor.y, self.tool_rotation)),
            1 => Box::new(Inserter::new(cursor.x, cursor.y, self.tool_rotation)),
            2 => Box::new(OreMine::new(cursor.x, cursor.y, self.tool_rotation)),
            3 => Box::new(Chest::new(cursor)),
            _ => Box::new(Furnace::new(cursor)),
        })
    }

    pub fn mouse_down(&mut self, pos: &[f64], button: i32) -> Result<JsValue, JsValue> {
        if pos.len() < 2 {
            return Err(JsValue::from_str("position must have 2 elements"));
        }
        let cursor = Position {
            x: (pos[0] / 32.) as i32,
            y: (pos[1] / 32.) as i32,
        };

        let mut events = vec![];

        if button == 0 {
            if let Some(selected_tool) = self.selected_tool {
                if let Some(count) = self
                    .player
                    .inventory
                    .get(&tool_defs[selected_tool].item_type)
                {
                    if 1 <= *count {
                        self.harvest(&cursor)?;
                        self.structures
                            .push(self.new_structure(selected_tool, &cursor)?);
                        if let Some(count) = self
                            .player
                            .inventory
                            .get_mut(&tool_defs[selected_tool].item_type)
                        {
                            *count -= 1;
                        }
                        self.on_player_update
                            .call1(&window(), &JsValue::from(self.get_player_inventory()?))
                            .unwrap_or(JsValue::from(true));
                        events.push(js_sys::Array::of1(&JsValue::from_str(
                            "updatePlayerInventory",
                        )));
                    }
                }
            } else {
                // Select clicked structure
                console_log!("opening inventory at {:?}", cursor);
                if self.open_structure_inventory(cursor.x, cursor.y).is_ok() {
                    // self.on_show_inventory.call0(&window()).unwrap();
                    events.push(js_sys::Array::of3(
                        &JsValue::from_str("showInventory"),
                        &JsValue::from(cursor.x),
                        &JsValue::from(cursor.y),
                    ));
                    // let inventory_elem: web_sys::HtmlElement = document().get_element_by_id("inventory2").unwrap().dyn_into().unwrap();
                    // inventory_elem.style().set_property("display", "block").unwrap();
                }
            }
        } else {
            self.harvest(&cursor)?;
            events.push(js_sys::Array::of1(&JsValue::from_str(
                "updatePlayerInventory",
            )));
        }
        console_log!("clicked: {}, {}", cursor.x, cursor.y);
        self.update_info();
        Ok(JsValue::from(events.iter().collect::<js_sys::Array>()))
    }

    pub fn mouse_move(&mut self, pos: &[f64]) -> Result<(), JsValue> {
        if pos.len() < 2 {
            return Err(JsValue::from_str("position must have 2 elements"));
        }
        let cursor = [(pos[0] / 32.) as i32, (pos[1] / 32.) as i32];
        self.cursor = Some(cursor);
        // console_log!("cursor: {}, {}", cursor[0], cursor[1]);
        self.update_info();
        Ok(())
    }

    pub fn mouse_leave(&mut self) -> Result<(), JsValue> {
        self.cursor = None;
        if let Some(ref elem) = self.info_elem {
            elem.set_inner_html("");
        }
        console_log!("mouse_leave");
        Ok(())
    }

    pub fn on_key_down(&mut self, key_code: i32) -> Result<bool, JsValue> {
        if key_code == 82 {
            self.rotate()
                .map_err(|err| JsValue::from(format!("Rotate failed: {:?}", err)))
        } else {
            Ok(false)
        }
    }

    pub fn render_init(
        &mut self,
        canvas: HtmlCanvasElement,
        info_elem: HtmlDivElement,
        image_assets: js_sys::Array,
    ) -> Result<(), JsValue> {
        self.viewport_width = canvas.width() as f64;
        self.viewport_height = canvas.height() as f64;
        self.info_elem = Some(info_elem);
        let load_image = |path| -> Result<_, JsValue> {
            if let Some(value) = image_assets.iter().find(|value| {
                let array = js_sys::Array::from(value);
                array.iter().next() == Some(JsValue::from_str(path))
            }) {
                let array = js_sys::Array::from(&value);
                array
                    .to_vec()
                    .into_iter()
                    .nth(1)
                    .unwrap_or_else(|| {
                        JsValue::from_str(&format!(
                            "Couldn't convert value to ImageBitmap: {:?}",
                            path
                        ))
                    })
                    .dyn_into::<ImageBitmap>()
            } else {
                Err(JsValue::from_str(&format!("Image not found: {:?}", path)))
            }
        };
        self.image_dirt = Some(load_image("dirt")?);
        self.image_ore = Some(load_image("iron")?);
        self.image_coal = Some(load_image("coal")?);
        self.image_copper = Some(load_image("copper")?);
        self.image_belt = Some(load_image("transport")?);
        self.image_chest = Some(load_image("chest")?);
        self.image_mine = Some(load_image("mine")?);
        self.image_furnace = Some(load_image("furnace")?);
        self.image_inserter = Some(load_image("inserter")?);
        self.image_direction = Some(load_image("direction")?);
        self.image_iron_ore = Some(load_image("ore")?);
        self.image_coal_ore = Some(load_image("coalOre")?);
        self.image_copper_ore = Some(load_image("copperOre")?);
        self.image_iron_plate = Some(load_image("ironPlate")?);
        self.image_copper_plate = Some(load_image("copperPlate")?);
        Ok(())
    }

    pub fn tool_defs(&self) -> Result<js_sys::Array, JsValue> {
        Ok(tool_defs
            .iter()
            .map(|tool| JsValue::from_str(tool.image))
            .collect::<js_sys::Array>())
    }

    /// Returns 2-array with [selected_tool, inventory_count]
    pub fn selected_tool(&self) -> js_sys::Array {
        if let Some(selected_tool) = self.selected_tool {
            [
                JsValue::from(selected_tool as f64),
                JsValue::from(
                    *self
                        .player
                        .inventory
                        .get(&tool_defs[selected_tool].item_type)
                        .unwrap_or(&0) as f64,
                ),
            ]
            .iter()
            .collect()
        } else {
            js_sys::Array::new()
        }
    }

    pub fn render_tool(
        &self,
        tool_index: usize,
        context: &CanvasRenderingContext2d,
    ) -> Result<(), JsValue> {
        let mut tool = self.new_structure(tool_index, &Position { x: 0, y: 0 })?;
        tool.set_rotation(&self.tool_rotation).ok();
        tool.draw(self, context)?;
        Ok(())
    }

    pub fn select_tool(&mut self, tool: i32) -> bool {
        self.selected_tool = if 0 <= tool
            && !(self.selected_tool.is_some() && self.selected_tool.unwrap() as i32 == tool)
        {
            Some(tool as usize)
        } else {
            None
        };
        self.selected_tool.is_some()
    }

    pub fn rotate_tool(&mut self) -> i32 {
        self.tool_rotation.next();
        self.tool_rotation.angle_4()
    }

    pub fn tool_inventory(&self) -> js_sys::Array {
        tool_defs
            .iter()
            .map(|tool| {
                JsValue::from(*self.player.inventory.get(&tool.item_type).unwrap_or(&0) as f64)
            })
            .collect()
    }

    pub fn render(&self, context: CanvasRenderingContext2d) -> Result<(), JsValue> {
        use std::f64;

        context.clear_rect(0., 0., self.viewport_width, self.viewport_height);

        match self
            .image_dirt
            .as_ref()
            .zip(self.image_ore.as_ref())
            .zip(self.image_coal.as_ref())
            .zip(self.image_copper.as_ref())
        {
            Some((((img, img_ore), img_coal), img_copper)) => {
                for y in 0..self.viewport_height as u32 / 32 {
                    for x in 0..self.viewport_width as u32 / 32 {
                        context.draw_image_with_image_bitmap(
                            img,
                            x as f64 * 32.,
                            y as f64 * 32.,
                        )?;
                        let draw_ore = |ore: u32, img: &ImageBitmap| -> Result<(), JsValue> {
                            if 0 < ore {
                                let idx = (ore / 10).min(3);
                                // console_log!("x: {}, y: {}, idx: {}, ore: {}", x, y, idx, ore);
                                context.draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                                    img, (idx * 32) as f64, 0., 32., 32., x as f64 * 32., y as f64 * 32., 32., 32.)?;
                            }
                            Ok(())
                        };
                        draw_ore(self.board[(x + y * self.width) as usize].iron_ore, img_ore)?;
                        draw_ore(self.board[(x + y * self.width) as usize].coal_ore, img_coal)?;
                        draw_ore(
                            self.board[(x + y * self.width) as usize].copper_ore,
                            img_copper,
                        )?;
                    }
                }
                // console_log!(
                //     "iron ore: {}",
                //     self.board.iter().fold(0, |accum, val| accum + val.iron_ore)
                // );
            }
            _ => {
                return Err(JsValue::from_str("image not available"));
            }
        }

        for structure in &self.structures {
            structure.draw(&self, &context)?;
        }

        for item in &self.drop_items {
            let img = match item.type_ {
                ItemType::IronOre => &self.image_iron_ore,
                ItemType::CoalOre => &self.image_coal_ore,
                ItemType::CopperOre => &self.image_copper_ore,
                ItemType::IronPlate => &self.image_iron_plate,
                ItemType::CopperPlate => &self.image_copper_plate,

                ItemType::TransportBelt => &self.image_belt,
                ItemType::Chest => &self.image_chest,
                ItemType::Inserter => &self.image_inserter,
                ItemType::OreMine => &self.image_mine,
                ItemType::Furnace => &self.image_furnace,
            };
            if let Some(ref image) = img {
                context.draw_image_with_image_bitmap(
                    image,
                    item.x as f64 - 8.,
                    item.y as f64 - 8.,
                )?;
            }
        }

        if let Some(ref cursor) = self.cursor {
            let (x, y) = ((cursor[0] * 32) as f64, (cursor[1] * 32) as f64);
            if let Some(selected_tool) = self.selected_tool {
                context.save();
                context.set_global_alpha(0.5);
                let mut tool = self.new_structure(selected_tool, &Position::from(cursor))?;
                tool.set_rotation(&self.tool_rotation).ok();
                tool.draw(self, &context)?;
                context.restore();
            }
            context.set_stroke_style(&JsValue::from_str("blue"));
            context.set_line_width(2.);
            context.stroke_rect(x, y, 32., 32.);
        }

        Ok(())
    }
}
