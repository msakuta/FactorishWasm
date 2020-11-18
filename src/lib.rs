#![allow(non_upper_case_globals)]

macro_rules! console_log {
    ($fmt:expr, $($arg1:expr),*) => {
        crate::log(&format!($fmt, $($arg1),+))
    };
    ($fmt:expr) => {
        crate::log($fmt)
    }
}
macro_rules! js_err {
    ($fmt:expr, $($arg1:expr),*) => {
        JsValue::from_str(&format!($fmt, $($arg1),+))
    };
    ($fmt:expr) => {
        JsValue::from_str($fmt)
    }
}

macro_rules! hash_map {
    { $($key:expr => $value:expr),+ } => {
        {
            let mut m = ::std::collections::HashMap::new();
            $(
                m.insert($key, $value);
            )+
            m
        }
    };
}

mod assembler;
mod boiler;
mod chest;
mod furnace;
mod inserter;
mod items;
mod ore_mine;
mod perlin_noise;
mod pipe;
mod steam_engine;
mod structure;
mod transport_belt;
mod utils;
mod water_well;

use assembler::Assembler;
use boiler::Boiler;
use chest::Chest;
use furnace::Furnace;
use inserter::Inserter;
use items::{item_to_str, render_drop_item, str_to_item, DropItem, ItemType};
use ore_mine::OreMine;
use perlin_noise::{perlin_noise_pixel, Xor128};
use pipe::Pipe;
use steam_engine::SteamEngine;
use structure::{FrameProcResult, ItemResponse, Position, Rotation, Structure};
use transport_belt::TransportBelt;
use water_well::{FluidType, WaterWell};

use serde::Serialize;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, HtmlDivElement, ImageBitmap};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub(crate) fn log(s: &str);
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
    fn count_item(&self, item: &ItemType) -> usize;
    fn merge(&mut self, other: Inventory);
    fn describe(&self) -> String;
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

    fn count_item(&self, item: &ItemType) -> usize {
        *self.get(item).unwrap_or(&0)
    }

    fn merge(&mut self, other: Inventory) {
        for (k, v) in other {
            if let Some(vv) = self.get_mut(&k) {
                *vv += v;
            } else {
                self.insert(k, v);
            }
        }
    }

    fn describe(&self) -> String {
        self.iter()
            .map(|item| format!("{:?}: {}<br>", item.0, item.1))
            .fold(String::from(""), |accum, item| accum + &item)
    }
}

use std::iter;
struct Ref<'r, T: ?Sized>(&'r T);
impl<'a, 'r, T: ?Sized> IntoIterator for &'a Ref<'r, T>
where
    &'a T: IntoIterator,
{
    type IntoIter = <&'a T as IntoIterator>::IntoIter;
    type Item = <&'a T as IntoIterator>::Item;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}
struct MutRef<'r, T: ?Sized>(&'r mut T);
impl<'a, 'r, T: ?Sized> IntoIterator for &'a mut MutRef<'r, T>
where
    &'a mut T: IntoIterator,
{
    type IntoIter = <&'a mut T as IntoIterator>::IntoIter;
    type Item = <&'a mut T as IntoIterator>::Item;
    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

const tilesize: i32 = 32;
struct ToolDef {
    item_type: ItemType,
}
const tool_defs: [ToolDef; 10] = [
    ToolDef {
        item_type: ItemType::TransportBelt,
    },
    ToolDef {
        item_type: ItemType::Inserter,
    },
    ToolDef {
        item_type: ItemType::OreMine,
    },
    ToolDef {
        item_type: ItemType::Chest,
    },
    ToolDef {
        item_type: ItemType::Furnace,
    },
    ToolDef {
        item_type: ItemType::Assembler,
    },
    ToolDef {
        item_type: ItemType::Boiler,
    },
    ToolDef {
        item_type: ItemType::WaterWell,
    },
    ToolDef {
        item_type: ItemType::Pipe,
    },
    ToolDef {
        item_type: ItemType::SteamEngine,
    },
];

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
            context.draw_image_with_image_bitmap(&img.bitmap, x, y)?;
            context.restore();
        }
        None => return Err(JsValue::from_str("direction image not available")),
    };
    Ok(())
}

type ItemSet = HashMap<ItemType, usize>;

#[derive(Clone)]
struct Recipe {
    input: ItemSet,
    input_fluid: Option<FluidType>,
    output: ItemSet,
    output_fluid: Option<FluidType>,
    power_cost: f64,
    recipe_time: f64,
}

