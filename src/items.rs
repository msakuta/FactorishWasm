use crate::{gl::utils::enable_buffer, Vector2f};

use super::{gl::utils::Flatten, FactorishState, ImageBundle, TILE_SIZE_F};
use cgmath::{Matrix3, Matrix4, One, Vector2, Vector3};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, WebGlRenderingContext as GL, WebGlTexture};

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, Serialize, Deserialize, PartialOrd, Ord)]
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
    SteelPlate,
    SciencePack1,
    SciencePack2,

    TransportBelt,
    Chest,
    Inserter,
    OreMine,
    Furnace,
    ElectricFurnace,
    Assembler,
    Lab,
    Boiler,
    WaterWell,
    OffshorePump,
    Pipe,
    UndergroundPipe,
    SteamEngine,
    ElectPole,
    Splitter,
    UndergroundBelt,
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
        ItemType::SteelPlate => "Steel Plate".to_string(),
        ItemType::SciencePack1 => "Science Pack 1".to_string(),
        ItemType::SciencePack2 => "Science Pack 2".to_string(),

        ItemType::TransportBelt => "Transport Belt".to_string(),
        ItemType::Chest => "Chest".to_string(),
        ItemType::Inserter => "Inserter".to_string(),
        ItemType::OreMine => "Ore Mine".to_string(),
        ItemType::Furnace => "Furnace".to_string(),
        ItemType::ElectricFurnace => "Electric Furnace".to_string(),
        ItemType::Assembler => "Assembler".to_string(),
        ItemType::Lab => "Lab".to_string(),
        ItemType::Boiler => "Boiler".to_string(),
        ItemType::WaterWell => "Water Well".to_string(),
        ItemType::OffshorePump => "Offshore Pump".to_string(),
        ItemType::Pipe => "Pipe".to_string(),
        ItemType::UndergroundPipe => "Underground Pipe".to_string(),
        ItemType::SteamEngine => "Steam Engine".to_string(),
        ItemType::ElectPole => "Electric Pole".to_string(),
        ItemType::Splitter => "Splitter".to_string(),
        ItemType::UndergroundBelt => "Underground Belt".to_string(),
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
        "Steel Plate" => Some(ItemType::SteelPlate),
        "Science Pack 1" => Some(ItemType::SciencePack1),
        "Science Pack 2" => Some(ItemType::SciencePack2),

        "Transport Belt" => Some(ItemType::TransportBelt),
        "Chest" => Some(ItemType::Chest),
        "Inserter" => Some(ItemType::Inserter),
        "Ore Mine" => Some(ItemType::OreMine),
        "Furnace" => Some(ItemType::Furnace),
        "Electric Furnace" => Some(ItemType::ElectricFurnace),
        "Assembler" => Some(ItemType::Assembler),
        "Lab" => Some(ItemType::Lab),
        "Boiler" => Some(ItemType::Boiler),
        "Water Well" => Some(ItemType::WaterWell),
        "Offshore Pump" => Some(ItemType::OffshorePump),
        "Pipe" => Some(ItemType::Pipe),
        "Underground Pipe" => Some(ItemType::UndergroundPipe),
        "Steam Engine" => Some(ItemType::SteamEngine),
        "Electric Pole" => Some(ItemType::ElectPole),
        "Splitter" => Some(ItemType::Splitter),
        "Underground Belt" => Some(ItemType::UndergroundBelt),

        _ => None,
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
        ItemType::SteelPlate => render16(&state.image_steel_plate),
        ItemType::SciencePack1 => render16(&state.image_science_pack_1),
        ItemType::SciencePack2 => render16(&state.image_science_pack_2),

        ItemType::TransportBelt => render16(&state.image_belt),
        ItemType::Chest => render16(&state.image_chest),
        ItemType::Inserter => render_animated32(&state.image_inserter),
        ItemType::OreMine => render16(&state.image_mine),
        ItemType::Furnace => render_animated32(&state.image_furnace),
        ItemType::ElectricFurnace => render_animated32(&state.image_electric_furnace),
        ItemType::Assembler => render16(&state.image_assembler),
        ItemType::Lab => render16(&state.image_lab),
        ItemType::Boiler => render16(&state.image_boiler),
        ItemType::WaterWell => render16(&state.image_water_well),
        ItemType::OffshorePump => render16(&state.image_offshore_pump),
        ItemType::Pipe => render16(&state.image_pipe),
        ItemType::UndergroundPipe => render16(&state.image_pipe),
        ItemType::SteamEngine => render16(&state.image_steam_engine),
        ItemType::ElectPole => render16(&state.image_elect_pole),
        ItemType::Splitter => render16(&state.image_splitter),
        ItemType::UndergroundBelt => render16(&state.image_underground_belt_item),
    }
}

