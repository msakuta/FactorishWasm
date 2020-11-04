#![allow(non_upper_case_globals)]
mod perlin_noise;
mod utils;

use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlDivElement, HtmlImageElement};

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

fn document() -> web_sys::Document {
    window()
        .document()
        .expect("should have a document on window")
}

fn body() -> web_sys::HtmlElement {
    document().body().expect("document should have a body")
}

#[derive(Copy, Clone)]
struct Cell {
    iron_ore: u32,
    coal_ore: u32,
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

enum ItemResponse {
    Move(i32, i32),
    Consume,
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
    ) {
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
    fn item_response(&mut self, _item: &DropItem) -> Result<ItemResponse, ()> {
        Err(())
    }
    fn output<'a>(
        &'a mut self,
        _state: &mut FactorishState,
        _position: &Position,
    ) -> Result<(DropItem, Box<dyn FnOnce(&DropItem) + 'a>), ()> {
        Err(())
    }
    fn inventory(&self) -> Option<&HashMap<String, usize>> {
        None
    }
}

const tilesize: i32 = 32;
struct ToolDef {
    item_name: &'static str,
    image: &'static str,
}
const tool_defs: [ToolDef; 4] = [
    ToolDef {
        item_name: "Transport Belt",
        image: "img/transport.png",
    },
    ToolDef {
        item_name: "Inserter",
        image: "img/inserter-base.png",
    },
    ToolDef {
        item_name: "Ore Mine",
        image: "img/mine.png",
    },
    ToolDef {
        item_name: "Chest",
        image: "img/chest.png",
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
                    context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
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

    fn item_response(&mut self, item: &DropItem) -> Result<ItemResponse, ()> {
        let moved_x = item.x + self.rotation.delta().0;
        let moved_y = item.y + self.rotation.delta().1;
        Ok(ItemResponse::Move(moved_x, moved_y))
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
            context.draw_image_with_html_image_element(img, x, y)?;
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
                context.draw_image_with_html_image_element(img, x, y)?;
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
    ) {
        if self.cooldown <= 1. {
            self.cooldown = 0.;
            let input_position = self.position.add(self.rotation.delta_inv());
            let output_position = self.position.add(self.rotation.delta());

            let mut try_output = |state: &mut FactorishState, type_| -> bool {
                if let Some(structure_idx) =
                    state.find_structure_tile_idx(&[output_position.x, output_position.y])
                {
                    console_log!(
                        "found structure to output: {}, {}, {}",
                        output_position.x,
                        output_position.y,
                        structure_idx
                    );
                    state.structures[structure_idx]
                        .item_response(&DropItem::new(
                            &mut state.serial_no,
                            type_,
                            output_position.x,
                            output_position.y,
                        ))
                        .is_ok()
                } else if let Ok(()) = state.new_object(output_position.x, output_position.y, type_)
                {
                    self.cooldown += 20.;
                    true
                } else {
                    false
                }
            };

            if let Some(&DropItem { type_, id, .. }) = state.find_item(&input_position) {
                if try_output(state, type_) {
                    state.remove_item(id);
                } else {
                    console_log!("fail output_object: {:?}", type_);
                }
            } else if let Some((_, structure)) = structures.enumerate().find(|(_, s)| {
                s.position().x == input_position.x && s.position().y == input_position.y
            }) {
                console_log!("outputting: {:?}", structure.position());
                if let Ok((item, callback)) = structure.output(state, &output_position) {
                    if try_output(state, item.type_) {
                        callback(&item);
                        console_log!("output succeeded: {:?}", item.type_);
                    }
                } else {
                    console_log!("output failed");
                }
            }
        } else {
            self.cooldown -= 1.;
        }
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
    inventory: HashMap<String, usize>,
}

impl Chest {
    fn new(position: &Position) -> Self {
        Chest {
            position: *position,
            inventory: HashMap::new(),
        }
    }
}

fn item_to_str(type_: &ItemType) -> String {
    match type_ {
        ItemType::IronOre => "Iron Ore".to_string(),
        ItemType::CoalOre => "Coal Ore".to_string(),
    }
}

fn str_to_item(name: &str) -> Option<ItemType> {
    match name {
        "Iron Ore" => Some(ItemType::IronOre),
        "Coal Ore" => Some(ItemType::CoalOre),
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
                context.draw_image_with_html_image_element(img, x, y)?;
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
                .map(|item| format!("{}: {}<br>", item.0, item.1))
                .fold(String::from(""), |accum, item| accum + &item)
        )
    }

    fn item_response(&mut self, _item: &DropItem) -> Result<ItemResponse, ()> {
        if self.inventory.len() < CHEST_CAPACITY {
            if let Some(item_ref) = self.inventory.get_mut(&item_to_str(&_item.type_)) {
                *item_ref += 1;
            } else {
                self.inventory.insert(item_to_str(&_item.type_), 1);
            }
            Ok(ItemResponse::Consume)
        } else {
            Err(())
        }
    }

    fn output<'a>(
        &'a mut self,
        state: &mut FactorishState,
        position: &Position,
    ) -> Result<(DropItem, Box<dyn FnOnce(&DropItem) + 'a>), ()> {
        if let Some(ref mut item) = self.inventory.iter_mut().next() {
            if 0 < *item.1 {
                let item_name = item.0.clone();
                Ok((
                    DropItem {
                        id: state.serial_no,
                        type_: str_to_item(&item.0).ok_or(())?,
                        x: position.x * 32,
                        y: position.y * 32,
                    },
                    Box::new(move |_| {
                        if let Some(item) = self.inventory.iter_mut().find(|it| *it.0 == item_name)
                        {
                            *item.1 -= 1
                        }
                    }),
                ))
            } else {
                Err(())
            }
        } else {
            Err(())
        }
    }

    fn inventory(&self) -> Option<&HashMap<String, usize>> {
        Some(&self.inventory)
    }
}