impl Recipe {
    fn new(input: ItemSet, output: ItemSet, power_cost: f64, recipe_time: f64) -> Self {
        Recipe {
            input,
            input_fluid: None,
            output,
            output_fluid: None,
            power_cost,
            recipe_time,
        }
    }
}

#[derive(Serialize)]
struct RecipeSerial {
    input: HashMap<String, usize>,
    output: HashMap<String, usize>,
    power_cost: f64,
    recipe_time: f64,
}

impl From<Recipe> for RecipeSerial {
    fn from(o: Recipe) -> Self {
        Self {
            input: o.input.iter().map(|(k, v)| (item_to_str(k), *v)).collect(),
            output: o.output.iter().map(|(k, v)| (item_to_str(k), *v)).collect(),
            power_cost: o.power_cost,
            recipe_time: o.recipe_time,
        }
    }
}

const objsize: i32 = 8;

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

struct ImageBundle {
    url: String,
    bitmap: ImageBitmap,
}

impl<'a> Into<&'a ImageBitmap> for &'a ImageBundle {
    fn into(self) -> &'a ImageBitmap {
        &self.bitmap
    }
}

struct TempEnt {
    position: (f64, f64),
    rotation: f64,
    life: f64,
    max_life: f64,
}

impl TempEnt {
    fn new(rng: &mut Xor128, position: Position) -> Self {
        let life = rng.next() * 3. + 6.;
        TempEnt {
            position: (
                (position.x as f64 + 0.5 + rng.next() * 0.5) * 32.,
                (position.y as f64 + rng.next() * 0.5) * 32.,
            ),
            rotation: rng.next() * std::f64::consts::PI * 2.,
            life,
            max_life: life,
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
    temp_ents: Vec<TempEnt>,
    rng: Xor128,

    // rendering states
    cursor: Option<[i32; 2]>,
    info_elem: Option<HtmlDivElement>,
    on_player_update: js_sys::Function,
    // on_show_inventory: js_sys::Function,
    image_dirt: Option<ImageBundle>,
    image_ore: Option<ImageBundle>,
    image_coal: Option<ImageBundle>,
    image_copper: Option<ImageBundle>,
    image_belt: Option<ImageBundle>,
    image_chest: Option<ImageBundle>,
    image_mine: Option<ImageBundle>,
    image_furnace: Option<ImageBundle>,
    image_assembler: Option<ImageBundle>,
    image_boiler: Option<ImageBundle>,
    image_steam_engine: Option<ImageBundle>,
    image_water_well: Option<ImageBundle>,
    image_pipe: Option<ImageBundle>,
    image_inserter: Option<ImageBundle>,
    image_direction: Option<ImageBundle>,
    image_iron_ore: Option<ImageBundle>,
    image_coal_ore: Option<ImageBundle>,
    image_copper_ore: Option<ImageBundle>,
    image_iron_plate: Option<ImageBundle>,
    image_copper_plate: Option<ImageBundle>,
    image_gear: Option<ImageBundle>,
    image_copper_wire: Option<ImageBundle>,
    image_circuit: Option<ImageBundle>,
    image_time: Option<ImageBundle>,
    image_smoke: Option<ImageBundle>,
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
                    (ItemType::Assembler, 3usize),
                    (ItemType::Boiler, 3usize),
                    (ItemType::WaterWell, 1usize),
                    (ItemType::Pipe, 15usize),
                    (ItemType::SteamEngine, 2usize),
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
            image_assembler: None,
            image_boiler: None,
            image_steam_engine: None,
            image_water_well: None,
            image_pipe: None,
            image_inserter: None,
            image_direction: None,
            image_iron_ore: None,
            image_coal_ore: None,
            image_copper_ore: None,
            image_iron_plate: None,
            image_copper_plate: None,
            image_gear: None,
            image_copper_wire: None,
            image_circuit: None,
            image_time: None,
            image_smoke: None,
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
                Box::new(Furnace::new(&Position::new(8, 3))),
                Box::new(Assembler::new(&Position::new(6, 3))),
                Box::new(WaterWell::new(&Position::new(14, 5))),
                Box::new(Boiler::new(&Position::new(13, 5))),
                Box::new(SteamEngine::new(&Position::new(12, 5))),
            ],
            selected_structure_inventory: None,
            selected_structure_item: None,
            drop_items: vec![],
            serial_no: 0,
            on_player_update,
            temp_ents: vec![],
            rng: Xor128::new(3142125),
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

        struct MutRef<'r, T: ?Sized>(&'r mut T);
        impl<'a, 'r, T: ?Sized> IntoIterator for &'a MutRef<'r, T>
        where
            &'a T: IntoIterator,
        {
            type IntoIter = <&'a T as IntoIterator>::IntoIter;
            type Item = <&'a T as IntoIterator>::Item;
            fn into_iter(self) -> Self::IntoIter {
                self.0.into_iter()
            }
        }
        impl<'a, 'r, T: ?Sized> IntoIterator for &'a mut MutRef<'r, T>
        where
            &'a mut T: IntoIterator,
        {
            type IntoIter = <&'a mut T as IntoIterator>::IntoIter;
            type Item = <&'a mut T as IntoIterator>::Item;
            fn into_iter(self) -> Self::IntoIter {
                self.0.into_iter()
            }
        }

