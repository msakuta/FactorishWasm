#![allow(non_upper_case_globals)]

/// Macros needs to come first in order to be accessible from all the codes
#[macro_use]
mod macros;

macro_rules! try_continue {
    ($e:expr) => {
        if let Some(result) = $e {
            result
        } else {
            continue;
        }
    };
}

mod assembler;
mod boiler;
mod burner;
mod chest;
mod drop_items;
mod dyn_iter;
mod elect_pole;
mod electric_furnace;
mod factory;
mod furnace;
mod inserter;
mod inventory;
mod items;
mod minimap;
mod offshore_pump;
mod ore_mine;
mod perf;
mod perlin_noise;
mod pipe;
mod power_network;
mod scenarios;
mod splitter;
mod steam_engine;
mod structure;
mod terrain;
mod transport_belt;
mod underground_belt;
mod underground_pipe;
mod utils;
mod water_well;
mod gl {
    pub(crate) mod assets;
    mod render_gl;
    mod shader_bundle;
    pub(crate) mod utils;

    pub(crate) use render_gl::{
        draw_direction_arrow_gl, draw_electricity_alarm_gl, draw_fuel_alarm_gl,
    };
    pub(crate) use shader_bundle::ShaderBundle;
}

use crate::{
    drop_items::{
        add_index, build_index, drop_item_id_iter, drop_item_iter, hit_check, hit_check_with_index,
        remove_index, update_index, DropItem, DropItemEntry, DropItemId, DropItemIndex,
        DROP_ITEM_SIZE, INDEX_CHUNK_SIZE,
    },
    gl::assets::Assets,
    perf::PerfStats,
    scenarios::select_scenario,
    terrain::{
        calculate_back_image, calculate_back_image_all, gen_chunk, Chunk, Chunks,
        TerrainParameters, CHUNK_SIZE, CHUNK_SIZE2, CHUNK_SIZE_I,
    },
};
use assembler::Assembler;
use boiler::Boiler;
use chest::Chest;
use elect_pole::ElectPole;
use electric_furnace::ElectricFurnace;
use furnace::Furnace;
use inserter::Inserter;
use inventory::{Inventory, InventoryTrait, InventoryType, STACK_SIZE};
use items::{item_to_str, str_to_item, ItemType};
use offshore_pump::OffshorePump;
use ore_mine::OreMine;
use perlin_noise::Xor128;
use pipe::Pipe;
use power_network::{build_power_networks, PowerNetwork};
use splitter::Splitter;
use steam_engine::SteamEngine;
use structure::{
    FrameProcResult, ItemResponse, Position, RotateErr, Rotation, Structure, StructureBundle,
    StructureComponents, StructureDynIter, StructureEntry, StructureId,
};
use transport_belt::TransportBelt;
use underground_belt::{UnderDirection, UndergroundBelt};
use underground_pipe::UndergroundPipe;
use water_well::{FluidType, WaterWell};

use serde::{Deserialize, Serialize};
use std::hash::Hash;
use std::{collections::HashMap, convert::TryFrom};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{
    CanvasRenderingContext2d, HtmlCanvasElement, HtmlDivElement, ImageBitmap, WebGlRenderingContext,
};

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

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

fn performance() -> web_sys::Performance {
    window()
        .performance()
        .expect("performance should be available")
}

const TILE_SIZE: f64 = 32.;
const TILE_SIZE_F: f32 = TILE_SIZE as f32;
const TILE_SIZE_I: i32 = TILE_SIZE as i32;

const COAL_POWER: f64 = 100.; // kilojoules
const SAVE_VERSION: i64 = 5;
const ORE_HARVEST_TIME: i32 = 20;

const WIRE_ATTACH_X: f64 = 28.;
const WIRE_ATTACH_Y: f64 = 8.;
const WIRE_HANG: f64 = 0.15;

/// Event types that can be communicated to the JavaScript code.
/// It is serialized into a JavaScript Object through serde.
#[derive(Serialize)]
enum JSEvent {
    UpdatePlayerInventory,
    ShowInventory,
    ShowInventoryAt {
        pos: (i32, i32),
        recipe_enable: bool,
    },
    UpdateStructureInventory(i32, i32),
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Debug)]
enum Ore {
    Iron,
    Coal,
    Copper,
    Stone,
}

#[derive(Copy, Clone, Serialize, Deserialize)]
struct OreValue(Ore, u32);

#[derive(Copy, Clone, Serialize, Deserialize)]
struct Cell {
    water: bool,
    ore: Option<OreValue>,
    #[serde(skip)]
    image: u8,
    #[serde(skip)]
    grass_image: u8,
}

impl Default for Cell {
    fn default() -> Self {
        Cell {
            water: false,
            ore: None,
            image: 0,
            grass_image: 0,
        }
    }
}

impl Cell {
    fn get_ore_type(&self) -> Option<ItemType> {
        match self.ore {
            Some(OreValue(Ore::Iron, _)) => Some(ItemType::IronOre),
            Some(OreValue(Ore::Copper, _)) => Some(ItemType::CopperOre),
            Some(OreValue(Ore::Coal, _)) => Some(ItemType::CoalOre),
            Some(OreValue(Ore::Stone, _)) => Some(ItemType::StoneOre),
            _ => None,
        }
    }
}