struct Recipe {
    item_type: ItemType,
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
                context.draw_image_with_html_image_element(img, x, y)?;
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
    ) {
        let otile = &state.tile_at(&[self.position.x, self.position.y]);
        if otile.is_none() {
            return;
        }
        let tile = otile.unwrap();

        if self.recipe.is_none() {
            if 0 < tile.iron_ore {
                self.recipe = Some(Recipe {
                    item_type: ItemType::IronOre,
                    power_cost: 0.1,
                    recipe_time: 80.,
                });
            } else if 0 < tile.coal_ore {
                self.recipe = Some(Recipe {
                    item_type: ItemType::CoalOre,
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
                    if let Err(_code) =
                        state.new_object(output_position.x, output_position.y, recipe.item_type)
                    {
                        // console_log!("Failed to create object: {:?}", code);
                    } else {
                        if let Some(tile) = state.tile_at_mut(&[self.position.x, self.position.y]) {
                            self.cooldown = recipe.recipe_time;
                            match recipe.item_type {
                                ItemType::IronOre => tile.iron_ore -= 1,
                                ItemType::CoalOre => tile.coal_ore -= 1,
                            }
                        }
                    }
                }
            } else {
                self.cooldown -= progress;
                self.power -= progress * recipe.power_cost;
            }
        }
    }

    fn rotate(&mut self) -> Result<(), ()> {
        self.rotation.next();
        Ok(())
    }

    fn set_rotation(&mut self, rotation: &Rotation) -> Result<(), ()> {
        self.rotation = *rotation;
        Ok(())
    }

    fn item_response(&mut self, item: &DropItem) -> Result<ItemResponse, ()> {
        if item.type_ == ItemType::CoalOre && self.power == 0. {
            self.max_power = 100.;
            self.power = 100.;
            Ok(ItemResponse::Consume)
        } else {
            Err(())
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
enum ItemType {
    IronOre,
    CoalOre,
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
    inventory: HashMap<String, usize>,
}

impl Player {
    fn add_item(&mut self, name: &str, count: usize) {
        if let Some(entry) = self.inventory.get_mut(name) {
            *entry += count;
        } else {
            self.inventory.insert(name.to_string(), count);
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
    drop_items: Vec<DropItem>,
    serial_no: u32,
    selected_tool: Option<usize>,
    tool_rotation: Rotation,
    player: Player,

    // rendering states
    cursor: Option<[i32; 2]>,
    info_elem: Option<HtmlDivElement>,
    on_player_update: js_sys::Function,

    image_dirt: Option<HtmlImageElement>,
    image_ore: Option<HtmlImageElement>,
    image_coal: Option<HtmlImageElement>,
    image_belt: Option<HtmlImageElement>,
    image_chest: Option<HtmlImageElement>,
    image_mine: Option<HtmlImageElement>,
    image_inserter: Option<HtmlImageElement>,
    image_direction: Option<HtmlImageElement>,
    image_iron_ore: Option<HtmlImageElement>,
    image_coal_ore: Option<HtmlImageElement>,
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
    pub fn new(on_player_update: js_sys::Function) -> Result<FactorishState, JsValue> {
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
                    ("Transport Belt", 10usize),
                    ("Inserter", 5usize),
                    ("Ore Mine", 5usize),
                    ("Chest", 3usize),
                ]
                .iter()
                .map(|(s, num)| (String::from(*s), *num))
                .collect(),
            },
            info_elem: None,
            image_dirt: None,
            image_ore: None,
            image_coal: None,
            image_belt: None,
            image_chest: None,
            image_mine: None,
            image_inserter: None,
            image_direction: None,
            image_iron_ore: None,
            image_coal_ore: None,
            board: {
                let mut ret = vec![
                    Cell {
                        iron_ore: 0,
                        coal_ore: 0
                    };
                    (width * height) as usize
                ];
                for y in 0..height {
                    for x in 0..width {
                        let [fx, fy] = [x as f64, y as f64];
                        let iron = (perlin_noise_pixel(fx, fy, 8) * 4000. - 3000.).max(0.) as u32;
                        let coal = (perlin_noise_pixel(fx, fy, 10) * 2000. - 1500.).max(0.) as u32;
                        if iron < coal {
                            ret[(x + y * width) as usize].coal_ore = coal;
                        } else if 0 < iron {
                            ret[(x + y * width) as usize].iron_ore = iron;
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
            drop_items: vec![],
            serial_no: 0,
            on_player_update,
        })
    }

    #[wasm_bindgen]
    pub fn simulate(&mut self, delta_time: f64) -> Result<(), JsValue> {
        // console_log!("simulating delta_time {}, {}", delta_time, self.sim_time);
        self.delta_time = delta_time;
        self.sim_time += delta_time;

        // This is silly way to avoid borrow checker that temporarily move the structures
        // away from self so that they do not claim mutable borrow twice, but it works.
        let mut structures = std::mem::take(&mut self.structures);
        for i in 0..structures.len() {
            if 0 < i {
                let (front, mid) = structures.split_at_mut(i);
                let (center, last) = mid
                    .split_first_mut()
                    .ok_or(JsValue::from_str("Structures split fail"))?;
                center.frame_proc(self, &mut front.into_iter().chain(last.into_iter()));
            } else {
                let (center, last) = structures
                    .split_first_mut()
                    .ok_or(JsValue::from_str("Structures split fail"))?;
                center.frame_proc(self, &mut last.iter_mut());
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
                match structures
                    .iter_mut()
                    .find(|s| s.position().x == item.x / 32 && s.position().y == item.y / 32)
                    .and_then(|structure| structure.item_response(item).ok())
                {
                    Some(ItemResponse::Move(moved_x, moved_y)) => {
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
                    Some(ItemResponse::Consume) => {
                        to_remove.push(item.id);
                    }
                    _ => {}
                }
            }
        }

        for id in to_remove {
            self.remove_item(id);
        }

        self.structures = structures;
        // self.drop_items = drop_items;
        self.update_info();
        Ok(())
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
                                Coal Ore: {}"#,
                                cell.iron_ore, cell.coal_ore
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
            if let Some(count) = self.player.inventory.get_mut(structure.name()) {
                *count += 1;
            } else {
                self.player
                    .inventory
                    .insert(structure.name().to_string(), 1);
            }
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

    pub fn get_player_inventory(&self) -> Result<js_sys::Array, JsValue> {
        Ok(self
            .player
            .inventory
            .iter()
            .map(|pair| {
                js_sys::Array::of2(
                    &JsValue::from_str(pair.0),
                    &JsValue::from_f64(*pair.1 as f64),
                )
            })
            .collect::<js_sys::Array>())
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
            _ => Box::new(Chest::new(cursor)),
        })
    }

    pub fn mouse_down(&mut self, pos: &[f64], button: i32) -> Result<(), JsValue> {
        if pos.len() < 2 {
            return Err(JsValue::from_str("position must have 2 elements"));
        }
        let cursor = Position {
            x: (pos[0] / 32.) as i32,
            y: (pos[1] / 32.) as i32,
        };
        if button == 0 {
            if let Some(selected_tool) = self.selected_tool {
                if let Some(count) = self
                    .player
                    .inventory
                    .get(tool_defs[selected_tool].item_name)
                {
                    if 1 <= *count {
                        self.harvest(&cursor)?;
                        self.structures
                            .push(self.new_structure(selected_tool, &cursor)?);
                        if let Some(count) = self
                            .player
                            .inventory
                            .get_mut(tool_defs[selected_tool].item_name)
                        {
                            *count -= 1;
                        }
                        self.on_player_update
                            .call1(&window(), &JsValue::from(self.get_player_inventory()?))
                            .unwrap_or(JsValue::from(true));
                    }
                }
            }
        } else {
            self.harvest(&cursor)?;
        }
        console_log!("clicked: {}, {}", cursor.x, cursor.y);
        self.update_info();
        Ok(())
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
    ) -> Result<(), JsValue> {
        self.viewport_width = canvas.width() as f64;
        self.viewport_height = canvas.height() as f64;
        self.info_elem = Some(info_elem);
        let load_image = |path| -> Result<_, JsValue> {
            let img = HtmlImageElement::new()?;
            img.set_attribute("src", path)?;
            img.style().set_property("display", "none")?;
            body().append_child(&img)?;
            Ok(img)
        };
        self.image_dirt = Some(load_image("img/dirt.png")?);
        self.image_ore = Some(load_image("img/iron.png")?);
        self.image_coal = Some(load_image("img/coal.png")?);
        self.image_belt = Some(load_image("img/transport.png")?);
        self.image_chest = Some(load_image("img/chest.png")?);
        self.image_mine = Some(load_image("img/mine.png")?);
        self.image_inserter = Some(load_image("img/inserter-base.png")?);
        self.image_direction = Some(load_image("img/direction.png")?);
        self.image_iron_ore = Some(load_image("img/ore.png")?);
        self.image_coal_ore = Some(load_image("img/coal-ore.png")?);
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
                        .get(tool_defs[selected_tool].item_name)
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
                JsValue::from(*self.player.inventory.get(tool.item_name).unwrap_or(&0) as f64)
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
        {
            Some(((img, img_ore), img_coal)) => {
                for y in 0..self.viewport_height as u32 / 32 {
                    for x in 0..self.viewport_width as u32 / 32 {
                        context.draw_image_with_html_image_element(
                            img,
                            x as f64 * 32.,
                            y as f64 * 32.,
                        )?;
                        let draw_ore = |ore: u32, img: &HtmlImageElement| -> Result<(), JsValue> {
                            if 0 < ore {
                                let idx = (ore / 10).min(3);
                                // console_log!("x: {}, y: {}, idx: {}, ore: {}", x, y, idx, ore);
                                context.draw_image_with_html_image_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                                    img, (idx * 32) as f64, 0., 32., 32., x as f64 * 32., y as f64 * 32., 32., 32.)?;
                            }
                            Ok(())
                        };
                        draw_ore(self.board[(x + y * self.width) as usize].iron_ore, img_ore)?;
                        draw_ore(self.board[(x + y * self.width) as usize].coal_ore, img_coal)?;
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
            match item.type_ {
                ItemType::IronOre => {
                    if let Some(ref img_iron_ore) = self.image_iron_ore {
                        context.draw_image_with_html_image_element(
                            img_iron_ore,
                            item.x as f64 - 8.,
                            item.y as f64 - 8.,
                        )?;
                    }
                }
                ItemType::CoalOre => {
                    if let Some(ref img_coal_ore) = self.image_coal_ore {
                        context.draw_image_with_html_image_element(
                            img_coal_ore,
                            item.x as f64 - 8.,
                            item.y as f64 - 8.,
                        )?;
                    }
                }
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
