#![allow(non_upper_case_globals)]

macro_rules! console_log {
    ($fmt:expr, $($arg1:expr),*) => {
        crate::log(&format!($fmt, $($arg1),+))
    };
    ($fmt:expr) => {
        crate::log($fmt)
    }
}

/// format-like macro that returns js_sys::String
macro_rules! js_str {
    ($fmt:expr, $($arg1:expr),*) => {
        JsValue::from_str(&format!($fmt, $($arg1),+))
    };
    ($fmt:expr) => {
        JsValue::from_str($fmt)
    }
}

/// format-like macro that returns Err(js_sys::String)
macro_rules! js_err {
    ($fmt:expr, $($arg1:expr),*) => {
        Err(JsValue::from_str(&format!($fmt, $($arg1),+)))
    };
    ($fmt:expr) => {
        Err(JsValue::from_str($fmt))
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
mod dyn_iter;
mod elect_pole;
mod furnace;
mod inserter;
mod items;
mod offshore_pump;
mod ore_mine;
mod perlin_noise;
mod pipe;
mod power_network;
mod splitter;
mod steam_engine;
mod structure;
mod transport_belt;
mod utils;
mod water_well;

use assembler::Assembler;
use boiler::Boiler;
use chest::Chest;
use dyn_iter::{Chained, DynIterMut, MutRef};
use elect_pole::ElectPole;
use furnace::Furnace;
use inserter::Inserter;
use items::{item_to_str, render_drop_item, str_to_item, DropItem, ItemType};
use offshore_pump::OffshorePump;
use ore_mine::OreMine;
use perlin_noise::{gen_terms, perlin_noise_pixel, Xor128};
use pipe::Pipe;
use power_network::{build_power_networks, PowerNetwork};
use splitter::Splitter;
use steam_engine::SteamEngine;
use structure::{
    FrameProcResult, ItemResponse, Position, Rotation, Structure, StructureBoxed, StructureDynIter,
    StructureEntry, StructureId,
};
use transport_belt::TransportBelt;
use water_well::{FluidType, WaterWell};

use serde::{Deserialize, Serialize};
use std::{cell::RefCell, collections::HashMap, convert::TryFrom};
use wasm_bindgen::prelude::*;
use wasm_bindgen::{Clamped, JsCast};
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, HtmlDivElement, ImageBitmap, ImageData,
};

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

const TILE_SIZE: f64 = 32.;
const TILE_SIZE_I: i32 = TILE_SIZE as i32;
const DROP_ITEM_SIZE: f64 = 8.;
const DROP_ITEM_SIZE_I: i32 = DROP_ITEM_SIZE as i32;

const COAL_POWER: f64 = 100.; // kilojoules
const SAVE_VERSION: i64 = 3;
const ORE_HARVEST_TIME: i32 = 20;
const POPUP_TEXT_LIFE: i32 = 30;

#[derive(Copy, Clone, Serialize, Deserialize)]
struct Cell {
    water: bool,
    iron_ore: u32,
    coal_ore: u32,
    copper_ore: u32,
    #[serde(skip)]
    image: u8,
    #[serde(skip)]
    grass_image: u8,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            water: false,
            iron_ore: 0,
            coal_ore: 0,
            copper_ore: 0,
            image: 0,
            grass_image: 0,
        }
    }
}