const tilesize: i32 = 32;
struct ToolDef {
    item_type: ItemType,
    desc: &'static str,
}
const tool_defs: [ToolDef; 16] = [
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
        item_type: ItemType::ElectricFurnace,
        desc: "Smelts metal ores into metal bars.<br>Uses electricity to operate.",
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
        item_type: ItemType::UndergroundPipe,
        desc: "Transport fluid beyond obstacles.",
    },
    ToolDef {
        item_type: ItemType::SteamEngine,
        desc: "Consumes steam and transmits electricity within a range of 3 tiles.",
    },
    ToolDef {
        item_type: ItemType::ElectPole,
        desc: "Electric pole.",
    },
    ToolDef {
        item_type: ItemType::UndergroundBelt,
        desc: "Underground belt can connect transport belts without blocking other structures in between.",
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
pub(crate) struct Recipe {
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
    velocity: (f64, f64),
    rotation: f64,
    life: f64,
    max_life: f64,
}

impl TempEnt {
    fn new(rng: &mut Xor128, position: Position) -> Self {
        let life = rng.next() * 3. + 6.;
        TempEnt {
            position: (
                (position.x as f64 + 0.5) * TILE_SIZE,
                (position.y as f64 + 0.25 + rng.next() * 0.5) * TILE_SIZE,
            ),
            velocity: (
                (rng.next() * 1.5 - 0.75 + 0.5), // A bit bias to the right
                (4. + rng.next()),
            ),
            rotation: rng.next() * std::f64::consts::PI * 2.,
            life,
            max_life: life,
        }
    }
}

#[derive(Eq, PartialEq, Hash, Copy, Clone, Serialize, Deserialize, Debug)]
struct PowerWire(StructureId, StructureId);

#[derive(Eq, PartialEq, Copy, Clone, Debug, Serialize)]
enum SelectedItem {
    /// This is index into `tool_belt`. It is kind of duplicate of `player.selected_item`,
    /// but we make it separate field because multiple tool belt slots refer to the same item type.
    ToolBelt(usize),
    PlayerInventory(ItemType, usize),
    StructInventory(StructureId, InventoryType, ItemType, usize),
}

impl SelectedItem {
    fn map_struct(&self, id: &StructureId) -> Option<ItemType> {
        if let SelectedItem::StructInventory(self_id, _, item, _) = self {
            if self_id == id {
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

#[derive(Serialize, Deserialize)]
struct Viewport {
    x: f64,
    y: f64,
    scale: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            x: 0.,
            y: 0.,
            scale: 1.,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Bounds {
    width: i32,
    height: i32,
}

fn apply_bounds(
    bounds: &Option<Bounds>,
    viewport: &Viewport,
    viewport_width: f64,
    viewport_height: f64,
) -> (i32, i32, i32, i32) {
    if let Some(bounds) = bounds.as_ref() {
        let left = ((-viewport.x).floor() as i32).max(0);
        let top = ((-viewport.y).floor() as i32).max(0);
        // Compensate the inclusive boundary with subtracting bounds.width and height by 1
        let right = (((viewport_width / TILE_SIZE / viewport.scale - viewport.x) + 1.) as i32)
            .min(bounds.width - 1);
        let bottom = (((viewport_height / TILE_SIZE / viewport.scale - viewport.y) + 1.) as i32)
            .min(bounds.height - 1);
        (left, top, right, bottom)
    } else {
        let left = (-viewport.x).floor() as i32;
        let top = (-viewport.y).floor() as i32;
        let right = ((viewport_width / TILE_SIZE / viewport.scale - viewport.x) + 1.) as i32;
        let bottom = ((viewport_height / TILE_SIZE / viewport.scale - viewport.y) + 1.) as i32;
        (left, top, right, bottom)
    }
}

#[wasm_bindgen]
pub struct FactorishState {
    #[allow(dead_code)]
    delta_time: f64,
    sim_time: f64,
    width: u32,
    height: u32,
    bounds: Option<Bounds>,
    viewport_width: f64,
    viewport_height: f64,
    viewport: Viewport,
    board: Chunks,
    terrain_params: TerrainParameters,
    structures: Vec<StructureEntry>,
    selected_structure_inventory: Option<StructureId>,
    drop_items: Vec<DropItemEntry>,
    drop_items_index: DropItemIndex,
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
    on_popup_text: js_sys::Function,
    on_structure_destroy: js_sys::Function,
    minimap_buffer: Vec<u8>,
    power_wires: Vec<PowerWire>,
    alt_mode: bool,
    debug_bbox: bool,
    debug_fluidbox: bool,
    debug_power_network: bool,
    use_webgl_instancing: bool,

    // Performance measurements
    perf_structures: PerfStats,
    perf_drop_items: PerfStats,
    perf_simulate: PerfStats,
    perf_minimap: PerfStats,
    perf_render: PerfStats,

    // on_show_inventory: js_sys::Function,
    image_belt: Option<ImageBundle>,
    image_underground_belt: Option<ImageBundle>,
    image_chest: Option<ImageBundle>,
    image_mine: Option<ImageBundle>,
    image_furnace: Option<ImageBundle>,
    image_electric_furnace: Option<ImageBundle>,
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
    image_stone_ore: Option<ImageBundle>,
    image_iron_plate: Option<ImageBundle>,
    image_copper_plate: Option<ImageBundle>,
    image_gear: Option<ImageBundle>,
    image_copper_wire: Option<ImageBundle>,
    image_circuit: Option<ImageBundle>,
    image_steel_plate: Option<ImageBundle>,
    image_time: Option<ImageBundle>,
    image_underground_belt_item: Option<ImageBundle>,

    assets: Assets,
}

#[derive(Debug)]
enum NewObjectErr {
    BlockedByStructure,
    BlockedByItem,
    OutOfMap,
    OnWater,
}

#[wasm_bindgen]
impl FactorishState {
    #[wasm_bindgen(constructor)]
    pub fn new(
        terrain_params: JsValue,
        on_player_update: js_sys::Function,
        on_popup_text: js_sys::Function,
        on_structure_destroy: js_sys::Function,
        // on_show_inventory: js_sys::Function,
        scenario: &str,
        context: WebGlRenderingContext,
        assets: js_sys::Array,
    ) -> Result<FactorishState, JsValue> {
        console_log!("FactorishState constructor");

        let terrain_params: TerrainParameters = serde_wasm_bindgen::from_value(terrain_params)?;

        let mut tool_belt = [None; 10];
        tool_belt[0] = Some(ItemType::OreMine);
        tool_belt[1] = Some(ItemType::Inserter);
        tool_belt[2] = Some(ItemType::TransportBelt);
        tool_belt[3] = Some(ItemType::Furnace);

        let (structures, board, drop_items) = select_scenario(scenario, &terrain_params)?;

        let mut ret = FactorishState {
            delta_time: 0.1,
            sim_time: 0.0,
            width: terrain_params.width,
            height: terrain_params.height,
            bounds: if terrain_params.unlimited {
                None
            } else {
                Some(Bounds {
                    width: terrain_params.width as i32,
                    height: terrain_params.height as i32,
                })
            },
            viewport_height: 0.,
            viewport_width: 0.,
            viewport: Viewport {
                x: 0.,
                y: 0.,
                scale: 1.,
            },
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
                    (ItemType::UndergroundBelt, 5usize),
                ]
                .iter()
                .copied()
                .collect(),
            },
            info_elem: None,
            minimap_buffer: vec![],
            power_wires: vec![],
            power_networks: vec![],
            alt_mode: false,
            debug_bbox: false,
            debug_fluidbox: false,
            debug_power_network: false,
            use_webgl_instancing: true,
            perf_structures: PerfStats::default(),
            perf_drop_items: PerfStats::default(),
            perf_simulate: PerfStats::default(),
            perf_minimap: PerfStats::default(),
            perf_render: PerfStats::default(),
            image_belt: None,
            image_underground_belt: None,
            image_chest: None,
            image_mine: None,
            image_furnace: None,
            image_electric_furnace: None,
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
            image_stone_ore: None,
            image_iron_plate: None,
            image_copper_plate: None,
            image_gear: None,
            image_copper_wire: None,
            image_circuit: None,
            image_steel_plate: None,
            image_time: None,
            image_underground_belt_item: None,
            board,
            terrain_params,
            structures,
            selected_structure_inventory: None,
            ore_harvesting: None,
            drop_items,
            drop_items_index: DropItemIndex::default(),
            on_player_update,
            on_popup_text,
            on_structure_destroy,
            temp_ents: vec![],
            rng: Xor128::new(3142125),
            // on_show_inventory,
            assets: Assets::new(&context, assets)?,
        };

        ret.update_cache()?;

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

        fn to_value<T: Serialize>(value: T, name: &str) -> Result<SValue, JsValue> {
            map_err(serde_json::to_value(value), name)
        }

        let mut map = serde_json::Map::new();
        map.insert("version".to_string(), to_value(&SAVE_VERSION, "version")?);
        map.insert("sim_time".to_string(), SValue::from(self.sim_time));
        map.insert("player".to_string(), to_value(&self.player, "player")?);
        map.insert(
            "viewport".to_string(),
            to_value(&self.viewport, "viewport")?,
        );
        map.insert("width".to_string(), serde_json::Value::from(self.width));
        map.insert("height".to_string(), serde_json::Value::from(self.height));
        if let Some(bounds) = self.bounds.as_ref() {
            map.insert("bounds".to_string(), to_value(bounds, "bounds")?);
        }
        map.insert(
            "structures".to_string(),
            serde_json::Value::from(
                self.structures
                    .iter()
                    .filter_map(|entry| entry.bundle.as_ref())
                    .map(|structure| {
                        let mut map = serde_json::Map::new();
                        map.insert(
                            "type".to_string(),
                            serde_json::Value::String(structure.dynamic.name().to_string()),
                        );
                        map.insert(
                            "payload".to_string(),
                            structure
                                .dynamic
                                .js_serialize()
                                .map_err(|e| js_str!("Serialize error: {}", e))?,
                        );
                        if let Some(position) = &structure.components.position {
                            map.insert(
                                "position".to_string(),
                                serde_json::to_value(position)
                                    .map_err(|e| js_str!("Position serialize error: {}", e))?,
                            );
                        }
                        if let Some(rotation) = &structure.components.rotation {
                            map.insert(
                                "rotation".to_string(),
                                serde_json::to_value(rotation)
                                    .map_err(|e| js_str!("Rotation serialize error: {}", e))?,
                            );
                        }
                        if let Some(burner) = &structure.components.burner {
                            map.insert(
                                "burner".to_string(),
                                burner
                                    .js_serialize()
                                    .map_err(|e| js_str!("Serialize error: {}", e))?,
                            );
                        }
                        if let Some(energy) = &structure.components.energy {
                            map.insert(
                                "energy".to_string(),
                                serde_json::to_value(energy)
                                    .map_err(|e| js_str!("Energy serialize error: {}", e))?,
                            );
                        }
                        if let Some(factory) = &structure.components.factory {
                            map.insert(
                                "factory".to_string(),
                                serde_json::to_value(factory)
                                    .map_err(|e| js_str!("Factory serialize error: {}", e))?,
                            );
                        }
                        let fluid_boxes = &structure.components.fluid_boxes;
                        if !fluid_boxes.is_empty() {
                            map.insert(
                                "fluid_boxes".to_string(),
                                fluid_boxes
                                    .iter()
                                    .map(|fbox| {
                                        serde_json::to_value(fbox)
                                            .map_err(|e| js_str!("FluidBox serialize error: {}", e))
                                    })
                                    .collect::<Result<serde_json::Value, JsValue>>()?,
                            );
                        }
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
            .filter(|(_, s)| s.bundle.is_some())
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
            serde_json::Value::from(
                self.drop_items
                    .iter()
                    .filter_map(|entry| entry.item.as_ref())
                    .map(serde_json::to_value)
                    .collect::<serde_json::Result<Vec<serde_json::Value>>>()
                    .map_err(|e| js_str!("Serialize error: {}", e))?,
            ),
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
                    .map(|chunk| {
                        Ok((
                            serde_json::to_value(chunk.0)?,
                            chunk
                                .1
                                .cells
                                .iter()
                                .enumerate()
                                .filter(|(_, cell)| cell.ore.is_some() || cell.water)
                                .map(|(idx, cell)| {
                                    let mut map = serde_json::Map::new();
                                    let x = idx % self.width as usize;
                                    let y = idx / self.height as usize;
                                    map.insert(
                                        "position".to_string(),
                                        serde_json::to_value((x, y))?,
                                    );
                                    map.insert("cell".to_string(), serde_json::to_value(cell)?);
                                    serde_json::to_value(map)
                                })
                                .collect::<serde_json::Result<Vec<serde_json::Value>>>()?,
                        ))
                    })
                    .collect::<serde_json::Result<Vec<_>>>()
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

        console_log!("deserialize");

        let mut json: Value =
            serde_json::from_str(&data).map_err(|_| js_str!("Deserialize error"))?;
        let version = json
            .get("version")
            .and_then(|value| value.as_i64())
            .ok_or_else(|| js_str!("Version not found!"))?;
        // Save version 2 is not backwards compatible
        if version < SAVE_VERSION {
            return js_err!("Save data version is old: {}", version);
        }

        self.structures.clear();
        self.drop_items.clear();

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

        self.viewport = json_take(&mut json, "viewport")
            .and_then(from_value)
            .unwrap_or_default();

        self.width = json_as_u64(json_get(&json, "width")?)? as u32;
        self.height = json_as_u64(json_get(&json, "height")?)? as u32;

        self.bounds = json_take(&mut json, "bounds")
            .and_then(from_value)
            .unwrap_or(None);

        let chunks = json
            .get_mut("board")
            .ok_or_else(|| js_str!("board not found in saved data"))?
            .as_array_mut()
            .ok_or_else(|| js_str!("board in saved data is not an array"))?;
        self.board = HashMap::new();
        for chunk in chunks {
            let chunk_pair = chunk
                .as_array_mut()
                .ok_or_else(|| js_str!("board in saved data is not an array"))?;
            let chunk_pos = chunk_pair
                .first_mut()
                .map(|i| std::mem::take(i))
                .ok_or_else(|| js_str!("Chunk does not have position"))?;
            let chunk_pos = from_value(chunk_pos)?;
            let chunk_data = chunk_pair
                .get_mut(1)
                .ok_or_else(|| js_str!("Chunk does not have data"))?
                .as_array_mut()
                .ok_or_else(|| js_str!("Chunk data is not an array"))?;
            let mut new_chunk = vec![Cell::default(); CHUNK_SIZE2];
            for tile in chunk_data {
                let position = json_get(tile, "position")?;
                let x: usize = json_as_u64(json_get(&position, 0)?)? as usize;
                let y: usize = json_as_u64(json_get(&position, 1)?)? as usize;
                new_chunk[x + y * CHUNK_SIZE] = from_value(json_take(tile, "cell")?)?;
            }
            self.board.insert(chunk_pos, Chunk::new(new_chunk));
        }
        calculate_back_image_all(&mut self.board);

        let structures = json
            .get_mut("structures")
            .ok_or_else(|| js_str!("structures not found in saved data"))?
            .as_array_mut()
            .ok_or_else(|| js_str!("structures in saved data is not an array"))?
            .iter_mut()
            .map(|structure| {
                Ok(StructureEntry {
                    gen: 0,
                    bundle: Some(Self::structure_from_json(structure)?),
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
            .filter_map(|s| s.bundle.as_ref()?.components.position)
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
            s.bundle
                .as_mut()
                .map(|b| {
                    b.dynamic
                        .on_construction_self(id, &mut b.components, &others, true)
                })
                .unwrap_or(Ok(()))?;
        }

        let s_d_iter = StructureDynIter::new_all(&mut self.structures);
        self.power_networks = build_power_networks(&s_d_iter, &self.power_wires);
        drop(s_d_iter);

        self.drop_items = json
            .get_mut("items")
            .ok_or_else(|| js_str!("\"items\" not found"))?
            .as_array_mut()
            .ok_or_else(|| js_str!("items in saved data is not an array"))?
            .into_iter()
            .map(|value| {
                Ok(DropItemEntry::from_value(
                    serde_json::from_value(std::mem::take(value))
                        .map_err(|e| js_str!("Item deserialization error: {:?}", e))?,
                ))
            })
            .collect::<Result<Vec<DropItemEntry>, JsValue>>()?;

        self.drop_items_index = build_index(&self.drop_items);

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
        mut f: impl FnMut(&mut Self, &mut StructureBundle, &StructureDynIter) -> Result<(), JsValue>,
    ) -> Result<(), JsValue> {
        // This is silly way to avoid borrow checker that temporarily move the structures
        // away from self so that they do not claim mutable borrow twice, but it works.
        let mut structures = std::mem::take(&mut self.structures);
        let mut res = Ok(());
        for i in 0..structures.len() {
            let (center, others) = StructureDynIter::new(&mut structures, i)?;
            if let Some(d) = center.bundle.as_mut() {
                res = f(self, d, &others);
                if res.is_err() {
                    break;
                }
            }
        }
        self.structures = structures;
        res
    }

    // fn update_fluid_connections(&mut self, position: &Position) -> Result<(), JsValue> {
    //     self.proc_structures_mutual(|state, neighbor, others| {
    //         if !neighbor
    //             .components
    //             .position
    //             .map(|s_pos| s_pos.is_neighbor(&position))
    //             .unwrap_or(false)
    //         {
    //             return Ok(());
    //         }
    //         let connections =
    //             neighbor
    //                 .dynamic
    //                 .connection(&neighbor.components, state, others.as_dyn_iter());
    //         console_log!(
    //             "Connection recalculated for {:?}: {:?}",
    //             neighbor.components.position,
    //             connections
    //         );
    //         for fbox in &mut neighbor.components.fluid_boxes {
    //             fbox.connect_to = connections;
    //         }
    //         Ok(())
    //     })
    // }

    fn get_pair_mut(
        &mut self,
        a: usize,
        b: usize,
    ) -> (
        Option<(StructureId, &mut StructureBundle)>,
        Option<(StructureId, &mut StructureBundle)>,
    ) {
        if a < b {
            let (left, right) = self.structures.split_at_mut(b);
            let a_gen = left[a].gen;
            (
                left[a].bundle.as_mut().map(|s| {
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
                            s.bundle.as_mut()?,
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
                            s.bundle.as_mut()?,
                        ))
                    })
                    .flatten(),
                left[b].bundle.as_mut().map(|s| {
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

    fn get_structure(&self, id: StructureId) -> Option<&StructureBundle> {
        self.structures
            .iter()
            .enumerate()
            .find(|(i, s)| id.id == *i as u32 && id.gen == s.gen)
            .map(|(_, s)| s.bundle.as_ref())
            .flatten()
    }

    fn update_fluid_connections(&mut self, position: &Position) -> Result<(), JsValue> {
        if let Some(i) = self
            .structures
            .iter()
            .enumerate()
            .find(|s| {
                s.1.bundle
                    .as_ref()
                    .map(|a| {
                        a.components.position == Some(*position)
                            && !a.components.fluid_boxes.is_empty()
                    })
                    .unwrap_or(false)
            })
            .map(|v| v.0)
        {
            for j in 0..self.structures.len() {
                if i != j {
                    if let (Some(a), Some(b)) = self.get_pair_mut(i, j) {
                        let (aid, bid) = (a.0, b.0);
                        let a_con = a.1.dynamic.fluid_connections(&a.1.components);
                        let b_con = b.1.dynamic.fluid_connections(&b.1.components);
                        if let Some(idx) = try_continue!(a.1.components.position)
                            .neighbor_index(try_continue!(&b.1.components.position))
                            .filter(|f| a_con[*f as usize] && b_con[(*f as usize + 2) % 4])
                        {
                            a.1.components
                                .fluid_boxes
                                .iter_mut()
                                .for_each(|fb| fb.connect_to[(idx as usize + 2) % 4] = Some(bid));
                            b.1.components
                                .fluid_boxes
                                .iter_mut()
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
                    .and_then(|s| s.bundle.as_mut())
                    .and_then(|s| Some((position.neighbor_index(&s.components.position?)?, s)))
                {
                    b.components
                        .fluid_boxes
                        .iter_mut()
                        .for_each(|fb| fb.connect_to[idx as usize] = None);
                }
            }
        }

        Ok(())
    }

    pub fn simulate(&mut self, delta_time: f64) -> Result<js_sys::Array, JsValue> {
        let start_simulate = performance().now();
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
                events.push(
                    JsValue::from_serde(&JSEvent::UpdateStructureInventory(pos.x, pos.y)).unwrap(),
                )
            }
        };

        self.ore_harvesting = (|| {
            let mut ore_harvesting = self.ore_harvesting?;
            let mut ret = true;
            if (ore_harvesting.timer + 1) % ORE_HARVEST_TIME < ore_harvesting.timer {
                console_log!("harvesting {:?}...", ore_harvesting.ore_type);
                let tile = self.tile_at_mut(&ore_harvesting.pos)?;
                let ore = tile.ore.as_mut()?;
                let expected_ore = match ore_harvesting.ore_type {
                    ItemType::IronOre => Ore::Iron,
                    ItemType::CopperOre => Ore::Copper,
                    ItemType::CoalOre => Ore::Coal,
                    ItemType::StoneOre => Ore::Stone,
                    _ => return None,
                };
                if expected_ore != ore.0 {
                    return None;
                }
                if 0 < ore.1 {
                    ore.1 -= 1;
                    if ore.1 == 0 {
                        tile.ore = None;
                        ret = false;
                    }
                    self.player.add_item(&ore_harvesting.ore_type, 1);
                    self.on_player_update
                        .call1(&window(), &JsValue::from(self.get_player_inventory().ok()?))
                        .unwrap_or_else(|_| JsValue::from(true));
                    if let Err(e) = self.new_popup_text(
                        format!("+1 {:?}", ore_harvesting.ore_type),
                        ore_harvesting.pos.x as f64 * TILE_SIZE,
                        ore_harvesting.pos.y as f64 * TILE_SIZE,
                    ) {
                        console_log!("Add text error: {:?}", e);
                    }
                } else {
                    ret = false;
                }
            }
            ore_harvesting.timer = (ore_harvesting.timer + 1) % ORE_HARVEST_TIME;
            if ret {
                Some(ore_harvesting)
            } else {
                None
            }
        })();

        let start_structures = performance().now();
        // This is silly way to avoid borrow checker that temporarily move the structures
        // away from self so that they do not claim mutable borrow twice, but it works.
        let mut structures = std::mem::take(&mut self.structures);
        for i in 0..structures.len() {
            let (center, mut others) = StructureDynIter::new(&mut structures, i)?;
            if let Some(bundle) = center.bundle.as_mut() {
                frame_proc_result_to_event(
                    bundle.dynamic.frame_proc(
                        StructureId {
                            id: i as u32,
                            gen: center.gen,
                        },
                        &mut bundle.components,
                        self,
                        &mut others,
                    ), // dynamic.frame_proc(self, &mut Chained(MutRef(front), MutRef(last)))
                );
                let components = &mut bundle.components;
                if let Some(position) = &components.position {
                    for fbox in &mut components.fluid_boxes {
                        fbox.simulate(position, self, &mut others)
                    }
                }
                if let Some(burner) = &mut components.burner {
                    frame_proc_result_to_event(burner.frame_proc(
                        components.position.as_mut(),
                        components.energy.as_mut(),
                        bundle.dynamic.as_mut(),
                    ));
                }
                if let Some(factory) = &mut components.factory {
                    frame_proc_result_to_event(
                        factory
                            .frame_proc(components.position.as_mut(), components.energy.as_mut()),
                    );
                }
            }
        }
        self.perf_structures
            .add(performance().now() - start_structures);

        let start_index = performance().now();
        let index = &mut self.drop_items_index; //build_index(&self.drop_items);
        for i in 0..self.drop_items.len() {
            // (id, item) in drop_item_id_iter_mut(&mut self.drop_items) {
            let entry = &self.drop_items[i];
            let item = if let Some(item) = entry.item.as_ref() {
                item
            } else {
                continue;
            };
            let id = DropItemId::new(i as u32, entry.gen);
            // if 0 < item.x
            //     && item.x < self.width as i32 * tilesize
            //     && 0 < item.y
            //     && item.y < self.height as i32 * tilesize
            // {
            //     if let Some(item_response_result) = structures
            //         .iter_mut()
            //         .filter_map(|s| s.bundle.as_mut())
            //         .find(|s| {
            //             s.dynamic.contains(
            //                 &s.components,
            //                 &Position {
            //                     x: item.x / TILE_SIZE_I,
            //                     y: item.y / TILE_SIZE_I,
            //                 },
            //             )
            //         })
            //         .and_then(|structure| {
            //             structure
            //                 .dynamic
            //                 .item_response(&mut structure.components, item)
            //                 .ok()
            //         })
            //     {
            //         match item_response_result.0 {
            //             ItemResponse::None => {}
            //             ItemResponse::Move(moved_x, moved_y) => {
            //                 if hit_check_with_index(
            //                     &self.drop_items,
            //                     &index,
            //                     moved_x,
            //                     moved_y,
            //                     Some(id),
            //                 ) {
            //                     continue;
            //                 }
            //                 let position = Position {
            //                     x: moved_x / 32,
            //                     y: moved_y / 32,
            //                 };
            //                 if let Some(s) = structures
            //                     .iter()
            //                     .filter_map(|s| s.bundle.as_ref())
            //                     .find(|s| s.dynamic.contains(&s.components, &position))
            //                 {
            //                     if !s.dynamic.movable() {
            //                         continue;
            //                     }
            //                 } else {
            if let Some(bounds) = self.bounds.as_ref() {
                if !(0 < item.x
                    && item.x < bounds.width * tilesize
                    && 0 < item.y
                    && item.y < bounds.height * tilesize)
                {
                    continue;
                }
            }
            if let Some(item_response_result) = structures
                .iter_mut()
                .filter_map(|s| s.bundle.as_mut())
                .find(|s| {
                    s.dynamic.contains(
                        &s.components,
                        &Position {
                            x: item.x.div_euclid(TILE_SIZE_I),
                            y: item.y.div_euclid(TILE_SIZE_I),
                        },
                    )
                })
                .and_then(|bundle| {
                    bundle
                        .dynamic
                        .item_response(&mut bundle.components, item)
                        .ok()
                })
            {
                match item_response_result.0 {
                    ItemResponse::Move(moved_x, moved_y) => {
                        if hit_check_with_index(
                            &self.drop_items,
                            &index,
                            moved_x,
                            moved_y,
                            Some(id),
                        ) {
                            continue;
                        }
                        let position = Position {
                            x: moved_x.div_euclid(TILE_SIZE_I),
                            y: moved_y.div_euclid(TILE_SIZE_I),
                        };
                        if let Some(s) = structures
                            .iter()
                            .filter_map(|s| s.bundle.as_ref())
                            .find(|s| s.dynamic.contains(&s.components, &position))
                        {
                            if !s.dynamic.movable() {
                                continue;
                            }
                        } else {
                            continue;
                        }
                        update_index(index, id, item.x, item.y, moved_x, moved_y);
                        let item = self.drop_items[i].item.as_mut().unwrap();
                        item.x = moved_x;
                        item.y = moved_y;
                    }
                    ItemResponse::Consume => {
                        remove_index(index, id, item.x, item.y);
                        self.drop_items[i].item = None;
                    }
                    ItemResponse::None => (),
                }
                if let Some(result) = item_response_result.1 {
                    frame_proc_result_to_event(Ok(result));
                }
            }
        }
        self.perf_drop_items.add(performance().now() - start_index);

        self.structures = structures;

        // Actually, taking away, filter and collect is easier than removing expied objects
        // one by one.
        self.temp_ents = std::mem::take(&mut self.temp_ents)
            .into_iter()
            .map(|mut ent| {
                ent.position.0 += delta_time * ent.velocity.0;
                ent.position.1 -= delta_time * ent.velocity.1;
                ent.life -= delta_time;
                ent
            })
            .filter(|ent| 0. < ent.life)
            .collect();

        self.perf_simulate.add(performance().now() - start_simulate);

        // self.drop_items = drop_items;
        self.update_info();
        Ok(events.iter().collect())
    }

    fn tile_at(&self, tile: &Position) -> Option<Cell> {
        let (chunk_pos, mp) = tile.div_mod(CHUNK_SIZE as i32);
        let chunk = self.board.get(&chunk_pos)?;
        if 0 <= mp.x && mp.x < CHUNK_SIZE as i32 && 0 <= mp.y && mp.y < CHUNK_SIZE as i32 {
            Some(chunk.cells[mp.x as usize + mp.y as usize * CHUNK_SIZE])
        } else {
            None
        }
    }

    fn tile_at_mut(&mut self, tile: &Position) -> Option<&mut Cell> {
        let (chunk_pos, mp) = tile.div_mod(CHUNK_SIZE as i32);
        let chunk = self.board.get_mut(&chunk_pos)?;
        if 0 <= mp.x && mp.x < CHUNK_SIZE as i32 && 0 <= mp.y && mp.y < CHUNK_SIZE as i32 {
            Some(&mut chunk.cells[mp.x as usize + mp.y as usize * CHUNK_SIZE])
        } else {
            None
        }
    }

    /// Look up a structure at a given tile coordinates
    fn find_structure_tile(&self, tile: &[i32]) -> Option<&StructureBundle> {
        self.structures
            .iter()
            .filter_map(|s| s.bundle.as_ref())
            .find(|s| {
                s.components.position
                    == Some(Position {
                        x: tile[0],
                        y: tile[1],
                    })
            })
    }

    /// Mutable variant of find_structure_tile
    fn find_structure_tile_mut(&mut self, tile: &[i32]) -> Option<&mut StructureBundle> {
        self.structures
            .iter_mut()
            .find(|s| {
                s.bundle
                    .as_ref()
                    .map(|bundle| {
                        bundle.components.position
                            == Some(Position {
                                x: tile[0],
                                y: tile[1],
                            })
                    })
                    .unwrap_or(false)
            })
            .map(|entry| entry.bundle.as_mut())
            .flatten()
    }

    /// Dirty hack to enable modifying a structure in an array.
    /// Instead of returning mutable reference, return an index into the array, so the
    /// caller can directly reference the structure from array `self.structures[idx]`.
    ///
    /// Because mutable version of find_structure_tile doesn't work.
    fn find_structure_tile_idx(&self, position: Position) -> Option<usize> {
        self.structure_iter()
            .enumerate()
            .find(|(_, s)| s.dynamic.contains(&s.components, &position))
            .map(|(idx, _)| idx)
    }

    fn find_structure_tile_id(&self, tile: &[i32]) -> Option<(StructureId, &StructureBundle)> {
        self.structure_id_iter()
            .find(|(_, bundle)| {
                bundle
                    .components
                    .position
                    .map(|pos| pos.x == tile[0] && pos.y == tile[1])
                    .unwrap_or(false)
            })
            .map(|(id, s)| (id, s))
    }

    fn find_structure_by_id(&self, id: StructureId) -> Option<&StructureBundle> {
        let s = self.structures.get(id.id as usize)?;
        if s.gen == id.gen {
            Some(s.bundle.as_ref()?)
        } else {
            None
        }
    }

    // fn find_structure_by_id_mut(&mut self, id: StructureId) -> Option<&mut dyn Structure> {
    //     let s = self.structures.get_mut(id.id as usize)?;
    //     if s.gen == id.gen {
    //         Some(s.dynamic.as_deref_mut()?)
    //     } else {
    //         None
    //     }
    // }

    /// Find structure by id and return a mutable reference along with player.
    ///
    /// Because the combination of these two often occurs in inventory transfer code, we made it a common method.
    fn find_structure_by_id_mut_and_player(
        &mut self,
        id: StructureId,
    ) -> Option<(&mut StructureBundle, &mut Player)> {
        let s = self.structures.get_mut(id.id as usize)?;
        if s.gen == id.gen {
            Some((s.bundle.as_mut()?, &mut self.player))
        } else {
            None
        }
    }

    /// `find_structure_by_id_mut_and_player` with common error string.
    fn find_structure_by_id_mut_and_player_err(
        &mut self,
        id: StructureId,
    ) -> Result<(&mut StructureBundle, &mut Player), JsValue> {
        self.find_structure_by_id_mut_and_player(id)
            .ok_or_else(|| js_str!("structure id {:?} not found at position", id))
    }

    // fn find_structure_tile_mut<'a>(&'a mut self, tile: &[i32]) -> Option<&'a mut dyn Structure> {
    //     self.structures
    //         .iter_mut()
    //         .find(|s| s.position().x == tile[0] && s.position().y == tile[1])
    //         .map(|s| s.as_mut())
    // }

    fn _find_structure(&self, pos: &[f64]) -> Option<&StructureBundle> {
        self.find_structure_tile(&[(pos[0] / 32.) as i32, (pos[1] / 32.) as i32])
    }

    fn find_item(&self, pos: &Position) -> Option<(DropItemId, &DropItem)> {
        drop_item_id_iter(&self.drop_items).find(|(_, item)| {
            item.x.div_euclid(TILE_SIZE_I) == pos.x && item.y.div_euclid(TILE_SIZE_I) == pos.y
        })
    }

    fn remove_item(&mut self, id: DropItemId) -> Option<DropItem> {
        if let Some(entry) = self.drop_items.get_mut(id.id as usize) {
            entry.item.take()
        } else {
            None
        }
    }

    fn _remove_item_pos(&mut self, pos: &Position) -> Option<DropItem> {
        if let Some(entry) = self.drop_items.iter_mut().find(|item| {
            if let Some(item) = item.item.as_ref() {
                item.x / 32 == pos.x && item.y / 32 == pos.y
            } else {
                false
            }
        }) {
            entry.item.take()
        } else {
            None
        }
    }

    fn update_info(&self) {
        if let Some(cursor) = self.cursor {
            if let Some(ref elem) = self.info_elem {
                elem.set_inner_html(
                    &if let Some(structure) = self.find_structure_tile(&cursor) {
                        format!(
                            r#"Type: {}<br>{}"#,
                            structure.dynamic.name(),
                            structure.dynamic.desc(&structure.components, &self)
                        )
                    } else {
                        let (chunk_pos, mp) =
                            Position::new(cursor[0], cursor[1]).div_mod(CHUNK_SIZE as i32);
                        if let Some(chunk) = self.board.get(&chunk_pos) {
                            let cell = &chunk.cells[mp.x as usize + mp.y as usize * CHUNK_SIZE];
                            format!(
                                r#"Empty tile<br>
                                {}<br>"#,
                                if let Some(ore) = cell.ore.as_ref() {
                                    format!("{:?}: {}", ore.0, ore.1)
                                } else {
                                    "No ore".to_string()
                                }
                            )
                        } else {
                            "Empty tile".to_string()
                        }
                    },
                );
            }
        }
    }

    fn rotate(&mut self) -> Result<bool, RotateErr> {
        if let Some(SelectedItem::ToolBelt(_selected_tool)) = self.selected_item {
            self.tool_rotation = self.tool_rotation.next();
            Ok(true)
        } else if let Some(SelectedItem::PlayerInventory(_item, _count)) = self.selected_item {
            self.tool_rotation = self.tool_rotation.next();
            Ok(true)
        } else {
            if let Some(ref cursor) = self.cursor {
                if let Some(idx) = self.find_structure_tile_idx(Position {
                    x: cursor[0],
                    y: cursor[1],
                }) {
                    let mut structures = std::mem::take(&mut self.structures);
                    if let Ok((
                        StructureEntry {
                            bundle: Some(ref mut bundle),
                            ..
                        },
                        others,
                    )) = StructureDynIter::new(&mut structures, idx)
                    {
                        match bundle.rotate(self, &others) {
                            Ok(()) => (),
                            // Rotation error is not a hard error; gracefully ignore
                            Err(s) => console_log!("rotate returned err: {:?}", s),
                        }
                    }
                    self.structures = structures;
                    return Ok(false);
                }
            }
            Err(RotateErr::NotFound)
        }
    }

    /// Insert an object on the board.  It could fail if there's already some object at the position.
    fn new_object(&mut self, pos: &Position, type_: ItemType) -> Result<(), NewObjectErr> {
        let cell = self.tile_at(pos).ok_or(NewObjectErr::OutOfMap)?;
        if cell.water {
            return Err(NewObjectErr::OnWater);
        }
        if let Some(bounds) = self.bounds.as_ref() {
            if !(0 <= pos.x && pos.x < bounds.width && 0 <= pos.y && pos.y < bounds.height) {
                return Err(NewObjectErr::OutOfMap);
            }
        }
        if let Some(stru) = self.find_structure_tile(&[pos.x, pos.y]) {
            if !stru.dynamic.movable() {
                return Err(NewObjectErr::BlockedByStructure);
            }
        }
        let item = DropItem::new(type_, pos.x, pos.y);
        // return board[c + r * ysize].structure.input(obj);
        if hit_check(&self.drop_items, item.x, item.y, None) {
            return Err(NewObjectErr::BlockedByItem);
        }
        let (x, y) = (item.x, item.y);
        let entry = self
            .drop_items
            .iter_mut()
            .enumerate()
            .find(|(_, entry)| entry.item.is_none());
        let id = if let Some((i, entry)) = entry {
            entry.item = Some(item);
            entry.gen += 1;
            DropItemId {
                id: i as u32,
                gen: entry.gen,
            }
        } else {
            let obj = DropItemEntry::from_value(item);
            let i = self.drop_items.len();
            self.drop_items.push(obj);
            DropItemId {
                id: i as u32,
                gen: 0,
            }
        };
        add_index(&mut self.drop_items_index, id, x, y);
        Ok(())
    }

    fn harvest(&mut self, position: &Position, clear_item: bool) -> Result<bool, JsValue> {
        let mut harvested_structure = false;
        let mut popup_text = String::new();

        for i in 0..self.structures.len() {
            if !self.structures[i]
                .bundle
                .as_ref()
                .map(|d| d.dynamic.contains(&d.components, position))
                .unwrap_or(false)
            {
                continue;
            }
            let mut structure = self.structures[i]
                .bundle
                .take()
                .expect("should be active entity");
            let gen = self.structures[i].gen;
            self.structures[i].gen += 1;
            self.player
                .inventory
                .add_item(&str_to_item(&structure.dynamic.name()).ok_or_else(|| {
                    JsValue::from_str(&format!(
                        "wrong structure name: {:?}",
                        structure.dynamic.name()
                    ))
                })?);
            popup_text += &format!("+1 {}\n", structure.dynamic.name());

            let mut structures = std::mem::take(&mut self.structures);
            for i in 0..structures.len() {
                let (notify_structure, others) = StructureDynIter::new(&mut structures, i)?;
                if let Some(s) = notify_structure.bundle.as_mut() {
                    match s.dynamic.on_construction(
                        &mut s.components,
                        StructureId { id: i as u32, gen },
                        &structure,
                        &others,
                        false,
                    ) {
                        Ok(()) => (),
                        Err(s) => {
                            drop(others);
                            self.structures = structures;
                            return Err(s);
                        }
                    }
                }
            }
            self.structures = structures;

            let position = try_continue!(structure.components.position);
            self.power_wires = std::mem::take(&mut self.power_wires)
                .into_iter()
                .filter(|power_wire| power_wire.0.id != i as u32 && power_wire.1.id != i as u32)
                .collect();
            let destroyed_id = StructureId { id: i as u32, gen };
            structure.dynamic.on_construction_self(
                StructureId { id: i as u32, gen },
                &mut structure.components,
                &StructureDynIter::new_all(&mut self.structures),
                false,
            )?;
            // if let Ok(ref mut data) = self.minimap_buffer.try_borrow_mut() {
            //     self.render_minimap_data_pixel(data, &position);
            // }
            let mut chunks = std::mem::take(&mut self.board);
            self.render_minimap_data_pixel(&mut chunks, &position);
            self.board = chunks;
            for (item_type, count) in structure.dynamic.destroy_inventory().into_iter().chain(
                structure
                    .components
                    .burner
                    .as_mut()
                    .map(|burner| burner.destroy_inventory())
                    .unwrap_or_else(|| Inventory::new())
                    .into_iter()
                    .chain(
                        structure
                            .components
                            .factory
                            .as_mut()
                            .map(|factory| factory.destroy_inventory())
                            .unwrap_or_else(|| Inventory::new())
                            .into_iter(),
                    ),
            ) {
                popup_text += &format!("+{} {}\n", count, &item_to_str(&item_type));
                self.player.add_item(&item_type, count)
            }

            self.power_networks = build_power_networks(
                &StructureDynIter::new_all(&mut self.structures),
                &self.power_wires,
            );

            self.update_fluid_connections(&position)?;

            self.on_player_update
                .call1(&window(), &JsValue::from(self.get_player_inventory()?))?;

            self.on_structure_destroy.call1(
                &window(),
                &JsValue::from_bool(self.selected_structure_inventory == Some(destroyed_id)),
            )?;

            harvested_structure = true;
        }
        let mut harvested_items = false;
        if !harvested_structure && clear_item {
            // Pick up dropped items in the cell
            let mut picked_items = Inventory::new();
            for entry in &mut self.drop_items {
                let item = if let Some(item) = entry.item.as_ref() {
                    item
                } else {
                    continue;
                };

                if !(item.x.div_euclid(TILE_SIZE_I) == position.x
                    && item.y.div_euclid(TILE_SIZE_I) == position.y)
                {
                    continue;
                }
                let item_type = item.type_;
                entry.item = None;
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
            )?;
        }
        Ok(harvested_structure || harvested_items)
    }

    fn flatten_inventory(inventory: &[(ItemType, usize)]) -> Vec<(ItemType, usize)> {
        let mut ret = vec![];
        for pair in inventory {
            let mut amount = pair.1;
            while STACK_SIZE < amount {
                ret.push((pair.0, amount.min(STACK_SIZE)));
                amount -= STACK_SIZE;
            }
            ret.push((pair.0, amount));
        }
        ret
    }

    fn inventory_to_vec(
        structure: &StructureBundle,
        inv_type: InventoryType,
    ) -> Option<Vec<(ItemType, usize)>> {
        let inventory = if let Some(inv) = structure.inventory(inv_type) {
            inv
        } else {
            return None;
        };
        Some(if inv_type == InventoryType::Storage {
            let mut v = inventory
                .iter()
                .map(|(item, count)| (*item, *count))
                .collect::<Vec<_>>();
            v.sort();
            Self::flatten_inventory(&v)
        } else {
            let mut inventory = std::borrow::Cow::Borrowed(inventory);
            if inv_type == InventoryType::Input {
                if let Some(recipe) = structure.dynamic.get_selected_recipe() {
                    let inventory = inventory.to_mut();
                    for key in recipe.input.keys() {
                        if !inventory.contains_key(key) {
                            inventory.insert(*key, 0);
                        }
                    }
                }
            }
            let mut v = inventory
                .iter()
                .map(|(item, count)| (*item, *count))
                .collect::<Vec<_>>();
            v.sort();
            v
        })
    }

    /// Construct JS array representing inventory contents.
    ///
    /// Optionally flattens the inventory for easier selection of small amount.
    /// Some inventory types, namely input, output and burner, does not benefit
    /// from splitting, because they are usually moved all at once, unlike storage.
    ///
    fn inventory_to_js(
        &self,
        inventory: &Inventory,
        selected_item: &Option<ItemType>,
        flatten: bool,
    ) -> Result<js_sys::Array, JsValue> {
        let mut v = inventory
            .iter()
            .map(|(item, count)| (*item, *count))
            .collect::<Vec<_>>();
        v.sort();
        if flatten {
            self.vec_to_js(&Self::flatten_inventory(&v), selected_item)
        } else {
            self.vec_to_js(&v, selected_item)
        }
    }

    /// Construct a JS object representing inventory from a vector slice.
    ///
    /// The returned object is 2-array of
    ///          * inventory (array), each of which element consists of
    ///              * name (string)
    ///              * count (int)
    ///          * selected item (string)
    fn vec_to_js(
        &self,
        inventory: &[(ItemType, usize)],
        selected_item: &Option<ItemType>,
    ) -> Result<js_sys::Array, JsValue> {
        Ok(js_sys::Array::of2(
            &JsValue::from({
                inventory
                    .iter()
                    .map(|(type_, count)| {
                        js_sys::Array::of2(
                            &JsValue::from_str(&item_to_str(type_)),
                            &JsValue::from_f64(*count as f64),
                        )
                    })
                    .collect::<js_sys::Array>()
            }),
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
        self.inventory_to_js(
            &self.player.inventory,
            &self.selected_item.and_then(|item| {
                if let SelectedItem::PlayerInventory(i, _) = item {
                    Some(i)
                } else {
                    None
                }
            }),
            true,
        )
    }

    pub fn select_player_inventory(
        &mut self,
        idx: usize,
        right_click: bool,
    ) -> Result<(), JsValue> {
        let mut v = self
            .player
            .inventory
            .iter()
            .map(|(item, count)| (*item, *count))
            .collect::<Vec<_>>();
        v.sort();
        let flat_inv = Self::flatten_inventory(&v);
        self.selected_item = Some(
            flat_inv
                .get(idx)
                .map(|i| {
                    SelectedItem::PlayerInventory(i.0, if right_click { i.1 / 2 } else { i.1 })
                })
                .ok_or_else(|| JsValue::from_str("Item name not identified"))?,
        );
        Ok(())
    }

    /// Deselect is a separate function from select because wasm-bindgen cannot overload Option
    pub fn deselect_inventory(&mut self) -> Result<(), JsValue> {
        self.selected_item = None;
        Ok(())
    }

    pub fn open_structure_inventory(&mut self, c: i32, r: i32) -> Result<bool, JsValue> {
        let pos = Position { x: c, y: r };
        // if let Some(s) = self.find_structure_tile(&[pos.x, pos.y]) {
        if let Some((id, s)) = self.find_structure_tile_id(&[pos.x, pos.y]) {
            let recipe_enable = !s.dynamic.get_recipes().is_empty();
            self.selected_structure_inventory = Some(id);
            Ok(recipe_enable)
        } else {
            Err(JsValue::from_str("structure not found"))
        }
    }

    /// Returns currently selected structure's coordinates in 2-array or `null` if none selected
    pub fn get_selected_inventory(&self) -> Result<JsValue, JsValue> {
        if let Some(pos) = self
            .selected_structure_inventory
            .and_then(|id| self.find_structure_by_id(id))
            .and_then(|s| s.components.position)
        {
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
    /// @param inventory_type a string indicating type of the inventory in the structure
    pub fn get_structure_inventory(
        &self,
        x: i32,
        y: i32,
        inventory_type: JsValue,
    ) -> Result<js_sys::Array, JsValue> {
        let inventory_type = InventoryType::try_from(inventory_type)?;
        if let Some((id, inventory)) = self
            .structure_id_iter()
            .find(|(_, d)| d.components.position == Some(Position { x, y }))
            .and_then(|(id, s)| Some((id, Self::inventory_to_vec(s, inventory_type)?)))
        {
            return self.vec_to_js(
                &inventory,
                &self.selected_item.and_then(|item| item.map_struct(&id)),
            );
        }

        // We do not make getting inventory of nonexist structure or inventory an error, instead return an empty one.
        // Because JavaScript side cannot track the object lifecycle, it is very easy to happen and it's annoying to
        // make it a hard error.
        Ok(js_sys::Array::new())
        // Err(JsValue::from_str(
        //     "structure is not found or doesn't have inventory",
        // ))
    }

    pub fn get_structure_progress(&self, x: i32, y: i32) -> Option<f64> {
        self.find_structure_tile(&[x, y]).and_then(|structure| {
            structure
                .components
                .factory
                .as_ref()
                .and_then(|factory| factory.progress)
                .or_else(|| structure.dynamic.get_progress())
        })
    }

    pub fn get_structure_burner_energy(&self, c: i32, r: i32) -> Option<js_sys::Array> {
        self.find_structure_tile(&[c, r]).and_then(|structure| {
            let energy = structure.components.energy.as_ref()?;
            Some(js_sys::Array::of2(
                &JsValue::from(energy.value),
                &JsValue::from(energy.max),
            ))
        })
    }

    /// Select an item in a structure inventory at index `idx`. If a structure was not previously selected, ignored.
    pub fn select_structure_inventory(
        &mut self,
        idx: usize,
        inventory_type: JsValue,
        right_click: bool,
    ) -> Result<(), JsValue> {
        let inv_type = InventoryType::try_from(inventory_type)?;

        let (id, flat_inv) = self
            .selected_structure_inventory
            .and_then(|id| Some((id, self.find_structure_by_id(id)?)))
            .and_then(|(id, s)| Some((id, Self::inventory_to_vec(s, inv_type)?)))
            .ok_or_else(|| js_str!("Structure does not have inventory type"))?;
        let item = flat_inv
            .get(idx)
            .ok_or_else(|| JsValue::from("Item name not valid"))?;
        self.selected_item = Some(SelectedItem::StructInventory(
            id,
            inv_type,
            item.0,
            if right_click { item.1 / 2 } else { item.1 },
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
                    .dynamic
                    .get_recipes()
                    .into_owned()
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
            if let Some(factory) = structure.components.factory.as_mut() {
                structure.dynamic.select_recipe(factory, index)
            } else {
                js_err!("Structure cannot set recipe")
            }
        } else {
            Err(JsValue::from_str("Structure is not found"))
        }
    }

    pub fn set_alt_mode(&mut self, value: bool) {
        self.alt_mode = value;
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

    pub fn set_use_webgl_instancing(&mut self, value: bool) {
        self.use_webgl_instancing = value;
    }

    /// We try to insert in this order. We don't want to insert into output.
    fn inventory_move_order(item: &ItemType) -> [InventoryType; 3] {
        if *item == ItemType::CoalOre {
            // If it was fuel, prefer putting it into burner inventory
            [
                InventoryType::Burner,
                InventoryType::Input,
                InventoryType::Storage,
            ]
        } else {
            [
                InventoryType::Input,
                InventoryType::Burner,
                InventoryType::Storage,
            ]
        }
    }

    /// Move inventory items between structure and player
    /// @param to_player whether the movement happen towards player
    /// @param inventory_type a string indicating type of the inventory in the structure
    /// @param all attempt to move all items of the same kind (ctrl key)
    pub fn move_selected_inventory_item(
        &mut self,
        to_player: bool,
        inventory_type: JsValue,
        all: bool,
    ) -> Result<bool, JsValue> {
        let inventory_type = InventoryType::try_from(inventory_type)?;
        let id = if let Some(id) = self.selected_structure_inventory {
            id
        } else {
            return Ok(false);
        };

        if to_player {
            if let Some(SelectedItem::StructInventory(id, sel_inventory_type, item, count)) =
                self.selected_item
            {
                console_log!("selected_structure_inventory: {:?}", id);
                let (structure, player) = self.find_structure_by_id_mut_and_player_err(id)?;
                let count = if all {
                    structure
                        .inventory(sel_inventory_type)
                        .map(|i| i.count_item(&item))
                        .unwrap_or(0)
                } else {
                    count
                };

                player.inventory.add_items(
                    &item,
                    structure
                        .add_inventory(sel_inventory_type, &item, -(count as isize))
                        .abs() as usize,
                );
                self.on_player_update
                    .call1(&window(), &JsValue::from(self.get_player_inventory()?))?;
                return Ok(count != 0);
            }
        } else {
            if let Some(SelectedItem::PlayerInventory(item, count)) = self.selected_item {
                let (structure, player) = self.find_structure_by_id_mut_and_player_err(id)?;
                let mut try_move = |inventory: &mut Inventory, inventory_type| {
                    let count = if all {
                        inventory.count_item(&item)
                    } else {
                        inventory.count_item(&item).min(count)
                    };
                    inventory.remove_items(
                        &item,
                        structure
                            .add_inventory(inventory_type, &item, count as isize)
                            .abs() as usize,
                    )
                };

                if all {
                    let try_order = Self::inventory_move_order(&item);

                    for invtype in try_order {
                        let moved_count = try_move(&mut player.inventory, invtype);
                        if moved_count != 0 {
                            self.on_player_update
                                .call1(&window(), &JsValue::from(self.get_player_inventory()?))?;
                            return Ok(moved_count != 0);
                        }
                    }
                } else {
                    let moved_count = try_move(&mut player.inventory, inventory_type);
                    self.on_player_update
                        .call1(&window(), &JsValue::from(self.get_player_inventory()?))?;
                    return Ok(moved_count != 0);
                }
            }
        }
        Ok(false)
    }

    pub fn move_all_inventory_items(
        &mut self,
        to_player: bool,
        inventory_type: JsValue,
    ) -> Result<bool, JsValue> {
        let inventory_type = InventoryType::try_from(inventory_type)?;
        let id = if let Some(id) = self.selected_structure_inventory {
            id
        } else {
            return Ok(false);
        };
        let (structure, player) = self.find_structure_by_id_mut_and_player_err(id)?;
        if to_player {
            // Player has no capacity limit, so copy everything by taking
            if let Some(inventory) = structure.inventory_mut(inventory_type) {
                let ret = !inventory.is_empty();
                player.inventory.merge(std::mem::take(inventory));
                inventory.clear();
                self.on_player_update
                    .call1(&window(), &JsValue::from(self.get_player_inventory()?))?;
                return Ok(ret);
            }
        } else {
            // Structure's inventory has capacity limit, so we need to check one item at a time
            // and exit if we fail to add a new item.
            let mut ret = false;
            let items = player
                .inventory
                .iter()
                .map(|(item, count)| (*item, *count))
                .collect::<Vec<_>>();
            for (item, count) in items {
                let try_order = Self::inventory_move_order(&item);

                for invtype in try_order {
                    let moved_count = structure.add_inventory(invtype, &item, count as isize);
                    if moved_count == 0 {
                        continue;
                    }
                    if player.inventory.remove_items(&item, moved_count as usize) != 0 {
                        ret = true;
                        break;
                    } else {
                        panic!("We have checked player has {} {:?}, no?", count, item);
                    }
                }
            }
            return Ok(ret);
        }

        Ok(false)
    }

    fn new_structure(
        &self,
        tool: &ItemType,
        cursor: &Position,
    ) -> Result<StructureBundle, JsValue> {
        match tool {
            ItemType::TransportBelt => {
                return Ok(TransportBelt::new(*cursor, self.tool_rotation));
            }
            ItemType::Inserter => {
                return Ok(Inserter::new(*cursor, self.tool_rotation));
            }
            ItemType::Splitter => {
                return Ok(Splitter::new(*cursor, self.tool_rotation));
            }
            ItemType::OreMine => {
                return Ok(OreMine::new(cursor.x, cursor.y, self.tool_rotation));
            }
            ItemType::Chest => return Ok(Chest::new(*cursor)),
            ItemType::Furnace => {
                return Ok(Furnace::new(cursor));
            }
            ItemType::ElectricFurnace => {
                return Ok(ElectricFurnace::new(cursor));
            }
            ItemType::Assembler => {
                return Ok(Assembler::new(cursor));
            }
            ItemType::Boiler => return Ok(Boiler::new(cursor)),
            ItemType::WaterWell => return Ok(WaterWell::new(*cursor)),
            ItemType::OffshorePump => return Ok(OffshorePump::new(cursor)),
            ItemType::Pipe => return Ok(Pipe::new(*cursor)),
            ItemType::SteamEngine => return Ok(SteamEngine::new(*cursor)),
            ItemType::ElectPole => return Ok(ElectPole::new(*cursor)),
            ItemType::UndergroundBelt => {
                return Ok(UndergroundBelt::new(
                    *cursor,
                    self.tool_rotation,
                    UnderDirection::ToGround,
                ))
            }
            ItemType::UndergroundPipe => {
                return Ok(UndergroundPipe::new(*cursor, self.tool_rotation))
            }
            _ => return js_err!("Can't make a structure from {:?}", tool),
        }
    }

    /// Destructively converts serde_json::Value into a Box<dyn Structure>.
    fn structure_from_json(value: &mut serde_json::Value) -> Result<StructureBundle, JsValue> {
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

        let payload: Box<dyn Structure> = match item_type {
            ItemType::TransportBelt => {
                Box::new(map_err(serde_json::from_value::<TransportBelt>(payload))?)
            }
            ItemType::Inserter => Box::new(map_err(serde_json::from_value::<Inserter>(payload))?),
            ItemType::Splitter => Box::new(map_err(serde_json::from_value::<Splitter>(payload))?),
            ItemType::OreMine => Box::new(map_err(serde_json::from_value::<OreMine>(payload))?),
            ItemType::Chest => Box::new(map_err(serde_json::from_value::<Chest>(payload))?),
            ItemType::Furnace => Box::new(map_err(serde_json::from_value::<Furnace>(payload))?),
            ItemType::ElectricFurnace => {
                Box::new(map_err(serde_json::from_value::<ElectricFurnace>(payload))?)
            }
            ItemType::Assembler => Box::new(map_err(serde_json::from_value::<Assembler>(payload))?),
            ItemType::Boiler => Box::new(map_err(serde_json::from_value::<Boiler>(payload))?),
            ItemType::WaterWell => Box::new(map_err(serde_json::from_value::<WaterWell>(payload))?),
            ItemType::OffshorePump => {
                Box::new(map_err(serde_json::from_value::<OffshorePump>(payload))?)
            }
            ItemType::Pipe => Box::new(map_err(serde_json::from_value::<Pipe>(payload))?),
            ItemType::UndergroundPipe => {
                Box::new(map_err(serde_json::from_value::<UndergroundPipe>(payload))?)
            }
            ItemType::SteamEngine => {
                Box::new(map_err(serde_json::from_value::<SteamEngine>(payload))?)
            }
            ItemType::ElectPole => Box::new(map_err(serde_json::from_value::<ElectPole>(payload))?),
            ItemType::UndergroundBelt => {
                Box::new(map_err(serde_json::from_value::<UndergroundBelt>(payload))?)
            }
            _ => return js_err!("Can't make a structure from {:?}", type_str),
        };

        Ok(StructureBundle {
            dynamic: payload,
            components: StructureComponents {
                position: if let Some(position) = value.get_mut("position") {
                    serde_json::from_value(position.take())
                        .map_err(|s| js_str!("structure deserialization error: {}", s))?
                } else {
                    None
                },
                rotation: if let Some(rotation) = value.get_mut("rotation") {
                    serde_json::from_value(rotation.take())
                        .map_err(|s| js_str!("structure rotation deserialization error: {}", s))?
                } else {
                    None
                },
                burner: if let Some(burner) = value.get_mut("burner") {
                    serde_json::from_value(burner.take())
                        .map_err(|s| js_str!("structure deserialization error: {}", s))?
                } else {
                    None
                },
                energy: if let Some(energy) = value.get_mut("energy") {
                    serde_json::from_value(energy.take())
                        .map_err(|s| js_str!("structure energy deserialization error: {}", s))?
                } else {
                    None
                },
                factory: if let Some(factory) = value.get_mut("factory") {
                    serde_json::from_value(factory.take())
                        .map_err(|s| js_str!("structure factory deserialization error: {}", s))?
                } else {
                    None
                },
                fluid_boxes: if let Some(serde_json::value::Value::Array(fluid_boxes)) =
                    value.get_mut("fluid_boxes")
                {
                    fluid_boxes
                        .into_iter()
                        .map(|fb| {
                            serde_json::from_value(fb.take()).map_err(|s| {
                                js_str!("structure factory deserialization error: {}", s)
                            })
                        })
                        .collect::<Result<Vec<_>, JsValue>>()?
                } else {
                    vec![]
                },
            },
        })
    }

    pub fn mouse_down(&mut self, pos: &[f64], button: i32) -> Result<JsValue, JsValue> {
        if pos.len() < 2 {
            return Err(JsValue::from_str("position must have 2 elements"));
        }
        let cursor = Position {
            x: (pos[0] / self.viewport.scale / TILE_SIZE - self.viewport.x).floor() as i32,
            y: (pos[1] / self.viewport.scale / TILE_SIZE - self.viewport.y).floor() as i32,
        };

        console_log!("mouse_down: {}, {}, button: {}", cursor.x, cursor.y, button);
        if button == 2
            && self.find_structure_tile(&[cursor.x, cursor.y]).is_none()
            // Let the player pick up drop items before harvesting ore below.
            && !drop_item_iter(&self.drop_items).any(|item| {
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
            x: (pos[0] / self.viewport.scale / TILE_SIZE - self.viewport.x).floor() as i32,
            y: (pos[1] / self.viewport.scale / TILE_SIZE - self.viewport.y).floor() as i32,
        };
        let mut events = vec![];

        if button == 0 {
            if let Some((selected_tool, _)) = self.get_selected_tool_or_item_opt() {
                let cell = self.tile_at(&cursor);
                if let Some((count, cell)) =
                    self.player.inventory.get(&selected_tool).zip(cell.as_ref())
                {
                    if 1 <= *count && cell.water ^ (selected_tool != ItemType::OffshorePump) {
                        let mut new_s = if let Ok(s) = self.new_structure(&selected_tool, &cursor) {
                            s
                        } else {
                            return Ok(JsValue::UNDEFINED);
                        };
                        if let Some(bbox) = new_s.dynamic.bounding_box(&new_s.components) {
                            for y in bbox.y0..bbox.y1 {
                                for x in bbox.x0..bbox.x1 {
                                    self.harvest(&Position { x, y }, !new_s.dynamic.movable())?;
                                }
                            }
                        }
                        // for structure in &mut self.structures {
                        //     structure
                        //         .dynamic
                        //         .on_construction(new_s.dynamic.as_mut(), true)?;
                        // }
                        // let new_dyn = new_s.dynamic.as_ref();
                        // let structures = std::mem::take(&mut self.structures);
                        // for structure_bundle in &structures {
                        //     let structure = structure_bundle.dynamic.as_ref();
                        //     let position = try_continue!(&structure_bundle.components.position);
                        //     if (new_dyn.power_sink() && structure.power_source()
                        //         || new_dyn.power_source() && structure.power_sink())
                        //         && cursor.distance(position)
                        //             <= new_dyn.wire_reach().min(structure.wire_reach()) as i32
                        //     {
                        //         self.add_power_wire(PowerWire(cursor, *position))?;
                        //     }
                        // }
                        // self.structures = structures;
                        // new_s
                        //     .dynamic
                        //     .on_construction_self(&Ref(&self.structures), true)?;

                        // let connections = new_s.dynamic.connection(
                        //     &new_s.components,
                        //     self,
                        //     &Ref(&self.structures),
                        // );
                        // console_log!(
                        //     "Connection recalculated for self {:?}: {:?}",
                        //     new_s.components.position,
                        //     connections
                        // );
                        // for fbox in &mut new_s.components.fluid_boxes {
                        //     fbox.connect_to = connections;
                        // }

                        // self.structures.push(new_s);
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
                            .find(|(_, s)| s.bundle.is_none())
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
                                    s.bundle.as_ref()?,
                                ))
                            })
                        {
                            let (new_pos, other_pos) = if let Some(pos) =
                                new_s.components.position.zip(structure.components.position)
                            {
                                pos
                            } else {
                                continue;
                            };
                            if (new_s.dynamic.power_sink() && structure.dynamic.power_source()
                                || new_s.dynamic.power_source() && structure.dynamic.power_sink())
                                && new_pos.distance(&other_pos)
                                    <= new_s
                                        .dynamic
                                        .wire_reach()
                                        .min(structure.dynamic.wire_reach())
                                        as i32
                            {
                                let new_power_wire = PowerWire(id, other_id);
                                if self.power_wires.iter().any(|p| *p == new_power_wire) {
                                    continue;
                                }
                                console_log!("power_wires: {}", self.power_wires.len());
                                self.power_wires.push(new_power_wire);
                            }
                        }

                        new_s.dynamic.on_construction_self(
                            id,
                            &mut new_s.components,
                            &StructureDynIter::new_all(&mut self.structures),
                            true,
                        )?;

                        // Notify structures after a slot has been decided
                        let mut structures = std::mem::take(&mut self.structures);
                        for i in 0..structures.len() {
                            let (structure, others) = StructureDynIter::new(&mut structures, i)?;
                            if let Some(s) = structure.bundle.as_mut() {
                                match s.dynamic.on_construction(
                                    &mut s.components,
                                    id,
                                    &new_s,
                                    &others,
                                    true,
                                ) {
                                    Ok(()) => (),
                                    Err(s) => {
                                        drop(others);
                                        self.structures = structures;
                                        return Err(s);
                                    }
                                }
                            }
                        }
                        self.structures = structures;

                        if id.id < self.structures.len() as u32 {
                            self.structures[id.id as usize].bundle = Some(new_s);

                            console_log!(
                                "Inserted to an empty slot: {}/{}, id: {:?}",
                                self.structures
                                    .iter()
                                    .filter(|s| s.bundle.is_none())
                                    .count(),
                                self.structures.len(),
                                id
                            );
                        } else {
                            self.structures.push(StructureEntry {
                                gen: 0,
                                bundle: Some(new_s),
                            });
                            console_log!(
                                "Pushed to the end: {}/{}",
                                self.structures
                                    .iter()
                                    .filter(|s| s.bundle.is_none())
                                    .count(),
                                self.structures.len()
                            );
                        }

                        self.power_networks = build_power_networks(
                            &StructureDynIter::new_all(&mut self.structures),
                            &self.power_wires,
                        );

                        self.update_fluid_connections(&cursor)?;

                        let mut chunks = std::mem::take(&mut self.board);
                        self.render_minimap_data_pixel(&mut chunks, &cursor);
                        self.board = chunks;

                        if let Some(count) = self.player.inventory.get_mut(&selected_tool) {
                            *count -= 1;
                        }

                        self.on_player_update
                            .call1(&window(), &JsValue::from(self.get_player_inventory()?))
                            .unwrap_or_else(|_| JsValue::from(true));
                        events.push(JsValue::from_serde(&JSEvent::UpdatePlayerInventory).unwrap());
                    }
                }
            } else if let Some(structure) = self.find_structure_tile(&[cursor.x, cursor.y]) {
                if structure.dynamic.inventory(InventoryType::Input).is_some()
                    || structure.dynamic.inventory(InventoryType::Output).is_some()
                    || structure
                        .dynamic
                        .inventory(InventoryType::Storage)
                        .is_some()
                    || structure.dynamic.inventory(InventoryType::Burner).is_some()
                    || structure.components.burner.is_some()
                    || structure.components.factory.is_some()
                {
                    // Select clicked structure
                    console_log!("opening inventory at {:?}", cursor);
                    if let Ok(recipe_enable) = self.open_structure_inventory(cursor.x, cursor.y) {
                        // self.on_show_inventory.call0(&window()).unwrap();
                        events.push(
                            JsValue::from_serde(&JSEvent::ShowInventoryAt {
                                pos: (cursor.x, cursor.y),
                                recipe_enable,
                            })
                            .unwrap(),
                        );
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
                events.push(JsValue::from_serde(&JSEvent::UpdatePlayerInventory).unwrap());
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
            (pos[0] / self.viewport.scale / TILE_SIZE - self.viewport.x).floor() as i32,
            (pos[1] / self.viewport.scale / TILE_SIZE - self.viewport.y).floor() as i32,
        ];
        if let Some(bounds) = self.bounds.as_ref() {
            if cursor[0] < 0
                || bounds.width as i32 <= cursor[0]
                || cursor[1] < 0
                || bounds.height as i32 <= cursor[1]
            {
                // return Err(js_str!("invalid mouse position: {:?}", cursor));
                // This is not particularly an error. Just ignore it.
                return Ok(());
            }
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
            (self.viewport.scale * base).min(8.)
        } else {
            (self.viewport.scale / base).max(0.5)
        };
        self.viewport.x +=
            (x as f64 / self.viewport.scale / 32.) * (1. - new_scale / self.viewport.scale);
        self.viewport.y +=
            (y as f64 / self.viewport.scale / 32.) * (1. - new_scale / self.viewport.scale);
        self.viewport.scale = new_scale;

        self.gen_chunks_in_viewport();

        Ok(())
    }

    /// Keyboard event handler. Returns true if re-rendering is necessary to update internal state.
    pub fn on_key_down(&mut self, key_code: i32) -> Result<JsValue, JsValue> {
        match key_code {
            // 'r'
            82 => match self.rotate() {
                Ok(b) => Ok(JsValue::from_bool(b)),
                // If the target structure is not found or uncapable of rotation, it's not a critical error.
                Err(RotateErr::NotFound) | Err(RotateErr::NotSupported) => {
                    Ok(JsValue::from_bool(false))
                }
                Err(RotateErr::Other(err)) => return js_err!("Rotate failed: {:?}", err),
            },
            // Detect keys through '0'..'9', that's a shame char literal cannot be used in place of i32
            code @ 48..=58 => {
                self.select_tool((code - '0' as i32 + 9) % 10)?;
                Ok(JsValue::from_bool(true))
            }
            37 => {
                // Left
                self.viewport.x = (self.viewport.x + 1.).min(0.);
                Ok(JsValue::from_bool(true))
            }
            38 => {
                // Up
                self.viewport.y = (self.viewport.y + 1.).min(0.);
                Ok(JsValue::from_bool(true))
            }
            39 => {
                // Right
                self.viewport.x = (self.viewport.x - 1.).max(-(self.width as f64));
                Ok(JsValue::from_bool(true))
            }
            40 => {
                // Down
                self.viewport.y = (self.viewport.y - 1.).max(-(self.height as f64));
                Ok(JsValue::from_bool(true))
            }
            69 => {
                //'e'
                if self.selected_structure_inventory.is_some() {
                    self.selected_structure_inventory = None;
                }
                Ok(
                    js_sys::Array::of1(&JsValue::from_serde(&JSEvent::ShowInventory).unwrap())
                        .into(),
                )
            }
            81 => {
                // 'q'
                if self.selected_item.is_some() {
                    self.selected_item = None;
                } else if let Some(cursor) = self.cursor {
                    if let Some(structure) = self
                        .find_structure_tile(&cursor)
                        .and_then(|s| str_to_item(s.dynamic.name()))
                    {
                        let count = self.player.inventory.count_item(&structure);
                        self.selected_item = if count > 0 {
                            Some(SelectedItem::PlayerInventory(
                                structure,
                                count.min(STACK_SIZE),
                            ))
                        } else {
                            None
                        };
                        console_log!("q: selected_tool is {:?}", self.selected_item);
                    }
                }
                Ok(JsValue::from_bool(true))
            }
            _ => {
                console_log!("unrecognized key: {}", key_code);
                Ok(JsValue::from_bool(false))
            }
        }
    }

    fn color_of_cell(cell: &Cell) -> [u8; 3] {
        if cell.water {
            [0x00, 0x00, 0xff]
        } else {
            match cell.ore {
                Some(OreValue(Ore::Iron, _)) => [0x3f, 0xaf, 0xff],
                Some(OreValue(Ore::Coal, _)) => [0x1f, 0x1f, 0x1f],
                Some(OreValue(Ore::Copper, _)) => [0x7f, 0x3f, 0x00],
                Some(OreValue(Ore::Stone, _)) => [0x5f, 0x5f, 0x5f],
                _ => [0xaf, 0x7f, 0x3f],
            }
        }
    }

    pub fn reset_viewport(&mut self, canvas: HtmlCanvasElement) {
        self.viewport_width = canvas.width() as f64;
        self.viewport_height = canvas.height() as f64;
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
        self.image_belt = Some(load_image("transport")?);
        self.image_underground_belt = Some(load_image("undergroundBelt")?);
        self.image_chest = Some(load_image("chest")?);
        self.image_mine = Some(load_image("mine")?);
        self.image_furnace = Some(load_image("furnace")?);
        self.image_electric_furnace = Some(load_image("electricFurnace")?);
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
        self.image_stone_ore = Some(load_image("stoneOre")?);
        self.image_iron_plate = Some(load_image("ironPlate")?);
        self.image_copper_plate = Some(load_image("copperPlate")?);
        self.image_gear = Some(load_image("gear")?);
        self.image_copper_wire = Some(load_image("copperWire")?);
        self.image_circuit = Some(load_image("circuit")?);
        self.image_steel_plate = Some(load_image("steelPlate")?);
        self.image_time = Some(load_image("time")?);
        self.image_underground_belt_item = Some(load_image("undergroundBeltItem")?);
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
        if let Some(SelectedItem::PlayerInventory(selected_item, _)) = self.selected_item {
            JsValue::from_f64(*self.player.inventory.get(&selected_item).unwrap_or(&0) as f64)
        } else {
            JsValue::null()
        }
    }

    /// Returns a selected item from either player inventory, toolbelt or a structure inventory.
    /// Returns null if none is selected.
    pub fn get_selected_tool_or_item(&self) -> JsValue {
        if let Some((item, count)) = self.get_selected_tool_or_item_opt() {
            js_sys::Array::of2(
                &JsValue::from_str(&item_to_str(&item)),
                &JsValue::from_f64(count as f64),
            )
            .into()
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
            if Some(SelectedItem::ToolBelt(tool_index)) == self.selected_item {
                context.set_fill_style(&js_str!("#00ffff"));
                context.fill_rect(0., 0., 32., 32.);
            }
            let mut tool = self.new_structure(item, &Position { x: 0, y: 0 })?;
            tool.set_rotation(&self.tool_rotation).ok();
            for depth in 0..3 {
                tool.dynamic
                    .draw(&tool.components, self, context, depth, true)?;
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

    /// Returns a selected item from either player inventory, toolbelt or a structure inventory.
    fn get_selected_tool_or_item_opt(&self) -> Option<(ItemType, usize)> {
        match self.selected_item {
            Some(SelectedItem::ToolBelt(tool)) => Some((self.tool_belt[tool]?, 1)),
            Some(SelectedItem::PlayerInventory(item, count)) => Some((item, count)),
            Some(SelectedItem::StructInventory(id, inventory_type, item, count)) => self
                .find_structure_by_id(id)
                .and_then(|s| s.inventory(inventory_type))
                .and_then(|inventory| inventory.get(&item))
                .and(Some((item, count))),
            None => None,
        }
    }

    pub fn get_selected_item_type(&self) -> JsValue {
        self.selected_item
            .and_then(|i| JsValue::from_serde(&i).ok())
            .unwrap_or(JsValue::NULL)
    }

    /// Returns either "Input", "Output" or "Burner" for the selected inventory
    pub fn get_selected_inventory_type(&self) -> JsValue {
        if let Some(SelectedItem::StructInventory(_, type_, _, _)) = self.selected_item {
            JsValue::from_serde(&type_).unwrap_or(JsValue::NULL)
        } else {
            JsValue::NULL
        }
    }

    /// Attempts to select or set a tool if the player is holding an item
    ///
    /// @param tool the index of the tool item, [0,9]
    /// @returns whether the tool bar item should be re-rendered
    pub fn select_tool(&mut self, tool: i32) -> Result<JsValue, JsValue> {
        if let Some(SelectedItem::PlayerInventory(item, _)) = self.selected_item {
            // We allow only items in tool_defs to present on the tool belt
            // This behavior is different from Factorio, maybe we can allow it
            if tool_defs.iter().any(|i| i.item_type == item) {
                self.tool_belt[tool as usize] = Some(item);
                // Deselect the item for the player to let him select from tool belt.
                self.selected_item = None;
                return Ok(JsValue::from_bool(true));
            } else {
                console_log!(
                    "select_tool could not find tool_def with item type: {:?}",
                    item
                );
                return Ok(JsValue::from_bool(false));
            }
        }
        self.selected_item =
            if 0 <= tool && Some(SelectedItem::ToolBelt(tool as usize)) != self.selected_item {
                Some(SelectedItem::ToolBelt(tool as usize))
            } else {
                None
            };
        if let Some(SelectedItem::ToolBelt(sel)) = self.selected_item {
            if self.tool_belt[sel].is_none() {
                return Ok(JsValue::from_serde(&JSEvent::ShowInventory).unwrap());
            }
        }
        Ok(JsValue::from_bool(self.selected_item.is_some()))
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
            self.viewport_width / self.viewport.scale,
            self.viewport_height / self.viewport.scale,
        )
    }

    pub fn get_viewport_scale(&self) -> f64 {
        self.viewport.scale
    }

    pub fn set_viewport_pos(&mut self, x: f64, y: f64) -> Result<js_sys::Array, JsValue> {
        let viewport = self.get_viewport();
        self.viewport.x = -(x - viewport.0 / TILE_SIZE / 2.)
            .max(0.)
            .min(self.width as f64 - viewport.0 / TILE_SIZE - 1.);
        self.viewport.y = -(y - viewport.1 / TILE_SIZE / 2.)
            .max(0.)
            .min(self.height as f64 - viewport.1 / TILE_SIZE - 1.);

        self.gen_chunks_in_viewport();

        Ok(js_sys::Array::of2(
            &JsValue::from_f64(viewport.0),
            &JsValue::from_f64(viewport.1),
        ))
    }

    /// Returns a mutable iterator over valid structures
    #[allow(dead_code)]
    fn structure_iter_mut(&mut self) -> impl Iterator<Item = &mut StructureBundle> {
        self.structures.iter_mut().filter_map(|s| s.bundle.as_mut())
    }

    /// * `scale_relative` -  If true, the delta is divided by current view zoom factor.
    ///   Intended for use with the minimap, which does not change scale.
    pub fn delta_viewport_pos(
        &mut self,
        x: f64,
        y: f64,
        scale_relative: bool,
    ) -> Result<(), JsValue> {
        if scale_relative {
            self.viewport.x += x / self.viewport.scale / TILE_SIZE;
            self.viewport.y += y / self.viewport.scale / TILE_SIZE;
        } else {
            self.viewport.x += x / TILE_SIZE;
            self.viewport.y += y / TILE_SIZE;
        }

        self.gen_chunks_in_viewport();

        Ok(())
    }

    fn gen_chunks_in_viewport(&mut self) {
        let (left, top, right, bottom) = apply_bounds(
            &self.bounds,
            &self.viewport,
            self.viewport_width,
            self.viewport_height,
        );
        for cx in left.div_euclid(CHUNK_SIZE_I)..=right.div_euclid(CHUNK_SIZE_I) {
            for cy in top.div_euclid(CHUNK_SIZE_I)..=bottom.div_euclid(CHUNK_SIZE_I) {
                let chunk_pos = Position::new(cx, cy);
                if !self.board.contains_key(&chunk_pos) {
                    console_log!(
                        "Generating chunk_pos {:?}, {} chunks total",
                        chunk_pos,
                        self.board.len()
                    );
                    let mut chunk = gen_chunk(chunk_pos, &self.terrain_params);
                    calculate_back_image(&self.board, &chunk_pos, &mut chunk.cells);
                    self.render_minimap_chunk(&chunk_pos, &mut chunk);
                    self.board.insert(chunk_pos, chunk);
                }
            }
        }
    }

    /// Add a new popup text that will show for a moment and automatically disappears
    ///
    /// It delegates actual HTML element creation to JavaScript code via a callback
    /// since it is too cumbersome to handle HTML elements directly in Rust code.
    /// While it's possible, it can be slower than JavaScript code due to many runtime
    /// calls.
    ///
    /// Also, we don't want to render text in WebGL directly. We know how painful it is to
    /// support text rendering in raw WebGL...
    fn new_popup_text(&mut self, text: String, x: f64, y: f64) -> Result<(), JsValue> {
        self.on_popup_text.call3(
            &window(),
            &JsValue::from_str(&text),
            &JsValue::from_f64((x + self.viewport.x * TILE_SIZE) * self.viewport.scale),
            &JsValue::from_f64((y + self.viewport.y * TILE_SIZE) * self.viewport.scale),
        )?;
        Ok(())
    }

    /// Returns an iterator over valid structures
    fn structure_iter(&self) -> impl Iterator<Item = &StructureBundle> {
        self.structures.iter().filter_map(|s| s.bundle.as_ref())
    }

    /// Returns an iterator over valid structures and their ids
    fn structure_id_iter(&self) -> impl Iterator<Item = (StructureId, &StructureBundle)> {
        self.structures.iter().enumerate().filter_map(|(id, s)| {
            Some((
                StructureId {
                    id: id as u32,
                    gen: s.gen,
                },
                s.bundle.as_ref()?,
            ))
        })
    }
}