pub(crate) fn render_drop_item_mat_gl(
    state: &FactorishState,
    gl: &GL,
    item_type: &ItemType,
    transform: Matrix4<f32>,
) -> Result<(), JsValue> {
    let shader = state
        .assets
        .textured_shader
        .as_ref()
        .ok_or_else(|| js_str!("Shader not found"))?;
    gl.use_program(Some(&shader.program));
    let render_gen = |img: &WebGlTexture, scale_x: f32| -> Result<(), JsValue> {
        gl.bind_texture(GL::TEXTURE_2D, Some(&img));
        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::from_nonuniform_scale(scale_x, 1.).flatten(),
        );

        gl.uniform_matrix4fv_with_f32_array(
            shader.transform_loc.as_ref(),
            false,
            (transform * Matrix4::from_scale(0.5)).flatten(),
        );

        gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
        Ok(())
    };
    let render16 = |img| render_gen(img, 1.);
    match item_type {
        ItemType::IronOre => render16(&state.assets.tex_iron_ore),
        ItemType::CoalOre => render16(&state.assets.tex_coal_ore),
        ItemType::CopperOre => render16(&state.assets.tex_copper_ore),
        ItemType::StoneOre => render16(&state.assets.tex_stone_ore),
        ItemType::IronPlate => render16(&state.assets.tex_iron_plate),
        ItemType::CopperPlate => render16(&state.assets.tex_copper_plate),
        ItemType::Gear => render16(&state.assets.tex_gear),
        ItemType::CopperWire => render16(&state.assets.tex_copper_wire),
        ItemType::Circuit => render16(&state.assets.tex_circuit),
        ItemType::SteelPlate => render16(&state.assets.tex_steel_plate),
        ItemType::SciencePack1 => render16(&state.assets.tex_science_pack_1),
        ItemType::SciencePack2 => render16(&state.assets.tex_science_pack_2),

        ItemType::TransportBelt => render16(&state.assets.tex_belt),
        ItemType::Chest => render16(&state.assets.tex_chest),
        ItemType::Inserter => render_gen(&state.assets.tex_inserter, 1. / 2.),
        ItemType::OreMine => render_gen(&state.assets.tex_ore_mine, 1. / 3.),
        ItemType::Furnace => render_gen(&state.assets.tex_furnace, 1. / 3.),
        ItemType::ElectricFurnace => render_gen(&state.assets.tex_electric_furnace, 1. / 3.),
        ItemType::Assembler => render_gen(&state.assets.tex_assembler, 1. / 4.),
        ItemType::Lab => render_gen(&state.assets.tex_lab, 1. / 4.),
        ItemType::Boiler => render_gen(&state.assets.tex_boiler, 1. / 3.),
        ItemType::WaterWell => render16(&state.assets.tex_water_well),
        ItemType::OffshorePump => render16(&state.assets.tex_offshore_pump),
        ItemType::Pipe => render16(&state.assets.tex_pipe),
        ItemType::UndergroundPipe => render16(&state.assets.tex_pipe),
        ItemType::SteamEngine => render_gen(&state.assets.tex_steam_engine, 1. / 3.),
        ItemType::ElectPole => render16(&state.assets.tex_elect_pole),
        ItemType::Splitter => render16(&state.assets.tex_splitter),
        ItemType::UndergroundBelt => render16(&state.assets.tex_underground_belt_item),
    }
}

