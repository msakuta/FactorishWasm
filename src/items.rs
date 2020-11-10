use super::{tilesize, FactorishState};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub(crate) enum ItemType {
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

pub(crate) fn item_to_str(type_: &ItemType) -> String {
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

pub(crate) fn str_to_item(name: &str) -> Option<ItemType> {
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
    let img = match item_type {
        ItemType::IronOre => &state.image_iron_ore,
        ItemType::CoalOre => &state.image_coal_ore,
        ItemType::CopperOre => &state.image_copper_ore,
        ItemType::IronPlate => &state.image_iron_plate,
        ItemType::CopperPlate => &state.image_copper_plate,

        ItemType::TransportBelt => &state.image_belt,
        ItemType::Chest => &state.image_chest,
        ItemType::Inserter => &state.image_inserter,
        ItemType::OreMine => &state.image_mine,
        ItemType::Furnace => &state.image_furnace,
    };
    if let Some(ref image) = img {
        context.draw_image_with_image_bitmap(image, x as f64 - 8., y as f64 - 8.)?;
    }
    Ok(())
}