        struct Chained<S, T>(S, T);
        impl<'a, S, T, Item: 'a> IntoIterator for &'a Chained<S, T>
        where
            &'a S: IntoIterator<Item = &'a Item>,
            &'a T: IntoIterator<Item = &'a Item>,
        {
            type IntoIter =
                iter::Chain<<&'a S as IntoIterator>::IntoIter, <&'a T as IntoIterator>::IntoIter>;
            type Item = &'a Item;
            fn into_iter(self) -> Self::IntoIter {
                self.0.into_iter().chain(self.1.into_iter())
            }
        }
        impl<'a, S, T, Item: 'a> IntoIterator for &'a mut Chained<S, T>
        where
            &'a mut S: IntoIterator<Item = &'a mut Item>,
            &'a mut T: IntoIterator<Item = &'a mut Item>,
        {
            type IntoIter = iter::Chain<
                <&'a mut S as IntoIterator>::IntoIter,
                <&'a mut T as IntoIterator>::IntoIter,
            >;
            type Item = &'a mut Item;
            fn into_iter(self) -> Self::IntoIter {
                self.0.into_iter().chain(self.1.into_iter())
            }
        }
        // This is silly way to avoid borrow checker that temporarily move the structures
        // away from self so that they do not claim mutable borrow twice, but it works.
        let mut structures = std::mem::take(&mut self.structures);
        for i in 0..structures.len() {
            let (front, mid) = structures.split_at_mut(i);
            let (center, last) = mid
                .split_first_mut()
                .ok_or(JsValue::from_str("Structures split fail"))?;
            frame_proc_result_to_event(
                center.frame_proc(self, &mut Chained(MutRef(front), MutRef(last))),
            );
        }

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

        // Actually, taking away, filter and collect is easier than removing expied objects
        // one by one.
        self.temp_ents = std::mem::take(&mut self.temp_ents)
            .into_iter()
            .map(|mut ent| {
                ent.position.0 += delta_time * 1.5;
                ent.position.1 -= delta_time * 4.2;
                ent.life -= delta_time;
                ent
            })
            .filter(|ent| 0. < ent.life)
            .collect();

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

