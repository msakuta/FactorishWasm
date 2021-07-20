use super::{tilesize, FactorishState, ImageBundle};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, Serialize, Deserialize)]
pub(crate) enum ItemType {
    IronOre,
    CoalOre,
    CopperOre,
    IronPlate,
    StoneOre,
    CopperPlate,
    Gear,
    CopperWire,
    Circuit,

    TransportBelt,
    Chest,
    Inserter,
    OreMine,
    Furnace,
    Assembler,
    Boiler,
    WaterWell,
    OffshorePump,
    Pipe,
    SteamEngine,
    ElectPole,
    Splitter,
}

pub(crate) fn item_to_str(type_: &ItemType) -> String {
    match type_ {
        ItemType::IronOre => "Iron Ore".to_string(),
        ItemType::CoalOre => "Coal Ore".to_string(),
        ItemType::CopperOre => "Copper Ore".to_string(),
        ItemType::StoneOre => "Stone Ore".to_string(),
        ItemType::IronPlate => "Iron Plate".to_string(),
        ItemType::CopperPlate => "Copper Plate".to_string(),
        ItemType::Gear => "Gear".to_string(),
        ItemType::CopperWire => "Copper Wire".to_string(),
        ItemType::Circuit => "Circuit".to_string(),

        ItemType::TransportBelt => "Transport Belt".to_string(),
        ItemType::Chest => "Chest".to_string(),
        ItemType::Inserter => "Inserter".to_string(),
        ItemType::OreMine => "Ore Mine".to_string(),
        ItemType::Furnace => "Furnace".to_string(),
        ItemType::Assembler => "Assembler".to_string(),
        ItemType::Boiler => "Boiler".to_string(),
        ItemType::WaterWell => "Water Well".to_string(),
        ItemType::OffshorePump => "Offshore Pump".to_string(),
        ItemType::Pipe => "Pipe".to_string(),
        ItemType::SteamEngine => "Steam Engine".to_string(),
        ItemType::ElectPole => "Electric Pole".to_string(),
        ItemType::Splitter => "Splitter".to_string(),
    }
}

