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
    Gear,

    TransportBelt,
    Chest,
    Inserter,
    OreMine,
    Furnace,
    Assembler,
}

pub(crate) fn item_to_str(type_: &ItemType) -> String {
    match type_ {
        ItemType::IronOre => "Iron Ore".to_string(),
        ItemType::CoalOre => "Coal Ore".to_string(),
        ItemType::CopperOre => "Copper Ore".to_string(),
        ItemType::IronPlate => "Iron Plate".to_string(),
        ItemType::CopperPlate => "Copper Plate".to_string(),
        ItemType::Gear => "Gear".to_string(),

        ItemType::TransportBelt => "Transport Belt".to_string(),
        ItemType::Chest => "Chest".to_string(),
        ItemType::Inserter => "Inserter".to_string(),
        ItemType::OreMine => "Ore Mine".to_string(),
        ItemType::Furnace => "Furnace".to_string(),
        ItemType::Assembler => "Assembler".to_string(),
    }
}

pub(crate) fn str_to_item(name: &str) -> Option<ItemType> {
    match name {
        "Iron Ore" => Some(ItemType::IronOre),
        "Coal Ore" => Some(ItemType::CoalOre),
        "Copper Ore" => Some(ItemType::CopperOre),
        "Iron Plate" => Some(ItemType::IronPlate),
        "Gear" => Some(ItemType::Gear),

        "Transport Belt" => Some(ItemType::TransportBelt),
        "Chest" => Some(ItemType::Chest),
        "Inserter" => Some(ItemType::Inserter),
        "Ore Mine" => Some(ItemType::OreMine),
        "Furnace" => Some(ItemType::Furnace),
        "Assembler" => Some(ItemType::Assembler),

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
    let render16 = |img: &Option<_>| -> Result<(), JsValue> {
        if let Some(image) = img.as_ref() {
            context.draw_image_with_image_bitmap_and_dw_and_dh(
                image,
                x as f64 - 8.,
                y as f64 - 8.,
                16.,
                16.,
            )?;
        }
        Ok(())
    };
    let render_animated32 = |img: &Option<_>| -> Result<(), JsValue> {
        if let Some(image) = img.as_ref() {
            context.draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                image,
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
        ItemType::IronPlate => render16(&state.image_iron_plate),
        ItemType::CopperPlate => render16(&state.image_copper_plate),
        ItemType::Gear => render16(&state.image_copper_plate),

        ItemType::TransportBelt => render16(&state.image_belt),
        ItemType::Chest => render16(&state.image_chest),
        ItemType::Inserter => render_animated32(&state.image_inserter),
        ItemType::OreMine => render16(&state.image_mine),
        ItemType::Furnace => render_animated32(&state.image_furnace),
        ItemType::Assembler => render16(&state.image_assembler),
    }
}