pub(crate) fn render_drop_item_gl(
    state: &FactorishState,
    gl: &GL,
    item_type: &ItemType,
    x: f64,
    y: f64,
) -> Result<(), JsValue> {
    render_drop_item_mat_gl(
        state,
        gl,
        item_type,
        state.get_world_transform()?
            * Matrix4::from_scale(2.)
            * Matrix4::from_translation(Vector3::new(
                x as f32 / TILE_SIZE_F + state.viewport.x as f32 - 0.25,
                y as f32 / TILE_SIZE_F + state.viewport.y as f32 - 0.25,
                0.,
            )),
    )
}

pub(crate) fn render_item_overlay_gl(
    state: &FactorishState,
    gl: &GL,
    item_type: &ItemType,
    pos: &Vector2f,
) -> Result<(), JsValue> {
    let transform = state.get_world_transform()?
        * Matrix4::from_scale(2.)
        * Matrix4::from_translation(
            (pos + Vector2::new(state.viewport.x as f32, state.viewport.y as f32)
                - Vector2::new(0.25, 0.25))
            .extend(0.),
        );

    let _render_dark_glow = (|img: &WebGlTexture| -> Result<(), JsValue> {
        let shader = state
            .assets
            .textured_alpha_shader
            .as_ref()
            .ok_or_else(|| js_str!("Shader not found"))?;
        enable_buffer(gl, &state.assets.screen_buffer, 2, shader.vertex_position);
        gl.use_program(Some(&shader.program));
        gl.uniform1i(shader.texture_loc.as_ref(), 0);
        gl.bind_texture(GL::TEXTURE_2D, Some(&img));
        gl.uniform1f(shader.alpha_loc.as_ref(), 1.);
        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::one().flatten(),
        );

        gl.uniform_matrix4fv_with_f32_array(
            shader.transform_loc.as_ref(),
            false,
            (transform * Matrix4::from_translation(Vector3::new(-0.25, -0.25, 0.))).flatten(),
        );

        gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
        Ok(())
    })(&state.assets.tex_dark_glow)?;

    render_drop_item_mat_gl(state, gl, item_type, transform)
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
        ItemType::SteelPlate => &state.image_steel_plate.as_ref().unwrap().url,
        ItemType::SciencePack1 => &state.image_science_pack_1.as_ref().unwrap().url,
        ItemType::SciencePack2 => &state.image_science_pack_2.as_ref().unwrap().url,

        ItemType::TransportBelt => &state.image_belt.as_ref().unwrap().url,
        ItemType::Chest => &state.image_chest.as_ref().unwrap().url,
        ItemType::Inserter => &state.image_inserter.as_ref().unwrap().url,
        ItemType::OreMine => &state.image_mine.as_ref().unwrap().url,
        ItemType::Furnace => &state.image_furnace.as_ref().unwrap().url,
        ItemType::ElectricFurnace => &state.image_electric_furnace.as_ref().unwrap().url,
        ItemType::Assembler => &state.image_assembler.as_ref().unwrap().url,
        ItemType::Lab => &state.image_lab.as_ref().unwrap().url,
        ItemType::Boiler => &state.image_boiler.as_ref().unwrap().url,
        ItemType::WaterWell => &state.image_water_well.as_ref().unwrap().url,
        ItemType::OffshorePump => &state.image_offshore_pump.as_ref().unwrap().url,
        ItemType::Pipe => &state.image_pipe.as_ref().unwrap().url,
        ItemType::UndergroundPipe => &state.image_pipe.as_ref().unwrap().url,
        ItemType::SteamEngine => &state.image_steam_engine.as_ref().unwrap().url,
        ItemType::ElectPole => &state.image_elect_pole.as_ref().unwrap().url,
        ItemType::Splitter => &state.image_splitter.as_ref().unwrap().url,
        ItemType::UndergroundBelt => &state.image_underground_belt_item.as_ref().unwrap().url,
    }
}