pub(crate) fn str_to_item(name: &str) -> Option<ItemType> {
    match name {
        "Iron Ore" => Some(ItemType::IronOre),
        "Coal Ore" => Some(ItemType::CoalOre),
        "Copper Ore" => Some(ItemType::CopperOre),
        "Stone Ore" => Some(ItemType::StoneOre),
        "Iron Plate" => Some(ItemType::IronPlate),
        "Copper Plate" => Some(ItemType::CopperPlate),
        "Gear" => Some(ItemType::Gear),
        "Copper Wire" => Some(ItemType::CopperWire),
        "Circuit" => Some(ItemType::Circuit),

        "Transport Belt" => Some(ItemType::TransportBelt),
        "Chest" => Some(ItemType::Chest),
        "Inserter" => Some(ItemType::Inserter),
        "Ore Mine" => Some(ItemType::OreMine),
        "Furnace" => Some(ItemType::Furnace),
        "Assembler" => Some(ItemType::Assembler),
        "Boiler" => Some(ItemType::Boiler),
        "Water Well" => Some(ItemType::WaterWell),
        "Offshore Pump" => Some(ItemType::OffshorePump),
        "Pipe" => Some(ItemType::Pipe),
        "Steam Engine" => Some(ItemType::SteamEngine),
        "Electric Pole" => Some(ItemType::ElectPole),
        "Splitter" => Some(ItemType::Splitter),

        _ => None,
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct DropItem {
    pub id: u32,
    pub type_: ItemType,
    pub x: i32,
    pub y: i32,
}

impl DropItem {
    pub(crate) fn new(serial_no: &mut u32, type_: ItemType, c: i32, r: i32) -> Self {
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

pub(crate) fn render_drop_item(
    state: &FactorishState,
    context: &CanvasRenderingContext2d,
    item_type: &ItemType,
    x: i32,
    y: i32,
) -> Result<(), JsValue> {
    let render16 = |img: &Option<ImageBundle>| -> Result<(), JsValue> {
        if let Some(image) = img.as_ref() {
            context.draw_image_with_image_bitmap_and_dw_and_dh(
                &image.bitmap,
                x as f64 - 8.,
                y as f64 - 8.,
                16.,
                16.,
            )?;
        }
        Ok(())
    };
    let render_animated32 = |img: &Option<ImageBundle>| -> Result<(), JsValue> {
        if let Some(image) = img.as_ref() {
            context.draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                &image.bitmap,
                0.,
                0.,
                32.,
                32.,
                x as f64 - 8.,
                y as f64 - 8.,
                16.,
                16.,
            )?;
        }
        Ok(())
    };
    match item_type {
        ItemType::IronOre => render16(&state.image_iron_ore),
        ItemType::CoalOre => render16(&state.image_coal_ore),
        ItemType::CopperOre => render16(&state.image_copper_ore),
        ItemType::StoneOre => render16(&state.image_stone_ore),
        ItemType::IronPlate => render16(&state.image_iron_plate),
        ItemType::CopperPlate => render16(&state.image_copper_plate),
        ItemType::Gear => render16(&state.image_gear),
        ItemType::CopperWire => render16(&state.image_copper_wire),
        ItemType::Circuit => render16(&state.image_circuit),

        ItemType::TransportBelt => render16(&state.image_belt),
        ItemType::Chest => render16(&state.image_chest),
        ItemType::Inserter => render_animated32(&state.image_inserter),
        ItemType::OreMine => render16(&state.image_mine),
        ItemType::Furnace => render_animated32(&state.image_furnace),
        ItemType::Assembler => render16(&state.image_assembler),
        ItemType::Boiler => render16(&state.image_boiler),
        ItemType::WaterWell => render16(&state.image_water_well),
        ItemType::OffshorePump => render16(&state.image_offshore_pump),
        ItemType::Pipe => render16(&state.image_pipe),
        ItemType::SteamEngine => render16(&state.image_steam_engine),
        ItemType::ElectPole => render16(&state.image_elect_pole),
        ItemType::Splitter => render16(&state.image_splitter),
    }
}

pub(crate) fn get_item_image_url<'a>(state: &'a FactorishState, item_type: &ItemType) -> &'a str {
    match item_type {
        ItemType::IronOre => &state.image_iron_ore.as_ref().unwrap().url,
        ItemType::CoalOre => &state.image_coal_ore.as_ref().unwrap().url,
        ItemType::CopperOre => &state.image_copper_ore.as_ref().unwrap().url,
        ItemType::StoneOre => &state.image_stone_ore.as_ref().unwrap().url,
        ItemType::IronPlate => &state.image_iron_plate.as_ref().unwrap().url,
        ItemType::CopperPlate => &state.image_copper_plate.as_ref().unwrap().url,
        ItemType::Gear => &state.image_gear.as_ref().unwrap().url,
        ItemType::CopperWire => &state.image_copper_wire.as_ref().unwrap().url,
        ItemType::Circuit => &state.image_circuit.as_ref().unwrap().url,

        ItemType::TransportBelt => &state.image_belt.as_ref().unwrap().url,
        ItemType::Chest => &state.image_chest.as_ref().unwrap().url,
        ItemType::Inserter => &state.image_inserter.as_ref().unwrap().url,
        ItemType::OreMine => &state.image_mine.as_ref().unwrap().url,
        ItemType::Furnace => &state.image_furnace.as_ref().unwrap().url,
        ItemType::Assembler => &state.image_assembler.as_ref().unwrap().url,
        ItemType::Boiler => &state.image_boiler.as_ref().unwrap().url,
        ItemType::WaterWell => &state.image_water_well.as_ref().unwrap().url,
        ItemType::OffshorePump => &state.image_offshore_pump.as_ref().unwrap().url,
        ItemType::Pipe => &state.image_pipe.as_ref().unwrap().url,
        ItemType::SteamEngine => &state.image_steam_engine.as_ref().unwrap().url,
        ItemType::ElectPole => &state.image_elect_pole.as_ref().unwrap().url,
        ItemType::Splitter => &state.image_splitter.as_ref().unwrap().url,
    }
}