    /// Mutable variant of find_structure_tile
    fn find_structure_tile_mut(&mut self, tile: &[i32]) -> Option<&mut Box<dyn Structure>> {
        self.structures
            .iter_mut()
            .find(|s| s.position().x == tile[0] && s.position().y == tile[1])
        // .map(|s| s.as_mut())
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
            let mut structure = self.structures.remove(index);
            for (name, count) in structure.destroy_inventory() {
                self.player.add_item(&name, count)
            }
            self.on_player_update
                .call1(&window(), &JsValue::from(self.get_player_inventory()?))
                .unwrap_or(JsValue::from(true));
            Ok(true)
        } else {
            // Pick up dropped items in the cell
            let mut ret = false;
            while let Some(item_index) = self
                .drop_items
                .iter()
                .position(|item| item.x / 32 == position.x && item.y / 32 == position.y)
            {
                self.player
                    .add_item(&self.drop_items.remove(item_index).type_, 1);
                ret = true;
            }
            Ok(ret)
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

    /// Returns currently selected structure's coordinates in 2-array or `null` if none selected
    pub fn get_selected_inventory(&self) -> Result<JsValue, JsValue> {
        if let Some(pos) = self.selected_structure_inventory {
            return Ok(JsValue::from(js_sys::Array::of2(
                &JsValue::from(pos.x),
                &JsValue::from(pos.y),
            )));
        }
        Ok(JsValue::null())
    }

    /// Returns inventory items in selected tile.
    /// @param c column number.
    /// @param r row number.
    /// @param is_input if true, returns input buffer, otherwise output. Some structures have either one but not both.
    pub fn get_structure_inventory(
        &self,
        c: i32,
        r: i32,
        is_input: bool,
    ) -> Result<js_sys::Array, JsValue> {
        if let Some(structure) = self.find_structure_tile(&[c, r]) {
            if let Some(inventory) = structure.inventory(is_input) {
                return self.get_inventory(inventory, &self.selected_structure_item);
            } else {
                return self.get_inventory(&Inventory::new(), &self.selected_structure_item);
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

    pub fn get_structure_recipes(&self, c: i32, r: i32) -> Result<JsValue, JsValue> {
        if let Some(structure) = self.find_structure_tile(&[c, r]) {
            // Ok(structure.get_recipes()
            //     .iter()
            //     .map(|recipe| {
            //         js_sys::Array::of2(
            //             &recipe.input.iter().map(|pair| js_sys::Object::::of2(
            //                 &JsValue::from_str(&item_to_str(pair.0)),
            //                 &JsValue::from(*pair.1 as f64)
            //             )).collect::<js_sys::Array>(),
            //             &recipe.output.iter().map(|pair| js_sys::Array::of2(
            //                 &JsValue::from_str(&item_to_str(pair.0)),
            //                 &JsValue::from(*pair.1 as f64)
            //             )).collect::<js_sys::Array>(),
            //         )
            //     })
            //     .collect::<js_sys::Array>(),
            // )
            Ok(JsValue::from_serde(
                &structure
                    .get_recipes()
                    .into_iter()
                    .map(|v| RecipeSerial::from(v))
                    .collect::<Vec<_>>(),
            )
            .unwrap())
        } else {
            Err(JsValue::from_str("structure is not found"))
        }
    }

    pub fn select_recipe(&mut self, c: i32, r: i32, index: usize) -> Result<bool, JsValue> {
        if let Some(structure) = self.find_structure_tile_mut(&[c, r]) {
            structure.select_recipe(index)
        } else {
            Err(JsValue::from_str("Structure is not found"))
        }
    }

    fn move_inventory_item(src: &mut Inventory, dst: &mut Inventory, item_type: &ItemType) -> bool {
        if let Some(src_item) = src.remove(item_type) {
            dst.add_items(item_type, src_item);
            true
        } else {
            false
        }
    }

    /// Move inventory items between structure and player
    /// @param to_player whether the movement happen towards player
    /// @param is_input if true, movement is performed to/from input buffer, otherwise output
    pub fn move_selected_inventory_item(
        &mut self,
        to_player: bool,
        is_input: bool,
    ) -> Result<bool, JsValue> {
        if let Some(pos) = self.selected_structure_inventory {
            if let Some(idx) = self.find_structure_tile_idx(&[pos.x, pos.y]) {
                if let Some(inventory) = self.structures[idx].inventory_mut(is_input) {
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
        tool: &ItemType,
        cursor: &Position,
    ) -> Result<Box<dyn Structure>, JsValue> {
        Ok(match tool {
            ItemType::TransportBelt => {
                Box::new(TransportBelt::new(cursor.x, cursor.y, self.tool_rotation))
            }
            ItemType::Inserter => Box::new(Inserter::new(cursor.x, cursor.y, self.tool_rotation)),
            ItemType::OreMine => Box::new(OreMine::new(cursor.x, cursor.y, self.tool_rotation)),
            ItemType::Chest => Box::new(Chest::new(cursor)),
            ItemType::Furnace => Box::new(Furnace::new(cursor)),
            ItemType::Assembler => Box::new(Assembler::new(cursor)),
            ItemType::Boiler => Box::new(Boiler::new(cursor)),
            ItemType::WaterWell => Box::new(WaterWell::new(cursor)),
            ItemType::Pipe => Box::new(Pipe::new(cursor)),
            ItemType::SteamEngine => Box::new(SteamEngine::new(cursor)),
            _ => {
                return Err(JsValue::from_str(&format!(
                    "Can't make a structure from {:?}",
                    tool
                )))
            }
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
                        self.structures.push(
                            self.new_structure(&tool_defs[selected_tool].item_type, &cursor)?,
                        );
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
            } else if let Some(structure) = self.find_structure_tile(&[cursor.x, cursor.y]) {
                if structure.inventory(true).is_some() || structure.inventory(false).is_some() {
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

        let load_image = |path| -> Result<ImageBundle, JsValue> {
            if let Some(value) = image_assets.iter().find(|value| {
                let array = js_sys::Array::from(value);
                array.iter().next() == Some(JsValue::from_str(path))
            }) {
                let array = js_sys::Array::from(&value).to_vec();
                Ok(ImageBundle {
                    url: array
                        .get(1)
                        .map(|v| v.clone())
                        .ok_or_else(|| {
                            JsValue::from_str(&format!(
                                "Couldn't convert value to String: {:?}",
                                path
                            ))
                        })?
                        .dyn_into::<js_sys::JsString>()?
                        .into(),
                    bitmap: array
                        .get(2)
                        .map(|v| v.clone())
                        .ok_or_else(|| {
                            JsValue::from_str(&format!(
                                "Couldn't convert value to ImageBitmap: {:?}",
                                path
                            ))
                        })?
                        .dyn_into::<ImageBitmap>()?,
                })
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
        self.image_assembler = Some(load_image("assembler")?);
        self.image_boiler = Some(load_image("boiler")?);
        self.image_steam_engine = Some(load_image("steamEngine")?);
        self.image_water_well = Some(load_image("waterWell")?);
        self.image_pipe = Some(load_image("pipe")?);
        self.image_inserter = Some(load_image("inserter")?);
        self.image_direction = Some(load_image("direction")?);
        self.image_iron_ore = Some(load_image("ore")?);
        self.image_coal_ore = Some(load_image("coalOre")?);
        self.image_copper_ore = Some(load_image("copperOre")?);
        self.image_iron_plate = Some(load_image("ironPlate")?);
        self.image_copper_plate = Some(load_image("copperPlate")?);
        self.image_gear = Some(load_image("gear")?);
        self.image_copper_wire = Some(load_image("copperWire")?);
        self.image_circuit = Some(load_image("circuit")?);
        self.image_time = Some(load_image("time")?);
        self.image_smoke = Some(load_image("smoke")?);
        Ok(())
    }

    pub fn tool_defs(&self) -> Result<js_sys::Array, JsValue> {
        Ok(tool_defs
            .iter()
            .map(|_| JsValue::null())
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
        context.clear_rect(0., 0., 32., 32.);
        let mut tool =
            self.new_structure(&tool_defs[tool_index].item_type, &Position { x: 0, y: 0 })?;
        tool.set_rotation(&self.tool_rotation).ok();
        for depth in 0..3 {
            tool.draw(self, context, depth)?;
        }
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
                            &img.bitmap,
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
                        draw_ore(
                            self.board[(x + y * self.width) as usize].iron_ore,
                            &img_ore.bitmap,
                        )?;
                        draw_ore(
                            self.board[(x + y * self.width) as usize].coal_ore,
                            &img_coal.bitmap,
                        )?;
                        draw_ore(
                            self.board[(x + y * self.width) as usize].copper_ore,
                            &img_copper.bitmap,
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

        let draw_structures = |depth| -> Result<(), JsValue> {
            for structure in &self.structures {
                structure.draw(&self, &context, depth)?;
            }
            Ok(())
        };

        draw_structures(0)?;

        for item in &self.drop_items {
            render_drop_item(self, &context, &item.type_, item.x, item.y)?;
        }

        draw_structures(1)?;
        draw_structures(2)?;

        for ent in &self.temp_ents {
            if let Some(img) = &self.image_smoke {
                let (x, y) = (ent.position.0 - 24., ent.position.1 - 24.);
                context.save();
                context
                    .set_global_alpha(((ent.max_life - ent.life).min(ent.life) * 0.25).min(0.35));
                context.translate(x + 16., y + 16.)?;
                context.rotate(ent.rotation)?;
                context.draw_image_with_image_bitmap_and_dw_and_dh(
                    &img.bitmap,
                    -16.,
                    -16.,
                    32.,
                    32.,
                )?;
                context.restore();
            }
        }

        if let Some(ref cursor) = self.cursor {
            let (x, y) = ((cursor[0] * 32) as f64, (cursor[1] * 32) as f64);
            if let Some(selected_tool) = self.selected_tool {
                context.save();
                context.set_global_alpha(0.5);
                let mut tool = self
                    .new_structure(&tool_defs[selected_tool].item_type, &Position::from(cursor))?;
                tool.set_rotation(&self.tool_rotation).ok();
                for depth in 0..3 {
                    tool.draw(self, &context, depth)?;
                }
                context.restore();
            }
            context.set_stroke_style(&JsValue::from_str("blue"));
            context.set_line_width(2.);
            context.stroke_rect(x, y, 32., 32.);
        }

        Ok(())
    }
}