impl Cell {
    fn get_ore_type(&self) -> Option<ItemType> {
        if 0 < self.iron_ore {
            Some(ItemType::IronOre)
        } else if 0 < self.copper_ore {
            Some(ItemType::CopperOre)
        } else if 0 < self.coal_ore {
            Some(ItemType::CoalOre)
        } else {
            None
        }
    }
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

#[derive(PartialEq, Debug, Serialize, Deserialize)]
enum InventoryType {
    Input,
    Output,
    Burner,
}

impl TryFrom<JsValue> for InventoryType {
    type Error = JsValue;
    fn try_from(value: JsValue) -> Result<Self, JsValue> {
        value.into_serde().map_err(|e| js_str!("{}", e.to_string()))
    }
}

const tilesize: i32 = 32;
struct ToolDef {
    item_type: ItemType,
    desc: &'static str,
}
const tool_defs: [ToolDef; 13] = [
    ToolDef {
        item_type: ItemType::TransportBelt,
        desc: "Transports items on ground",
    },
    ToolDef {
        item_type: ItemType::Inserter,
        desc: "Picks items from one side and puts on the other side<br>in the direction indicated by an arrow.<br>Costs no energy to operate.",
    },
    ToolDef {
        item_type: ItemType::Splitter,
        desc: "Connects to transport belt. Splits inputs and outputs into two lanes.",
    },
    ToolDef {
        item_type: ItemType::OreMine,
        desc: "Mines ores and puts them to adjacent ground<br>or a structure in the direction indicated by an arrow.<br>Requires coal ores to operate.",
    },
    ToolDef {
        item_type: ItemType::Chest,
        desc: "Can store 100 items.<br>Use inserters to automatically store/retrieve items.",
    },
    ToolDef {
        item_type: ItemType::Furnace,
        desc: "Smelts metal ores into metal bars.<br>Requires coal ores to operate.",
    },
    ToolDef {
        item_type: ItemType::Assembler,
        desc: "Assembles items from ingredients with recipes.<br>Set a recipe in the inventory GUI to operate.<br>Requires electricity to operate.",
    },
    ToolDef {
        item_type: ItemType::Boiler,
        desc: "Burns coal ores and use the generated heat to convert water into steam.",
    },
    ToolDef {
        item_type: ItemType::WaterWell,
        desc: "Pumps underground water at a fixed rate of 0.01 units per tick.",
    },
    ToolDef {
        item_type: ItemType::OffshorePump,
        desc: "Pumps water from coastline.",
    },
    ToolDef {
        item_type: ItemType::Pipe,
        desc: "Conveys fluid such as water or steam.",
    },
    ToolDef {
        item_type: ItemType::SteamEngine,
        desc: "Consumes steam and transmits electricity within a range of 3 tiles.",
    },
    ToolDef {
        item_type: ItemType::ElectPole,
        desc: "Electric pole.",
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

#[derive(Clone, Serialize, Deserialize)]
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

#[derive(Serialize, Deserialize)]
struct Player {
    inventory: Inventory,
}

impl Player {
    fn add_item(&mut self, name: &ItemType, count: usize) {
        self.inventory.add_items(name, count);
    }
}

struct ImageBundle {
    url: String,
    bitmap: ImageBitmap,
}

impl<'a> From<&'a ImageBundle> for &'a ImageBitmap {
    fn from(o: &'a ImageBundle) -> Self {
        &o.bitmap
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

#[derive(Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize, Debug)]
struct PowerWire(StructureId, StructureId);

struct PopupText {
    text: String,
    x: f64,
    y: f64,
    life: i32,
}

#[derive(Eq, PartialEq, Copy, Clone, Debug)]
enum SelectedItem {
    /// This is index into `tool_belt`. It is kind of duplicate of `player.selected_item`,
    /// but we make it separate field because multiple tool belt slots refer to the same item type.
    ToolBelt(usize),
    PlayerInventory(ItemType),
    StructInventory(Position, ItemType),
}

impl SelectedItem {
    fn map_struct(&self, position: &Position) -> Option<ItemType> {
        if let SelectedItem::StructInventory(self_pos, item) = self {
            if self_pos == position {
                Some(*item)
            } else {
                None
            }
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
struct OreHarvesting {
    pos: Position,
    ore_type: ItemType,
    timer: i32,
}

fn calculate_back_image(ret: &mut [Cell], width: u32, height: u32) {
    let mut rng = Xor128::new(23424321);
    // Some number with fractional part is desirable, but we don't care too precisely since it is just a visual aid.
    let noise_scale = 3.75213;
    let bits = 1;
    let grass_terms = gen_terms(&mut rng, bits);
    for uy in 0..height {
        let y = uy as i32;
        for ux in 0..width {
            let x = ux as i32;
            if ret[(ux + uy * width) as usize].water {
                ret[(ux + uy * width) as usize].image = 15;
                continue;
            }
            let get_at = |x: i32, y: i32| {
                if x < 0 || width as i32 <= x || y < 0 || height as i32 <= y {
                    false
                } else {
                    ret[(x as u32 + y as u32 * width) as usize].water
                }
            };
            let l = get_at(x - 1, y) as u8;
            let t = get_at(x, y - 1) as u8;
            let r = get_at(x + 1, y) as u8;
            let b = get_at(x, y + 1) as u8;
            let lt = get_at(x - 1, y - 1) as u8;
            let rt = get_at(x + 1, y - 1) as u8;
            let rb = get_at(x + 1, y + 1) as u8;
            let lb = get_at(x - 1, y + 1) as u8;
            let neighbor = l | (t << 1) | (r << 2) | (b << 3);
            let diagonal = lt | (rt << 1) | (rb << 2) | (lb << 3);
            let cell = &mut ret[(ux + uy * width) as usize];
            cell.image = if neighbor != 0 {
                neighbor
            } else if diagonal != 0 {
                diagonal | (1 << 4)
            } else {
                0
            };

            cell.grass_image = ((perlin_noise_pixel(
                x as f64 / noise_scale,
                y as f64 / noise_scale,
                bits,
                &grass_terms,
            ) - 0.)
                * 4.
                * 6.)
                .max(0.)
                .min(6.) as u8;
        }
    }
}

#[wasm_bindgen]
pub struct FactorishState {
    #[allow(dead_code)]
    delta_time: f64,
    sim_time: f64,
    width: u32,
    height: u32,
    viewport_width: f64,
    viewport_height: f64,
    viewport_x: f64,
    viewport_y: f64,
    view_scale: f64,
    board: Vec<Cell>,
    structures: Vec<StructureEntry>,
    selected_structure_inventory: Option<Position>,
    drop_items: Vec<DropItem>,
    serial_no: u32,
    tool_belt: [Option<ItemType>; 10],
    power_networks: Vec<PowerNetwork>,

    selected_item: Option<SelectedItem>,
    ore_harvesting: Option<OreHarvesting>,

    tool_rotation: Rotation,
    player: Player,
    temp_ents: Vec<TempEnt>,
    rng: Xor128,

    // rendering states
    cursor: Option<[i32; 2]>,
    info_elem: Option<HtmlDivElement>,
    on_player_update: js_sys::Function,
    minimap_buffer: RefCell<Vec<u8>>,
    power_wires: Vec<PowerWire>,
    popup_texts: Vec<PopupText>,
    debug_bbox: bool,
    debug_fluidbox: bool,
    debug_power_network: bool,

    // on_show_inventory: js_sys::Function,
    image_dirt: Option<ImageBundle>,
    image_back_tiles: Option<ImageBundle>,
    image_weeds: Option<ImageBundle>,
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
    image_offshore_pump: Option<ImageBundle>,
    image_pipe: Option<ImageBundle>,
    image_elect_pole: Option<ImageBundle>,
    image_splitter: Option<ImageBundle>,
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
    image_fuel_alarm: Option<ImageBundle>,
    image_electricity_alarm: Option<ImageBundle>,
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
        width: u32,
        height: u32,
        on_player_update: js_sys::Function,
        // on_show_inventory: js_sys::Function,
        terrain_seed: u32,
        water_noise_threshold: f64,
        resource_amount: f64,
        noise_scale: f64,
        noise_threshold: f64,
    ) -> Result<FactorishState, JsValue> {
        console_log!("FactorishState constructor");

        fn wrap_structure(s: StructureBoxed) -> StructureEntry {
            StructureEntry {
                gen: 0,
                dynamic: Some(s),
            }
        }

        let mut tool_belt = [None; 10];
        tool_belt[0] = Some(ItemType::OreMine);
        tool_belt[1] = Some(ItemType::Inserter);
        tool_belt[2] = Some(ItemType::TransportBelt);
        tool_belt[3] = Some(ItemType::Furnace);

        let mut structures: Vec<StructureEntry> = vec![
            wrap_structure(Box::new(TransportBelt::new(10, 3, Rotation::Left))),
            wrap_structure(Box::new(TransportBelt::new(11, 3, Rotation::Left))),
            wrap_structure(Box::new(TransportBelt::new(12, 3, Rotation::Left))),
            wrap_structure(Box::new(OreMine::new(12, 2, Rotation::Bottom))),
            wrap_structure(Box::new(Furnace::new(&Position::new(8, 3)))),
            wrap_structure(Box::new(Assembler::new(&Position::new(6, 3)))),
            wrap_structure(Box::new(WaterWell::new(&Position::new(14, 5)))),
            wrap_structure(Box::new(Boiler::new(&Position::new(13, 5)))),
            wrap_structure(Box::new(SteamEngine::new(&Position::new(12, 5)))),
        ];
        use std::iter::Iterator;
        structures.extend((10..=100).filter_map(|x| {
            if x % 2 == 0 {
                let p = Box::new(Assembler::new(&Position::new(x, 10)));
                Some(wrap_structure(p as Box<dyn Structure>))
            } else {
                let p = Box::new(ElectPole::new(&Position::new(x, 10)));
                Some(wrap_structure(p as Box<dyn Structure>))
            }
        }));
        structures.extend((10..=100).map(|x| {
            wrap_structure(if x % 2 == 0 {
                Box::new(Assembler::new(&Position::new(x, 100))) as Box<dyn Structure>
            } else {
                Box::new(ElectPole::new(&Position::new(x, 100))) as Box<dyn Structure>
            })
        }));
        structures.extend(
            (11..=99).map(|x| {
                if x % 2 == 0 {
                    wrap_structure(
                        Box::new(Assembler::new(&Position::new(10, x))) as Box<dyn Structure>
                    )
                } else {
                    wrap_structure(
                        Box::new(ElectPole::new(&Position::new(10, x))) as Box<dyn Structure>
                    )
                }
            }),
        );
        structures.extend((11..=99).map(|x| {
            wrap_structure(if x % 2 == 0 {
                Box::new(Assembler::new(&Position::new(100, x))) as Box<dyn Structure>
            } else {
                Box::new(ElectPole::new(&Position::new(100, x))) as Box<dyn Structure>
            })
        }));

        let mut ret = FactorishState {
            delta_time: 0.1,
            sim_time: 0.0,
            width,
            height,
            viewport_height: 0.,
            viewport_width: 0.,
            viewport_x: 0.,
            viewport_y: 0.,
            view_scale: 1.,
            cursor: None,
            tool_belt,
            selected_item: None,
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
                    (ItemType::OffshorePump, 2usize),
                    (ItemType::Pipe, 15usize),
                    (ItemType::SteamEngine, 2usize),
                ]
                .iter()
                .copied()
                .collect(),
            },
            info_elem: None,
            minimap_buffer: RefCell::new(vec![]),
            power_wires: vec![],
            power_networks: vec![],
            popup_texts: vec![],
            debug_bbox: false,
            debug_fluidbox: false,
            debug_power_network: false,
            image_dirt: None,
            image_back_tiles: None,
            image_weeds: None,
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
            image_offshore_pump: None,
            image_pipe: None,
            image_elect_pole: None,
            image_splitter: None,
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
            image_fuel_alarm: None,
            image_electricity_alarm: None,
            board: {
                let mut ret = vec![Cell::default(); (width * height) as usize];
                let bits = 1;
                let mut rng = Xor128::new(terrain_seed);
                let ocean_terms = gen_terms(&mut rng, bits);
                let iron_terms = gen_terms(&mut rng, bits);
                let copper_terms = gen_terms(&mut rng, bits);
                let coal_terms = gen_terms(&mut rng, bits);
                for y in 0..height {
                    for x in 0..width {
                        let [fx, fy] = [x as f64 / noise_scale, y as f64 / noise_scale];
                        let cell = &mut ret[(x + y * width) as usize];
                        cell.water =
                            water_noise_threshold < perlin_noise_pixel(fx, fy, bits, &ocean_terms);
                        if cell.water {
                            continue; // No ores in water
                        }
                        let iron = (perlin_noise_pixel(fx, fy, bits, &iron_terms)
                            - noise_threshold)
                            * 4.
                            * resource_amount;
                        let copper = (perlin_noise_pixel(fx, fy, bits, &copper_terms)
                            - noise_threshold)
                            * 4.
                            * resource_amount;
                        let coal = (perlin_noise_pixel(fx, fy, bits, &coal_terms)
                            - noise_threshold)
                            * 4.
                            * resource_amount;

                        match [iron, copper, coal]
                            .iter()
                            .map(|v| v.max(0.) as u32)
                            .enumerate()
                            .max_by_key(|v| v.1)
                        {
                            Some((0, v)) => cell.iron_ore = v,
                            Some((1, v)) => cell.copper_ore = v,
                            Some((2, v)) => cell.coal_ore = v,
                            _ => (),
                        }
                    }
                }
                calculate_back_image(&mut ret, width, height);
                ret
            },
            structures,
            selected_structure_inventory: None,
            ore_harvesting: None,
            drop_items: vec![],
            serial_no: 0,
            on_player_update,
            temp_ents: vec![],
            rng: Xor128::new(3142125),
            // on_show_inventory,
        };

        let positions = ret
            .structures
            .iter()
            .map(|s| s.dynamic.as_deref().map(|d| *d.position()))
            .flatten()
            .collect::<Vec<_>>();
        for position in positions {
            ret.update_fluid_connections(&position).unwrap();
        }

        for s in ret
            .structures
            .iter_mut()
            .filter_map(|s| s.dynamic.as_deref_mut())
        {
            s.select_recipe(0).ok();
        }

        let structures = std::mem::take(&mut ret.structures);
        for (i, structure1) in structures
            .iter()
            .enumerate()
            .filter_map(|(i, s)| Some((i, s.dynamic.as_deref()?)))
        {
            for structure2 in structures[i + 1..]
                .iter()
                .filter_map(|s| s.dynamic.as_deref())
            {
                if (structure1.power_sink() && structure2.power_source()
                    || structure1.power_source() && structure2.power_sink())
                    && structure1.position().distance(structure2.position())
                        <= structure1.wire_reach().min(structure2.wire_reach()) as i32
                {
                    ret.add_power_wire(PowerWire(*structure1.position(), *structure2.position()))?;
                }
            }
        }
        ret.structures = structures;

        for i in 0..ret.structures.len() {
            let (s, others) = StructureDynIter::new(&mut ret.structures, i)?;
            let id = StructureId {
                id: i as u32,
                gen: s.gen,
            };
            s.dynamic
                .as_deref_mut()
                .map(|d| d.on_construction_self(id, &others, true))
                .unwrap_or(Ok(()))?;
        }

        Ok(ret)
    }

    pub fn serialize_game(&self) -> Result<String, JsValue> {
        use serde_json::Value as SValue;
        console_log!("Serializing...");
        fn map_err(
            result: Result<SValue, serde_json::Error>,
            name: &str,
        ) -> Result<SValue, JsValue> {
            result.map_err(|e| js_str!("serialize failed for {}: {}", name, e))
        }
        let mut map = serde_json::Map::new();
        map.insert(
            "version".to_string(),
            map_err(serde_json::to_value(&SAVE_VERSION), "version")?,
        );
        map.insert(
            "sim_time".to_string(),
            serde_json::Value::from(self.sim_time),
        );
        map.insert(
            "player".to_string(),
            map_err(serde_json::to_value(&self.player), "player")?,
        );
        map.insert("width".to_string(), serde_json::Value::from(self.width));
        map.insert("height".to_string(), serde_json::Value::from(self.height));
        map.insert(
            "structures".to_string(),
            serde_json::Value::from(
                self.structures
                    .iter()
                    .filter_map(|entry| entry.dynamic.as_ref())
                    .map(|structure| {
                        let mut map = serde_json::Map::new();
                        map.insert(
                            "type".to_string(),
                            serde_json::Value::String(structure.name().to_string()),
                        );
                        map.insert(
                            "payload".to_string(),
                            structure
                                .serialize()
                                .map_err(|e| js_str!("Serialize error: {}", e))?,
                        );
                        Ok(serde_json::Value::Object(map))
                    })
                    .collect::<Result<Vec<serde_json::Value>, JsValue>>()?,
            ),
        );

        // This mapping is necessary to fill the gaps from deleted structures since we only serialize live structures.
        let id_to_index = self
            .structures
            .iter()
            .enumerate()
            .map(|(id, s)| {
                (
                    StructureId {
                        id: id as u32,
                        gen: s.gen,
                    },
                    s,
                )
            })
            .filter(|(_, s)| s.dynamic.is_some())
            .enumerate()
            .map(|(idx, (id, _))| (id, idx))
            .collect::<HashMap<_, _>>();
        map.insert(
            "power_wires".to_string(),
            serde_json::to_value(
                &self
                    .power_wires
                    .iter()
                    .filter_map(|w| Some((id_to_index.get(&w.0)?, id_to_index.get(&w.1)?)))
                    .collect::<Vec<_>>(),
            )
            .map_err(|e| js_str!("Serialize error: {}", e))?,
        );

        map.insert(
            "items".to_string(),
            serde_json::to_value(
                self.drop_items
                    .iter()
                    .map(serde_json::to_value)
                    .collect::<serde_json::Result<Vec<serde_json::Value>>>()
                    .map_err(|e| js_str!("Serialize error: {}", e))?,
            )
            .map_err(|e| js_str!("Serialize error: {}", e))?,
        );
        map.insert(
            "tool_belt".to_string(),
            map_err(serde_json::to_value(self.tool_belt), "toolbelt")?,
        );
        map.insert(
            "board".to_string(),
            serde_json::to_value(
                self.board
                    .iter()
                    .enumerate()
                    .filter(|(_, cell)| {
                        0 < cell.coal_ore || 0 < cell.iron_ore || 0 < cell.copper_ore || cell.water
                    })
                    .map(|(idx, cell)| {
                        let mut map = serde_json::Map::new();
                        let x = idx % self.width as usize;
                        let y = idx / self.height as usize;
                        map.insert("position".to_string(), serde_json::to_value((x, y))?);
                        map.insert("cell".to_string(), serde_json::to_value(cell)?);
                        serde_json::to_value(map)
                    })
                    .collect::<serde_json::Result<Vec<serde_json::Value>>>()
                    .map_err(|e| js_str!("Serialize error on board: {}", e))?,
            )
            .map_err(|e| js_str!("Serialize error on board: {}", e))?,
        );
        serde_json::to_string(&map).map_err(|e| js_str!("Serialize error: {}", e))
    }

    pub fn save_game(&self) -> Result<(), JsValue> {
        if let Some(storage) = window().local_storage()? {
            storage.set_item("FactorishWasmGameSave", &self.serialize_game()?)?;
            Ok(())
        } else {
            js_err!("The subsystem does not support localStorage")
        }
    }

    pub fn deserialize_game(&mut self, data: &str) -> Result<(), JsValue> {
        use serde_json::Value;

        let mut json: Value =
            serde_json::from_str(&data).map_err(|_| js_str!("Deserialize error"))?;

        // Check version first
        let version = if let Some(version) = json.get("version") {
            version
                .as_i64()
                .ok_or_else(|| js_str!("Version string cannot be parsed as int"))?
        } else {
            0
        };

        if version < SAVE_VERSION {
            return js_err!("Save data version is too old. Please start a new game.");
        }

        self.structures.clear();
        self.drop_items.clear();

        fn json_get<I: serde_json::value::Index + std::fmt::Display + Copy>(
            value: &serde_json::Value,
            key: I,
        ) -> Result<&serde_json::Value, JsValue> {
            value.get(key).ok_or_else(|| js_str!("{} not found", key))
        }

        fn json_take<I: serde_json::value::Index + std::fmt::Display + Copy>(
            value: &mut serde_json::Value,
            key: I,
        ) -> Result<serde_json::Value, JsValue> {
            Ok(value
                .get_mut(key)
                .ok_or_else(|| js_str!("{} not found", key))?
                .take())
        }

        fn json_as_u64(value: &serde_json::Value) -> Result<u64, JsValue> {
            value
                .as_u64()
                .ok_or_else(|| js_str!("value could not be converted to u64"))
        }

        fn from_value<T: serde::de::DeserializeOwned>(
            value: serde_json::Value,
        ) -> Result<T, JsValue> {
            serde_json::from_value(value).map_err(|e| js_str!("deserialization error {}", e))
        }

        self.sim_time = json_get(&json, "sim_time")?
            .as_f64()
            .ok_or_else(|| js_str!("sim_time is not float"))?;

        self.player = from_value(json_take(&mut json, "player")?)?;

        self.width = json_as_u64(json_get(&json, "width")?)? as u32;
        self.height = json_as_u64(json_get(&json, "height")?)? as u32;

        let tiles = json
            .get_mut("board")
            .ok_or_else(|| js_str!("board not found in saved data"))?
            .as_array_mut()
            .ok_or_else(|| js_str!("board in saved data is not an array"))?;
        self.board = vec![Cell::default(); (self.width * self.height) as usize];
        for tile in tiles {
            let position = json_get(tile, "position")?;
            let x: usize = json_as_u64(json_get(&position, 0)?)? as usize;
            let y: usize = json_as_u64(json_get(&position, 1)?)? as usize;
            self.board[x + y * self.width as usize] = from_value(json_take(tile, "cell")?)?;
        }

        calculate_back_image(&mut self.board, self.width, self.height);

        let structures = json
            .get_mut("structures")
            .ok_or_else(|| js_str!("structures not found in saved data"))?
            .as_array_mut()
            .ok_or_else(|| js_str!("structures in saved data is not an array"))?
            .iter_mut()
            .map(|structure| {
                Ok(StructureEntry {
                    gen: 0,
                    dynamic: Some(Self::structure_from_json(structure)?),
                })
            })
            .collect::<Result<Vec<StructureEntry>, JsValue>>()?;

        self.power_wires = serde_json::from_value::<Vec<(u32, u32)>>(
            json.get_mut("power_wires")
                .ok_or_else(|| js_str!("power_wires not found in saved data"))?
                .take(),
        )
        .map_err(|e| js_str!("power_wires deserialization error: {}", e))?
        .into_iter()
        .map(|w| {
            PowerWire(
                StructureId {
                    id: w.0 as u32,
                    gen: 0,
                },
                StructureId { id: w.1, gen: 0 },
            )
        })
        .collect();

        self.structures = structures;

        // We need to collect the positions into a temporary Vec to allow passing &mut self to update_fluid_connections
        for pos in self
            .structures
            .iter()
            .filter_map(|s| Some(*s.dynamic.as_deref()?.position()))
            .collect::<Vec<_>>()
        {
            self.update_fluid_connections(&pos)?;
        }

        for i in 0..self.structures.len() {
            let (s, others) = StructureDynIter::new(&mut self.structures, i)?;
            let id = StructureId {
                id: i as u32,
                gen: s.gen,
            };
            s.dynamic
                .as_deref_mut()
                .map(|d| d.on_construction_self(id, &others, true))
                .unwrap_or(Ok(()))?;
        }

        let s_d_iter = StructureDynIter::new_all(&mut self.structures);
        self.power_networks = build_power_networks(&s_d_iter, &self.power_wires);

        self.drop_items = serde_json::from_value(
            json.get_mut("items")
                .ok_or_else(|| js_str!("\"items\" not found"))?
                .take(),
        )
        .map_err(|_| js_str!("drop items deserialization error"))?;

        self.tool_belt = from_value(json_take(&mut json, "tool_belt")?)?;

        // Redraw minimap
        self.render_minimap_data()?;

        Ok(())
    }

    pub fn load_game(&mut self) -> Result<(), JsValue> {
        if let Some(storage) = window().local_storage()? {
            let data = storage
                .get_item("FactorishWasmGameSave")?
                .ok_or_else(|| js_str!("save data not found!"))?;
            self.deserialize_game(&data)
        } else {
            js_err!("The subsystem does not support localStorage")
        }
    }

    #[allow(dead_code)]
    fn proc_structures_mutual(
        &mut self,
        mut f: impl FnMut(
            &mut Self,
            &mut StructureBoxed,
            &dyn DynIterMut<Item = StructureEntry>,
        ) -> Result<(), JsValue>,
    ) -> Result<(), JsValue> {
        // This is silly way to avoid borrow checker that temporarily move the structures
        // away from self so that they do not claim mutable borrow twice, but it works.
        let mut structures = std::mem::take(&mut self.structures);
        let mut res = Ok(());
        for i in 0..structures.len() {
            let (front, mid) = structures.split_at_mut(i);
            let (center, last) = mid
                .split_first_mut()
                .ok_or_else(|| JsValue::from_str("Structures split fail"))?;
            if let Some(d) = center.dynamic.as_mut() {
                let other_structures = Chained(MutRef(front), MutRef(last));
                // let mut other_structures = dyn_iter::FilterMapped(|s: &mut StructureEntry| s.dynamic);
                // let mut o = &other_structures as &dyn DynIterMut<Item = StructureBoxed>;
                res = f(self, d, &other_structures);
                if res.is_err() {
                    break;
                }
            }
        }
        self.structures = structures;
        res
    }

    fn get_pair_mut(
        &mut self,
        a: usize,
        b: usize,
    ) -> (
        Option<(StructureId, &mut StructureBoxed)>,
        Option<(StructureId, &mut StructureBoxed)>,
    ) {
        if a < b {
            let (left, right) = self.structures.split_at_mut(b);
            let a_gen = left[a].gen;
            (
                left[a].dynamic.as_mut().map(|s| {
                    (
                        StructureId {
                            id: a as u32,
                            gen: a_gen,
                        },
                        s,
                    )
                }),
                right
                    .first_mut()
                    .map(|s| {
                        Some((
                            StructureId {
                                id: b as u32,
                                gen: s.gen,
                            },
                            s.dynamic.as_mut()?,
                        ))
                    })
                    .flatten(),
            )
        } else if b < a {
            let (left, right) = self.structures.split_at_mut(a);
            let b_gen = left[b].gen;
            (
                right
                    .first_mut()
                    .map(|s| {
                        Some((
                            StructureId {
                                id: a as u32,
                                gen: s.gen,
                            },
                            s.dynamic.as_mut()?,
                        ))
                    })
                    .flatten(),
                left[b].dynamic.as_mut().map(|s| {
                    (
                        StructureId {
                            id: b as u32,
                            gen: b_gen,
                        },
                        s,
                    )
                }),
            )
        } else {
            (None, None)
        }
    }

    fn get_structure(&self, id: StructureId) -> Option<&dyn Structure> {
        self.structures.iter()
            .enumerate()
            .find(|(i, s)| id.id == *i as u32 && id.gen == s.gen)
            .map(|(_, s)| s.dynamic.as_deref())
            .flatten()
    }

    fn update_fluid_connections(&mut self, position: &Position) -> Result<(), JsValue> {
        if let Some(i) = self
            .structures
            .iter()
            .enumerate()
            .find(|s| {
                s.1.dynamic
                    .as_deref()
                    .map(|a| *a.position() == *position && a.fluid_box().is_some())
                    .unwrap_or(false)
            })
            .map(|v| v.0)
        {
            for j in 0..self.structures.len() {
                if i != j {
                    if let (Some(a), Some(b)) = self.get_pair_mut(i, j) {
                        let (aid, bid) = (a.0, b.0);
                        if let Some(((idx, mut av), mut bv)) =
                            a.1.position()
                                .neighbor_index(b.1.position())
                                .zip(a.1.fluid_box_mut())
                                .zip(b.1.fluid_box_mut())
                        {
                            av.iter_mut()
                                .for_each(|fb| fb.connect_to[(idx as usize + 2) % 4] = Some(bid));
                            bv.iter_mut()
                                .for_each(|fb| fb.connect_to[idx as usize] = Some(aid));
                        }
                    }
                }
            }
        } else {
            for j in 0..self.structures.len() {
                if let Some((idx, b)) = self
                    .structures
                    .get_mut(j)
                    .map(|s| s.dynamic.as_deref_mut())
                    .flatten()
                    .map(|s| Some((position.neighbor_index(s.position())?, s)))
                    .flatten()
                {
                    if let Some(mut bv) = b.fluid_box_mut() {
                        bv.iter_mut()
                            .for_each(|fb| fb.connect_to[idx as usize] = None);
                    }
                }
            }
        }

        Ok(())
    }

    pub fn simulate(&mut self, delta_time: f64) -> Result<js_sys::Array, JsValue> {
        // console_log!("simulating delta_time {}, {}", delta_time, self.sim_time);
        const SERIALIZE_PERIOD: f64 = 100.;
        if (self.sim_time / SERIALIZE_PERIOD).floor()
            < ((self.sim_time + delta_time) / SERIALIZE_PERIOD).floor()
        {
            self.save_game()?;
        }

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

        self.ore_harvesting = if let Some(mut ore_harvesting) = self.ore_harvesting {
            let mut ret = true;
            if (ore_harvesting.timer + 1) % ORE_HARVEST_TIME < ore_harvesting.timer {
                console_log!("harvesting {:?}...", ore_harvesting.ore_type);
                if let Some(tile) = self.tile_at_mut(&ore_harvesting.pos) {
                    if let Some(ore) = match ore_harvesting.ore_type {
                        ItemType::IronOre => Some(&mut tile.iron_ore),
                        ItemType::CopperOre => Some(&mut tile.copper_ore),
                        ItemType::CoalOre => Some(&mut tile.coal_ore),
                        _ => None,
                    } {
                        if 0 < *ore {
                            *ore -= 1;
                            self.player.add_item(&ore_harvesting.ore_type, 1);
                            self.on_player_update
                                .call1(&window(), &JsValue::from(self.get_player_inventory()?))
                                .unwrap_or_else(|_| JsValue::from(true));
                            self.new_popup_text(
                                format!("+1 {:?}", ore_harvesting.ore_type),
                                ore_harvesting.pos.x as f64 * TILE_SIZE,
                                ore_harvesting.pos.y as f64 * TILE_SIZE,
                            );
                        } else {
                            ret = false;
                        }
                    }
                }
            }
            ore_harvesting.timer = (ore_harvesting.timer + 1) % ORE_HARVEST_TIME;
            if ret {
                Some(ore_harvesting)
            } else {
                None
            }
        } else {
            None
        };

        let mut delete_me = vec![];
        for (i, item) in self.popup_texts.iter_mut().enumerate() {
            if item.life <= 0 {
                delete_me.push(i);
            } else {
                item.y -= 1.;
                item.life -= 1;
            }
        }

        for i in delete_me.iter().rev() {
            self.popup_texts.remove(*i);
        }

        // This is silly way to avoid borrow checker that temporarily move the structures
        // away from self so that they do not claim mutable borrow twice, but it works.
        let mut structures = std::mem::take(&mut self.structures);
        for i in 0..structures.len() {
            let (center, mut dyn_iter) = StructureDynIter::new(&mut structures, i)?;
            if let Some(dynamic) = center.dynamic.as_deref_mut() {
                frame_proc_result_to_event(
                    dynamic.frame_proc(
                        StructureId {
                            id: i as u32,
                            gen: center.gen,
                        },
                        self,
                        &mut dyn_iter,
                    ), // dynamic.frame_proc(self, &mut Chained(MutRef(front), MutRef(last)))
                );
            }
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
                    .filter_map(|s| s.dynamic.as_mut())
                    .find(|s| {
                        s.contains(&Position {
                            x: item.x / TILE_SIZE_I,
                            y: item.y / TILE_SIZE_I,
                        })
                    })
                    .and_then(|structure| structure.item_response(item).ok())
                {
                    match item_response_result.0 {
                        ItemResponse::Move(moved_x, moved_y) => {
                            if self.hit_check(moved_x, moved_y, Some(item.id)) {
                                continue;
                            }
                            let position = Position {
                                x: moved_x / 32,
                                y: moved_y / 32,
                            };
                            if let Some(s) = structures
                                .iter()
                                .filter_map(|s| s.dynamic.as_deref())
                                .find(|s| s.contains(&position))
                            {
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

    fn tile_at(&self, tile: &Position) -> Option<Cell> {
        if 0 <= tile.x && tile.x < self.width as i32 && 0 <= tile.y && tile.y < self.height as i32 {
            Some(self.board[tile.x as usize + tile.y as usize * self.width as usize])
        } else {
            None
        }
    }

    fn tile_at_mut(&mut self, tile: &Position) -> Option<&mut Cell> {
        if 0 <= tile.x && tile.x < self.width as i32 && 0 <= tile.y && tile.y < self.height as i32 {
            Some(&mut self.board[tile.x as usize + tile.y as usize * self.width as usize])
        } else {
            None
        }
    }

    /// Look up a structure at a given tile coordinates
    fn find_structure_tile(&self, tile: &[i32]) -> Option<&dyn Structure> {
        self.structure_iter()
            .find(|s| s.position().x == tile[0] && s.position().y == tile[1])
    }

    /// Mutable variant of find_structure_tile
    fn find_structure_tile_mut(&mut self, tile: &[i32]) -> Option<&mut Box<dyn Structure>> {
        self.structures
            .iter_mut()
            .filter_map(|s| s.dynamic.as_mut())
            .find(|s| s.position().x == tile[0] && s.position().y == tile[1])
        // .map(|s| s.as_mut())
    }

    /// Dirty hack to enable modifying a structure in an array.
    /// Instead of returning mutable reference, return an index into the array, so the
    /// caller can directly reference the structure from array `self.structures[idx]`.
    ///
    /// Because mutable version of find_structure_tile doesn't work.
    fn find_structure_tile_idx(&self, tile: &[i32]) -> Option<usize> {
        self.structure_iter()
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
            Some(self.drop_items.remove(i))
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
            if (x - item.x).abs() < DROP_ITEM_SIZE_I && (y - item.y).abs() < DROP_ITEM_SIZE_I {
                return true;
            }
        }
        false
    }

    fn rotate(&mut self) -> Result<bool, RotateErr> {
        if let Some(SelectedItem::ToolBelt(_selected_tool)) = self.selected_item {
            self.tool_rotation = self.tool_rotation.next();
            Ok(true)
        } else {
            if let Some(ref cursor) = self.cursor {
                if let Some(idx) = self.find_structure_tile_idx(cursor) {
                    if let Some(d) = self.structures[idx].dynamic.as_mut() {
                        d.as_mut()
                            .rotate()
                            .map_err(|()| RotateErr::NotSupported)
                            .map(|_| false)?;
                    }
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

    fn harvest(&mut self, position: &Position, clear_item: bool) -> Result<bool, JsValue> {
        let mut harvested_structure = false;
        let mut popup_text = String::new();
        for i in 0..self.structures.len() {
            if !self.structures[i]
                .dynamic
                .as_deref()
                .map(|d| d.contains(position))
                .unwrap_or(false)
            {
                continue;
            }
            let mut structure = self.structures[i]
                .dynamic
                .take()
                .expect("should be active entity");
            let gen = self.structures[i].gen;
            self.structures[i].gen += 1;
            self.player
                .inventory
                .add_item(&str_to_item(&structure.name()).ok_or_else(|| {
                    JsValue::from_str(&format!("wrong structure name: {:?}", structure.name()))
                })?);
            popup_text += &format!("+1 {}\n", structure.name());
            for notify_structure in &mut self.structures {
                if let Some(s) = notify_structure.dynamic.as_deref_mut() {
                    s.on_construction(
                        StructureId { id: i as u32, gen },
                        structure.as_mut(),
                        false,
                    )?;
                }
            }
            let position = *structure.position();
            self.power_wires = std::mem::take(&mut self.power_wires)
                .into_iter()
                .filter(|power_wire| power_wire.0.id != i as u32 && power_wire.1.id != i as u32)
                .collect();
            structure.on_construction_self(
                StructureId { id: i as u32, gen },
                &StructureDynIter::new_all(&mut self.structures),
                false,
            )?;
            if let Ok(ref mut data) = self.minimap_buffer.try_borrow_mut() {
                self.render_minimap_data_pixel(data, &position);
            }
            for (item_type, count) in structure.destroy_inventory() {
                popup_text += &format!("+{} {}\n", count, &item_to_str(&item_type));
                self.player.add_item(&item_type, count)
            }

            self.power_networks = build_power_networks(
                &StructureDynIter::new_all(&mut self.structures),
                &self.power_wires,
            );

            self.update_fluid_connections(&position)?;

            self.on_player_update
                .call1(&window(), &JsValue::from(self.get_player_inventory()?))
                .unwrap_or_else(|_| JsValue::from(true));
            harvested_structure = true;
        }
        let mut harvested_items = false;
        if !harvested_structure && clear_item {
            // Pick up dropped items in the cell
            let mut picked_items = Inventory::new();
            while let Some(item_index) = self.drop_items.iter().position(|item| {
                item.x / TILE_SIZE_I == position.x && item.y / TILE_SIZE_I == position.y
            }) {
                let item_type = self.drop_items.remove(item_index).type_;
                picked_items.add_item(&item_type);
                self.player.add_item(&item_type, 1);
                harvested_items = true;
            }
            for (item_type, count) in picked_items {
                popup_text += &format!("+{} {}\n", count, &item_to_str(&item_type));
            }
        }
        if !popup_text.is_empty() {
            self.new_popup_text(
                popup_text,
                position.x as f64 * TILE_SIZE,
                position.y as f64 * TILE_SIZE,
            );
        }
        Ok(harvested_structure || harvested_items)
    }

    /// @returns 2-array of
    ///          * inventory (object) and
    ///          * selected item (string)
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
                    .unwrap_or_else(|| "".to_string()),
            ),
        ))
    }

    /// Returns [[itemName, itemCount]*, selectedItemName]
    pub fn get_player_inventory(&self) -> Result<js_sys::Array, JsValue> {
        self.get_inventory(
            &self.player.inventory,
            &self.selected_item.and_then(|item| {
                if let SelectedItem::PlayerInventory(i) = item {
                    Some(i)
                } else {
                    None
                }
            }),
        )
    }

    pub fn select_player_inventory(&mut self, name: &str) -> Result<(), JsValue> {
        self.selected_item = Some(SelectedItem::PlayerInventory(
            str_to_item(name).ok_or_else(|| JsValue::from_str("Item name not identified"))?,
        ));
        Ok(())
    }

    /// Deselect is a separate function from select because wasm-bindgen cannot overload Option
    pub fn deselect_player_inventory(&mut self) -> Result<(), JsValue> {
        self.selected_item = None;
        Ok(())
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
    /// @param inventory_type a string indicating type of the inventory in the structure
    pub fn get_structure_inventory(
        &self,
        c: i32,
        r: i32,
        inventory_type: JsValue,
    ) -> Result<js_sys::Array, JsValue> {
        let inventory_type = InventoryType::try_from(inventory_type)?;
        if let Some(structure) = self.find_structure_tile(&[c, r]) {
            match inventory_type {
                InventoryType::Burner => {
                    if let Some(inventory) = structure.burner_inventory() {
                        return self.get_inventory(inventory, &None);
                    } else {
                        return Ok(js_sys::Array::new());
                    }
                }
                _ => {
                    if let Some(inventory) =
                        structure.inventory(inventory_type == InventoryType::Input)
                    {
                        return self.get_inventory(
                            inventory,
                            &self
                                .selected_item
                                .and_then(|item| item.map_struct(&Position { x: c, y: r })),
                        );
                    } else {
                        return Ok(js_sys::Array::new());
                    }
                }
            }
        }
        Err(JsValue::from_str(
            "structure is not found or doesn't have inventory",
        ))
    }

    pub fn get_structure_burner_energy(&self, c: i32, r: i32) -> Option<js_sys::Array> {
        self.find_structure_tile(&[c, r]).and_then(|structure| {
            let (current, max) = structure.burner_energy()?;
            Some(js_sys::Array::of2(
                &JsValue::from(current),
                &JsValue::from(max),
            ))
        })
    }

    pub fn select_structure_inventory(&mut self, name: &str) -> Result<(), JsValue> {
        self.selected_item = Some(SelectedItem::StructInventory(
            self.selected_structure_inventory
                .ok_or_else(|| js_str!("Structure not selected"))?,
            str_to_item(name).ok_or_else(|| JsValue::from("Item name not valid"))?,
        ));
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
                    .map(RecipeSerial::from)
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

    pub fn set_debug_bbox(&mut self, value: bool) {
        self.debug_bbox = value;
    }

    pub fn set_debug_fluidbox(&mut self, value: bool) {
        self.debug_fluidbox = value;
    }

    pub fn set_debug_power_network(&mut self, value: bool) {
        self.debug_power_network = value;
    }

    /// Move inventory items between structure and player
    /// @param to_player whether the movement happen towards player
    /// @param inventory_type a string indicating type of the inventory in the structure
    pub fn move_selected_inventory_item(
        &mut self,
        to_player: bool,
        inventory_type: JsValue,
    ) -> Result<bool, JsValue> {
        let inventory_type = InventoryType::try_from(inventory_type)?;
        let pos = if let Some(pos) = self.selected_structure_inventory {
            pos
        } else {
            return Ok(false);
        };
        if let Some(idx) = self.find_structure_tile_idx(&[pos.x, pos.y]) {
            let structure = self
                .structures
                .get_mut(idx)
                .ok_or_else(|| js_str!("structure out of bounds"))?
                .dynamic
                .as_deref_mut()
                .ok_or_else(|| js_str!("Dead structure"))?;
            match inventory_type {
                InventoryType::Burner => {
                    if to_player {
                        if let Some(burner_inventory) = structure.burner_inventory() {
                            if let Some((&item, &count)) = burner_inventory.iter().next() {
                                self.player.inventory.add_items(
                                    &item,
                                    -structure.add_burner_inventory(&item, -(count as isize))
                                        as usize,
                                );
                                return Ok(true);
                            }
                        }
                    } else {
                        if let Some(SelectedItem::PlayerInventory(i)) = self.selected_item {
                            self.player.inventory.remove_items(
                                &i,
                                structure
                                    .add_burner_inventory(
                                        &i,
                                        self.player.inventory.count_item(&i) as isize,
                                    )
                                    .abs() as usize,
                            );
                            return Ok(true);
                        }
                    }
                }
                _ => {
                    if let Some(inventory) =
                        structure.inventory_mut(inventory_type == InventoryType::Input)
                    {
                        let (src, dst, item_name) = if to_player {
                            (
                                inventory,
                                &mut self.player.inventory,
                                self.selected_item.and_then(|item| item.map_struct(&pos)),
                            )
                        } else {
                            (
                                &mut self.player.inventory,
                                inventory,
                                self.selected_item.and_then(|item| {
                                    if let SelectedItem::PlayerInventory(i) = item {
                                        Some(i)
                                    } else {
                                        None
                                    }
                                }),
                            )
                        };
                        // console_log!("moving {:?}", item_name);
                        if let Some(item_name) = item_name {
                            if FactorishState::move_inventory_item(src, dst, &item_name) {
                                self.on_player_update.call1(
                                    &window(),
                                    &JsValue::from(self.get_player_inventory()?),
                                )?;
                                return Ok(true);
                            }
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
            ItemType::Splitter => Box::new(Splitter::new(cursor.x, cursor.y, self.tool_rotation)),
            ItemType::OreMine => Box::new(OreMine::new(cursor.x, cursor.y, self.tool_rotation)),
            ItemType::Chest => Box::new(Chest::new(cursor)),
            ItemType::Furnace => Box::new(Furnace::new(cursor)),
            ItemType::Assembler => Box::new(Assembler::new(cursor)),
            ItemType::Boiler => Box::new(Boiler::new(cursor)),
            ItemType::WaterWell => Box::new(WaterWell::new(cursor)),
            ItemType::OffshorePump => Box::new(OffshorePump::new(cursor)),
            ItemType::Pipe => Box::new(Pipe::new(cursor)),
            ItemType::SteamEngine => Box::new(SteamEngine::new(cursor)),
            ItemType::ElectPole => Box::new(ElectPole::new(cursor)),
            _ => return js_err!("Can't make a structure from {:?}", tool),
        })
    }

    /// Destructively converts serde_json::Value into a Box<dyn Structure>.
    fn structure_from_json(value: &mut serde_json::Value) -> Result<Box<dyn Structure>, JsValue> {
        let type_str = if let serde_json::Value::String(s) = value
            .get_mut("type")
            .ok_or_else(|| js_str!("\"type\" not found"))?
            .take()
        {
            s
        } else {
            return js_err!("Type must be a string");
        };

        let item_type = str_to_item(&type_str)
            .ok_or_else(|| js_str!("The structure type {} is not defined", type_str))?;

        let payload = value
            .get_mut("payload")
            .ok_or_else(|| js_str!("\"payload\" not found"))?
            .take();

        fn map_err<T: Structure>(result: serde_json::Result<T>) -> Result<T, JsValue> {
            result.map_err(|s| js_str!("structure deserialization error: {}", s))
        }

        Ok(match item_type {
            ItemType::TransportBelt => {
                Box::new(map_err(serde_json::from_value::<TransportBelt>(payload))?)
            }
            ItemType::Inserter => Box::new(map_err(serde_json::from_value::<Inserter>(payload))?),
            ItemType::Splitter => Box::new(map_err(serde_json::from_value::<Splitter>(payload))?),
            ItemType::OreMine => Box::new(map_err(serde_json::from_value::<OreMine>(payload))?),
            ItemType::Chest => Box::new(map_err(serde_json::from_value::<Chest>(payload))?),
            ItemType::Furnace => Box::new(map_err(serde_json::from_value::<Furnace>(payload))?),
            ItemType::Assembler => Box::new(map_err(serde_json::from_value::<Assembler>(payload))?),
            ItemType::Boiler => Box::new(map_err(serde_json::from_value::<Boiler>(payload))?),
            ItemType::WaterWell => Box::new(map_err(serde_json::from_value::<WaterWell>(payload))?),
            ItemType::OffshorePump => {
                Box::new(map_err(serde_json::from_value::<OffshorePump>(payload))?)
            }
            ItemType::Pipe => Box::new(map_err(serde_json::from_value::<Pipe>(payload))?),
            ItemType::SteamEngine => {
                Box::new(map_err(serde_json::from_value::<SteamEngine>(payload))?)
            }
            ItemType::ElectPole => Box::new(map_err(serde_json::from_value::<ElectPole>(payload))?),
            _ => return js_err!("Can't make a structure from {:?}", type_str),
        })
    }

    pub fn mouse_down(&mut self, pos: &[f64], button: i32) -> Result<JsValue, JsValue> {
        if pos.len() < 2 {
            return Err(JsValue::from_str("position must have 2 elements"));
        }
        let cursor = Position {
            x: (pos[0] / self.view_scale / 32. - self.viewport_x) as i32,
            y: (pos[1] / self.view_scale / 32. - self.viewport_y) as i32,
        };

        console_log!("mouse_down: {}, {}, button: {}", cursor.x, cursor.y, button);
        if button == 2
            && self.find_structure_tile(&[cursor.x, cursor.y]).is_none()
            // Let the player pick up drop items before harvesting ore below.
            && !self.drop_items.iter().any(|item| {
                item.x / TILE_SIZE_I == pos[0] as i32 / TILE_SIZE_I
                    && item.y / TILE_SIZE_I == pos[1] as i32 / TILE_SIZE_I
            })
        {
            if let Some(tile) = self.tile_at(&cursor) {
                if let Some(ore_type) = tile.get_ore_type() {
                    self.ore_harvesting = Some(OreHarvesting {
                        pos: cursor,
                        ore_type,
                        timer: 0,
                    });
                }
            }
        }
        self.update_info();
        Ok(JsValue::from(js_sys::Array::new()))
    }

    pub fn mouse_up(&mut self, pos: &[f64], button: i32) -> Result<JsValue, JsValue> {
        if pos.len() < 2 {
            return Err(JsValue::from_str("position must have 2 elements"));
        }
        let cursor = Position {
            x: (pos[0] / self.view_scale / 32. - self.viewport_x) as i32,
            y: (pos[1] / self.view_scale / 32. - self.viewport_y) as i32,
        };
        let mut events = vec![];

        if button == 0 {
            if let Some(selected_tool) = self.get_selected_tool_or_item_opt() {
                let cell = self.tile_at(&cursor);
                if let Some((count, cell)) =
                    self.player.inventory.get(&selected_tool).zip(cell.as_ref())
                {
                    if 1 <= *count && cell.water ^ (selected_tool != ItemType::OffshorePump) {
                        let mut new_s = self.new_structure(&selected_tool, &cursor)?;
                        let bbox = new_s.bounding_box();
                        for y in bbox.y0..bbox.y1 {
                            for x in bbox.x0..bbox.x1 {
                                self.harvest(&Position { x, y }, !new_s.movable())?;
                            }
                        }
                        // let connections = new_s.connection(self, &Ref(&self.structures));
                        // console_log!(
                        //     "Connection recalculated for self {:?}: {:?}",
                        //     new_s.position(),
                        //     connections
                        // );
                        // if let Some(fluid_boxes) = new_s.fluid_box_mut() {
                        //     for fbox in fluid_boxes {
                        //         fbox.connect_to = connections;
                        //     }
                        // }

                        // First, find an empty slot
                        let id = self
                            .structures
                            .iter()
                            .enumerate()
                            .find(|(_, s)| s.dynamic.is_none())
                            .map(|(i, slot)| StructureId {
                                id: i as u32,
                                gen: slot.gen,
                            })
                            .unwrap_or_else(|| StructureId {
                                id: self.structures.len() as u32,
                                gen: 0,
                            });

                        for (other_id, structure) in
                            self.structures.iter().enumerate().filter_map(|(i, s)| {
                                Some((
                                    StructureId {
                                        id: i as u32,
                                        gen: s.gen,
                                    },
                                    s.dynamic.as_deref()?,
                                ))
                            })
                        {
                            if (new_s.power_sink() && structure.power_source()
                                || new_s.power_source() && structure.power_sink())
                                && new_s.position().distance(structure.position())
                                    <= new_s.wire_reach().min(structure.wire_reach()) as i32
                            {
                                let new_power_wire = PowerWire(id, other_id);
                                if self.power_wires.iter().any(|p| *p == new_power_wire) {
                                    continue;
                                }
                                console_log!("power_wires: {}", self.power_wires.len());
                                self.power_wires.push(new_power_wire);
                            }
                        }

                        new_s.on_construction_self(
                            id,
                            &StructureDynIter::new_all(&mut self.structures),
                            true,
                        )?;

                        // Notify structures after a slot has been decided
                        for structure in &mut self.structures {
                            if let Some(s) = structure.dynamic.as_deref_mut() {
                                s.on_construction(id, new_s.as_mut(), true)?;
                            }
                        }

                        if id.id < self.structures.len() as u32 {
                            self.structures[id.id as usize].dynamic = Some(new_s);

                            console_log!(
                                "Inserted to an empty slot: {}/{}, id: {:?}",
                                self.structures
                                    .iter()
                                    .filter(|s| s.dynamic.is_none())
                                    .count(),
                                self.structures.len(),
                                id
                            );
                        } else {
                            self.structures.push(StructureEntry {
                                gen: 0,
                                dynamic: Some(new_s),
                            });
                            console_log!(
                                "Pushed to the end: {}/{}",
                                self.structures
                                    .iter()
                                    .filter(|s| s.dynamic.is_none())
                                    .count(),
                                self.structures.len()
                            );
                        }

                        self.power_networks = build_power_networks(
                            &StructureDynIter::new_all(&mut self.structures),
                            &self.power_wires,
                        );

                        self.update_fluid_connections(&cursor)?;

                        if let Ok(ref mut data) = self.minimap_buffer.try_borrow_mut() {
                            self.render_minimap_data_pixel(data, &cursor);
                        }
                        if let Some(count) = self.player.inventory.get_mut(&selected_tool) {
                            *count -= 1;
                        }
                        self.on_player_update
                            .call1(&window(), &JsValue::from(self.get_player_inventory()?))
                            .unwrap_or_else(|_| JsValue::from(true));
                        events.push(js_sys::Array::of1(&JsValue::from_str(
                            "updatePlayerInventory",
                        )));
                    }
                }
            } else if let Some(structure) = self.find_structure_tile(&[cursor.x, cursor.y]) {
                if structure.inventory(true).is_some()
                    || structure.inventory(false).is_some()
                    || structure.burner_inventory().is_some()
                {
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
        } else if button == 2 {
            if self.ore_harvesting.is_some() {
                self.ore_harvesting = None;
            } else {
                // Right click means explicit cleanup, so we pick up items no matter what.
                self.harvest(&cursor, true)?;
                events.push(js_sys::Array::of1(&JsValue::from_str(
                    "updatePlayerInventory",
                )));
            }
        }

        console_log!("mouse_up: {}, {}", cursor.x, cursor.y);
        self.update_info();
        Ok(JsValue::from(events.iter().collect::<js_sys::Array>()))
    }

    pub fn mouse_move(&mut self, pos: &[f64]) -> Result<(), JsValue> {
        if pos.len() < 2 {
            return Err(JsValue::from_str("position must have 2 elements"));
        }
        let cursor = [
            (pos[0] / self.view_scale / 32. - self.viewport_x) as i32,
            (pos[1] / self.view_scale / 32. - self.viewport_y) as i32,
        ];
        if cursor[0] < 0
            || self.width as i32 <= cursor[0]
            || cursor[1] < 0
            || self.height as i32 <= cursor[1]
        {
            // return Err(js_str!("invalid mouse position: {:?}", cursor));
            // This is not particularly an error. Just ignore it.
            return Ok(());
        }
        self.cursor = Some(cursor);
        // console_log!("mouse_move: cursor: {}, {}", cursor[0], cursor[1]);
        self.update_info();
        Ok(())
    }

    pub fn mouse_leave(&mut self) -> Result<(), JsValue> {
        self.cursor = None;
        if let Some(ref elem) = self.info_elem {
            elem.set_inner_html("");
        }
        if self.ore_harvesting.is_some() {
            self.ore_harvesting = None;
        }
        console_log!("mouse_leave");
        Ok(())
    }

    pub fn mouse_wheel(&mut self, delta: i32, x: f64, y: f64) -> Result<(), JsValue> {
        let base = (2_f64).powf(1. / 5.);
        let new_scale = if delta < 0 {
            (self.view_scale * base).min(8.)
        } else {
            (self.view_scale / base).max(0.5)
        };
        self.viewport_x += (x as f64 / self.view_scale / 32.) * (1. - new_scale / self.view_scale);
        self.viewport_y += (y as f64 / self.view_scale / 32.) * (1. - new_scale / self.view_scale);
        self.view_scale = new_scale;
        Ok(())
    }

    pub fn on_key_down(&mut self, key_code: i32) -> Result<bool, JsValue> {
        match key_code {
            82 => self
                .rotate()
                .map_err(|err| JsValue::from(format!("Rotate failed: {:?}", err))),
            // Detect keys through '0'..'9', that's a shame char literal cannot be used in place of i32
            code @ 48..=58 => {
                self.select_tool((code - '0' as i32 + 9) % 10);
                Ok(true)
            }
            37 => {
                // Left
                self.viewport_x = (self.viewport_x + 1.).min(0.);
                Ok(true)
            }
            38 => {
                // Up
                self.viewport_y = (self.viewport_y + 1.).min(0.);
                Ok(true)
            }
            39 => {
                // Right
                self.viewport_x = (self.viewport_x - 1.).max(-(self.width as f64));
                Ok(true)
            }
            40 => {
                // Down
                self.viewport_y = (self.viewport_y - 1.).max(-(self.height as f64));
                Ok(true)
            }
            81 => {
                // 'q'
                if self.selected_item.is_some() {
                    self.selected_item = None;
                } else if let Some(cursor) = self.cursor {
                    if let Some(structure) = self
                        .find_structure_tile(&cursor)
                        .and_then(|s| str_to_item(s.name()))
                    {
                        self.selected_item = if self.player.inventory.count_item(&structure) > 0 {
                            Some(SelectedItem::PlayerInventory(structure))
                        } else {
                            None
                        };
                        console_log!("q: selected_tool is {:?}", self.selected_item);
                    }
                }
                Ok(true)
            }
            _ => {
                console_log!("unrecognized key: {}", key_code);
                Ok(false)
            }
        }
    }

    fn color_of_cell(cell: &Cell) -> [u8; 3] {
        if cell.water {
            [0x00, 0x00, 0xff]
        } else if 0 < cell.iron_ore {
            [0x3f, 0xaf, 0xff]
        } else if 0 < cell.coal_ore {
            [0x1f, 0x1f, 0x1f]
        } else if 0 < cell.copper_ore {
            [0x7f, 0x3f, 0x00]
        } else {
            [0x7f, 0x7f, 0x7f]
        }
    }

    fn render_minimap_data(&mut self) -> Result<(), JsValue> {
        let mut data = self
            .minimap_buffer
            .try_borrow_mut()
            .map_err(|_| js_str!("Couldn't acquire mutable ref for minimap buffer"))?;
        *data = vec![0u8; (self.width * self.height * 4) as usize];

        for y in 0..self.height as i32 {
            for x in 0..self.width as i32 {
                let cell = self.tile_at(&Position { x, y }).unwrap();
                let start = ((x + y * self.width as i32) * 4) as usize;
                data[start + 3] = 255;
                let color = Self::color_of_cell(&cell);
                data[start..start + 3].copy_from_slice(&color);
            }
        }

        // context.set_fill_style(&JsValue::from_str("#00ff7f"));
        let color = [0x00, 0xff, 0x7f];
        for structure in self.structure_iter() {
            let Position { x, y } = *structure.position();
            if x < self.width as i32 && y < self.height as i32 {
                let start = ((x + y * self.width as i32) * 4) as usize;
                data[start..start + 3].copy_from_slice(&color);
            }
        }

        Ok(())
    }

    fn render_minimap_data_pixel(&self, data: &mut Vec<u8>, position: &Position) {
        let Position { x, y } = *position;
        let color;
        if self.structures.iter().any(|structure| {
            structure
                .dynamic
                .as_deref()
                .map(|s| *s.position() == *position)
                .unwrap_or(false)
        }) {
            color = [0x00, 0xff, 0x7f];
        } else {
            let cell = self.tile_at(position).unwrap();
            color = Self::color_of_cell(&cell);
        }
        let start = ((x + y * self.width as i32) * 4) as usize;
        data[start..start + 3].copy_from_slice(&color);
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

        self.render_minimap_data()?;

        let load_image = |path| -> Result<ImageBundle, JsValue> {
            if let Some(value) = image_assets.iter().find(|value| {
                let array = js_sys::Array::from(value);
                array.iter().next() == Some(JsValue::from_str(path))
            }) {
                let array = js_sys::Array::from(&value).to_vec();
                Ok(ImageBundle {
                    url: array
                        .get(1)
                        .cloned()
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
                        .cloned()
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
        self.image_back_tiles = Some(load_image("backTiles")?);
        self.image_weeds = Some(load_image("weeds")?);
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
        self.image_offshore_pump = Some(load_image("offshorePump")?);
        self.image_pipe = Some(load_image("pipe")?);
        self.image_elect_pole = Some(load_image("electPole")?);
        self.image_splitter = Some(load_image("splitter")?);
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
        self.image_fuel_alarm = Some(load_image("fuelAlarm")?);
        self.image_electricity_alarm = Some(load_image("electricityAlarm")?);
        Ok(())
    }

    pub fn tool_defs(&self) -> Result<js_sys::Array, JsValue> {
        Ok(tool_defs
            .iter()
            .map(|tool| {
                js_sys::Array::of2(
                    &JsValue::from_str(&item_to_str(&tool.item_type)),
                    &JsValue::from_str(&tool.desc),
                )
            })
            .collect::<js_sys::Array>())
    }

    /// Returns 2-array with [selected_tool, inventory_count]
    pub fn selected_tool(&self) -> js_sys::Array {
        if let Some(SelectedItem::ToolBelt(selected_tool)) = self.selected_item {
            [
                JsValue::from(selected_tool as f64),
                JsValue::from(
                    *self.tool_belt[selected_tool]
                        .and_then(|item| self.player.inventory.get(&item))
                        .unwrap_or(&0) as f64,
                ),
            ]
            .iter()
            .collect()
        } else {
            js_sys::Array::new()
        }
    }

    /// Returns count of selected item or null
    pub fn get_selected_item(&self) -> JsValue {
        if let Some(SelectedItem::PlayerInventory(selected_item)) = self.selected_item {
            JsValue::from_f64(*self.player.inventory.get(&selected_item).unwrap_or(&0) as f64)
        } else {
            JsValue::null()
        }
    }

    pub fn get_selected_tool_or_item(&self) -> JsValue {
        if let Some(selected_item) = self.get_selected_tool_or_item_opt() {
            JsValue::from_str(&item_to_str(&selected_item))
        } else {
            JsValue::null()
        }
    }

    /// Renders a tool item on the toolbar icon.
    pub fn render_tool(
        &self,
        tool_index: usize,
        context: &CanvasRenderingContext2d,
    ) -> Result<(), JsValue> {
        context.clear_rect(0., 0., 32., 32.);
        if let Some(item) = self.tool_belt.get(tool_index).unwrap_or(&None) {
            let mut tool = self.new_structure(item, &Position { x: 0, y: 0 })?;
            tool.set_rotation(&self.tool_rotation).ok();
            for depth in 0..3 {
                tool.draw(self, context, depth, true)?;
            }
        }
        Ok(())
    }

    /// Returns [item_name, desc] if there is an item on the tool belt slot at `index`,
    /// otherwise null.
    pub fn get_tool_desc(&self, index: usize) -> Result<JsValue, JsValue> {
        Ok(self
            .tool_belt
            .get(index)
            .unwrap_or(&None)
            .and_then(|item| tool_defs.iter().find(|tool| tool.item_type == item))
            .map(|def| {
                JsValue::from(
                    [
                        JsValue::from(&item_to_str(&def.item_type)),
                        JsValue::from(def.desc),
                    ]
                    .iter()
                    .collect::<js_sys::Array>(),
                )
            })
            .unwrap_or_else(JsValue::null))
    }

    /// @returns (number|null) selected tool internally in the FactorishState (number) or null if unselected.
    pub fn get_selected_tool(&self) -> JsValue {
        if let Some(SelectedItem::ToolBelt(value)) = self.selected_item {
            JsValue::from(value as i32)
        } else {
            JsValue::null()
        }
    }

    fn get_selected_tool_or_item_opt(&self) -> Option<ItemType> {
        match self.selected_item {
            Some(SelectedItem::ToolBelt(tool)) => self.tool_belt[tool],
            Some(SelectedItem::PlayerInventory(item)) => tool_defs
                .iter()
                .find(|def| def.item_type == item)
                .and(Some(item)),
            Some(SelectedItem::StructInventory(pos, item)) => self
                .structure_iter()
                .find(|s| *s.position() == pos)
                .and_then(|s| s.inventory(false))
                .and_then(|inventory| inventory.get(&item))
                .and(Some(item)),
            None => None,
        }
    }

    /// Attempts to select or set a tool if the player is holding an item
    ///
    /// @param tool the index of the tool item, [0,9]
    /// @returns whether the tool bar item should be re-rendered
    pub fn select_tool(&mut self, tool: i32) -> bool {
        if let Some(SelectedItem::PlayerInventory(item)) = self.selected_item {
            // We allow only items in tool_defs to present on the tool belt
            // This behavior is different from Factorio, maybe we can allow it
            if tool_defs.iter().any(|i| i.item_type == item) {
                self.tool_belt[tool as usize] = Some(item);
                // Deselect the item for the player to let him select from tool belt.
                self.selected_item = None;
                return true;
            } else {
                console_log!(
                    "select_tool could not find tool_def with item type: {:?}",
                    item
                );
                return false;
            }
        }
        self.selected_item =
            if 0 <= tool && Some(SelectedItem::ToolBelt(tool as usize)) != self.selected_item {
                Some(SelectedItem::ToolBelt(tool as usize))
            } else {
                None
            };
        self.selected_item.is_some()
    }

    pub fn rotate_tool(&mut self) -> i32 {
        self.tool_rotation = self.tool_rotation.next();
        self.tool_rotation.angle_4()
    }

    /// Returns an array of item count for tool bar items
    pub fn tool_inventory(&self) -> js_sys::Array {
        self.tool_belt
            .iter()
            .map(|item| {
                JsValue::from(
                    *item
                        .and_then(|item| self.player.inventory.get(&item))
                        .unwrap_or(&0) as f64,
                )
            })
            .collect()
    }

    fn get_viewport(&self) -> (f64, f64) {
        (
            self.viewport_width / self.view_scale,
            self.viewport_height / self.view_scale,
        )
    }

    pub fn set_viewport_pos(&mut self, x: f64, y: f64) -> Result<js_sys::Array, JsValue> {
        let viewport = self.get_viewport();
        self.viewport_x = -(x - viewport.0 / 32. / 2.)
            .max(0.)
            .min(self.width as f64 - viewport.0 / 32. - 1.);
        self.viewport_y = -(y - viewport.1 / 32. / 2.)
            .max(0.)
            .min(self.height as f64 - viewport.1 / 32. - 1.);
        Ok(js_sys::Array::of2(
            &JsValue::from_f64(viewport.0),
            &JsValue::from_f64(viewport.1),
        ))
    }

    pub fn delta_viewport_pos(&mut self, x: f64, y: f64) -> Result<(), JsValue> {
        self.viewport_x += x / self.view_scale / 32.;
        self.viewport_y += y / self.view_scale / 32.;
        Ok(())
    }

    /// Add a new popup text that will show for a moment and automatically disappears
    ///
    /// @param text Is given as owned string because the text is most likely dynamic.
    fn new_popup_text(&mut self, text: String, x: f64, y: f64) {
        let pop = PopupText {
            text: text.to_string(),
            x: (x + self.viewport_x * TILE_SIZE) * self.view_scale,
            y: (y + self.viewport_y * TILE_SIZE) * self.view_scale,
            life: POPUP_TEXT_LIFE,
        };
        self.popup_texts.push(pop);
    }

    /// Returns an iterator over valid structures
    fn structure_iter(&self) -> impl Iterator<Item = &dyn Structure> {
        self.structures.iter().filter_map(|s| s.dynamic.as_deref())
    }

    pub fn render(&self, context: CanvasRenderingContext2d) -> Result<(), JsValue> {
        use std::f64;

        context.clear_rect(0., 0., self.viewport_width, self.viewport_height);

        context.save();
        context.scale(self.view_scale, self.view_scale)?;
        context.translate(self.viewport_x * 32., self.viewport_y * 32.)?;

        match self
            .image_dirt
            .as_ref()
            .zip(self.image_back_tiles.as_ref())
            .zip(self.image_ore.as_ref())
            .zip(self.image_coal.as_ref())
            .zip(self.image_copper.as_ref())
        {
            Some(((((img, back_tiles), img_ore), img_coal), img_copper)) => {
                // let mut cell_draws = 0;
                let left = (-self.viewport_x).max(0.) as u32;
                let top = (-self.viewport_y).max(0.) as u32;
                let right = (((self.viewport_width / 32. / self.view_scale - self.viewport_x) + 1.)
                    as u32)
                    .min(self.width);
                let bottom = (((self.viewport_height / 32. / self.view_scale - self.viewport_y)
                    + 1.) as u32)
                    .min(self.height);
                for y in top..bottom {
                    for x in left..right {
                        let cell = &self.board[(x + y * self.width) as usize];
                        let (dx, dy) = (x as f64 * 32., y as f64 * 32.);
                        if cell.water || cell.image != 0 {
                            let srcx = cell.image % 4;
                            let srcy = cell.image / 4;
                            context.draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                                &back_tiles.bitmap, (srcx * 32) as f64, (srcy * 32) as f64, 32., 32., dx, dy, 32., 32.)?;
                        } else {
                            context.draw_image_with_image_bitmap(&img.bitmap, dx, dy)?;
                            if let Some(weeds) = &self.image_weeds {
                                if 0 < cell.grass_image {
                                    context.draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                                        &weeds.bitmap,
                                        (cell.grass_image * 32) as f64, 0., 32., 32., dx, dy, 32., 32.)?;
                                }
                            } else {
                                console_log!("Weed image not found");
                            }
                        }
                        let draw_ore = |ore: u32, img: &ImageBitmap| -> Result<(), JsValue> {
                            if 0 < ore {
                                let idx = (ore / 10).min(3);
                                // console_log!("x: {}, y: {}, idx: {}, ore: {}", x, y, idx, ore);
                                context.draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                                    img, (idx * 32) as f64, 0., 32., 32., x as f64 * 32., y as f64 * 32., 32., 32.)?;
                            }
                            Ok(())
                        };
                        draw_ore(cell.iron_ore, &img_ore.bitmap)?;
                        draw_ore(cell.coal_ore, &img_coal.bitmap)?;
                        draw_ore(cell.copper_ore, &img_copper.bitmap)?;
                        // cell_draws += 1;
                    }
                }
                // console_log!(
                //     "size: {:?}, scale: {:?}, cell_draws: {} []: {:?}",
                //     self.get_viewport(),
                //     self.view_scale,
                //     cell_draws,
                //     [left, top, right, bottom] // self.board.iter().fold(0, |accum, val| accum + val.iron_ore)
                // );
            }
            _ => {
                return Err(JsValue::from_str("image not available"));
            }
        }

        let draw_structures = |depth| -> Result<(), JsValue> {
            for structure in self.structure_iter() {
                structure.draw(&self, &context, depth, false)?;
            }
            Ok(())
        };

        draw_structures(0)?;

        for item in &self.drop_items {
            render_drop_item(self, &context, &item.type_, item.x, item.y)?;
        }

        const WIRE_ATTACH_X: f64 = 28.;
        const WIRE_ATTACH_Y: f64 = 8.;

        let draw_wires = |wires: &[PowerWire]| {
            for PowerWire(first, second) in wires {
                context.begin_path();
                let first = if let Some(d) = self
                    .get_structure(*first)
                {
                    d.position()
                } else {
                    continue;
                };
                context.move_to(
                    first.x as f64 * TILE_SIZE + WIRE_ATTACH_X,
                    first.y as f64 * TILE_SIZE + WIRE_ATTACH_Y,
                );
                let second = if let Some(d) = self
                    .get_structure(*second)
                {
                    d.position()
                } else {
                    continue;
                };
                context.quadratic_curve_to(
                    (first.x + second.x) as f64 / 2. * TILE_SIZE + WIRE_ATTACH_X,
                    (first.y + second.y) as f64 / 1.9 * TILE_SIZE + WIRE_ATTACH_Y,
                    second.x as f64 * TILE_SIZE + WIRE_ATTACH_X,
                    second.y as f64 * TILE_SIZE + WIRE_ATTACH_Y,
                );
                context.stroke();
            }
        };

        if self.debug_power_network {
            for (i, nw) in self.power_networks.iter().enumerate() {
                context.set_stroke_style(&js_str!(
                    ["rgb(255,0,0)", "rgb(0,0,255)", "rgb(0,255,0)"][i % 3]
                ));
                context.set_line_width(3.);
                draw_wires(&nw.wires);
            }
        }

        context.set_stroke_style(&js_str!("rgb(191,127,0)"));
        context.set_line_width(1.);
        draw_wires(&self.power_wires);

        draw_structures(1)?;
        draw_structures(2)?;

        if self.debug_bbox {
            context.save();
            context.set_stroke_style(&js_str!("red"));
            context.set_line_width(1.);
            for structure in self.structure_iter() {
                let bb = structure.bounding_box();
                context.stroke_rect(
                    bb.x0 as f64 * TILE_SIZE,
                    bb.y0 as f64 * TILE_SIZE,
                    (bb.x1 - bb.x0) as f64 * TILE_SIZE,
                    (bb.y1 - bb.y0) as f64 * TILE_SIZE,
                );
            }
            context.set_stroke_style(&js_str!("purple"));
            for item in &self.drop_items {
                context.stroke_rect(
                    item.x as f64 - DROP_ITEM_SIZE / 2.,
                    item.y as f64 - DROP_ITEM_SIZE / 2.,
                    DROP_ITEM_SIZE,
                    DROP_ITEM_SIZE,
                );
            }
            context.restore();
        }

        if self.debug_fluidbox {
            context.save();
            for structure in self.structure_iter() {
                if let Some(fluid_boxes) = structure.fluid_box() {
                    let bb = structure.bounding_box();
                    for (i, fb) in fluid_boxes.iter().enumerate() {
                        const BAR_MARGIN: f64 = 4.;
                        const BAR_WIDTH: f64 = 4.;
                        context.set_stroke_style(&js_str!("red"));
                        context.set_fill_style(&js_str!("black"));
                        context.fill_rect(
                            bb.x0 as f64 * TILE_SIZE + BAR_MARGIN + 6. * i as f64,
                            bb.y0 as f64 * TILE_SIZE + BAR_MARGIN,
                            BAR_WIDTH,
                            (bb.y1 - bb.y0) as f64 * TILE_SIZE - BAR_MARGIN * 2.,
                        );
                        context.stroke_rect(
                            bb.x0 as f64 * TILE_SIZE + BAR_MARGIN + 6. * i as f64,
                            bb.y0 as f64 * TILE_SIZE + BAR_MARGIN,
                            BAR_WIDTH,
                            (bb.y1 - bb.y0) as f64 * TILE_SIZE - BAR_MARGIN * 2.,
                        );
                        context.set_fill_style(&js_str!(match fb.type_ {
                            Some(FluidType::Water) => "#00ffff",
                            Some(FluidType::Steam) => "#afafaf",
                            _ => "#7f7f7f",
                        }));
                        let bar_height = fb.amount / fb.max_amount
                            * ((bb.y1 - bb.y0) as f64 * TILE_SIZE - BAR_MARGIN * 2.);
                        context.fill_rect(
                            bb.x0 as f64 * TILE_SIZE + BAR_MARGIN + 6. * i as f64,
                            bb.y1 as f64 * TILE_SIZE - BAR_MARGIN - bar_height,
                            4.,
                            bar_height,
                        );
                    }
                }
            }
            context.restore();
        }

        for ent in &self.temp_ents {
            if let Some(img) = &self.image_smoke {
                let (x, y) = (ent.position.0 - 24., ent.position.1 - 24.);
                context.save();
                context
                    .set_global_alpha(((ent.max_life - ent.life).min(ent.life) * 0.15).min(0.35));
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
            if let Some(selected_tool) = self.get_selected_tool_or_item_opt() {
                context.save();
                context.set_global_alpha(0.5);
                let mut tool = self.new_structure(&selected_tool, &Position::from(cursor))?;
                tool.set_rotation(&self.tool_rotation).ok();
                for depth in 0..3 {
                    tool.draw(self, &context, depth, false)?;
                }
                context.restore();
            }
            context.set_stroke_style(&JsValue::from_str("blue"));
            context.set_line_width(2.);
            context.stroke_rect(x, y, 32., 32.);
        }

        if let Some(ore_harvesting) = &self.ore_harvesting {
            context.set_stroke_style(&js_str!("rgb(255,127,255)"));
            context.set_line_width(4.);
            context.begin_path();
            context.arc(
                (ore_harvesting.pos.x as f64 + 0.5) * TILE_SIZE,
                (ore_harvesting.pos.y as f64 + 0.5) * TILE_SIZE,
                TILE_SIZE / 2. + 2.,
                0.,
                ore_harvesting.timer as f64 / ORE_HARVEST_TIME as f64 * 2. * f64::consts::PI,
            )?;
            context.stroke();
        }

        context.restore();

        context.set_font("bold 14px sans-serif");
        context.set_stroke_style(&js_str!("white"));
        context.set_line_width(2.);
        context.set_fill_style(&js_str!("rgb(0,0,0)"));
        for item in &self.popup_texts {
            context.stroke_text(&item.text, item.x, item.y)?;
            context.fill_text(&item.text, item.x, item.y)?;
        }

        Ok(())
    }

    pub fn render_minimap(&self, context: CanvasRenderingContext2d) -> Result<(), JsValue> {
        let width = self.width as f64;
        let height = self.height as f64;
        context.save();

        context.set_fill_style(&JsValue::from_str("#7f7f7f"));
        context.fill_rect(0., 0., width, height);

        if let Ok(ref mut data) = self.minimap_buffer.try_borrow_mut() {
            let image_data = ImageData::new_with_u8_clamped_array_and_sh(
                Clamped::<_>(&mut *data),
                self.width as u32,
                self.height as u32,
            )?;

            context.put_image_data(&image_data, 0., 0.)?;
        }

        context.set_stroke_style(&JsValue::from_str("blue"));
        context.set_line_width(1.);
        let viewport = self.get_viewport();
        context.stroke_rect(
            -self.viewport_x,
            -self.viewport_y,
            viewport.0 / 32.,
            viewport.1 / 32.,
        );
        context.restore();
        Ok(())
    }
}
