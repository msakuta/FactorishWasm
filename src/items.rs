use super::{
    structure::StructureBundle, tilesize, FactorishState, ImageBundle, ItemResponse, Position,
    DROP_ITEM_SIZE_I, TILE_SIZE_I,
};
use serde::{Deserialize, Serialize};
use specs::{Component, Entities, Entity, ReadStorage, System, VecStorage, WriteStorage};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash, Serialize, Deserialize)]
pub(crate) enum ItemType {
    IronOre,
    CoalOre,
    CopperOre,
    IronPlate,
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
        ItemType::Pipe => &state.image_pipe.as_ref().unwrap().url,
        ItemType::SteamEngine => &state.image_steam_engine.as_ref().unwrap().url,
        ItemType::ElectPole => &state.image_elect_pole.as_ref().unwrap().url,
        ItemType::Splitter => &state.image_splitter.as_ref().unwrap().url,
    }
}

#[derive(Debug)]
pub(crate) struct ItemPosition {
    pub x: i32,
    pub y: i32,
}

impl Component for ItemPosition {
    type Storage = VecStorage<Self>;
}

pub(crate) struct UpdatePos<'a> {
    pub structures: &'a mut Vec<StructureBundle>,
}

impl<'a, 'b> System<'a> for UpdatePos<'b> {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, ItemType>,
        WriteStorage<'a, ItemPosition>,
    );

    fn run(&mut self, (entities, item_type, mut pos): Self::SystemData) {
        use specs::Join;
        let mut to_remove = vec![];
        for (entity, item_type, pos) in (&entities, &item_type, &mut pos).join() {
            if let Some(item_response_result) = self
                .structures
                .iter_mut()
                .find(|s| {
                    s.dynamic.contains(
                        &s.components,
                        &Position {
                            x: pos.x / TILE_SIZE_I,
                            y: pos.y / TILE_SIZE_I,
                        },
                    )
                })
                .and_then(|structure| {
                    structure
                        .dynamic
                        .item_response(
                            &mut structure.components,
                            &DropItem {
                                id: entity.id(),
                                type_: *item_type,
                                x: pos.x,
                                y: pos.y,
                            },
                        )
                        .ok()
                })
            {
                match item_response_result.0 {
                    ItemResponse::None => {}
                    ItemResponse::Move(moved_x, moved_y) => {
                        // if self.state.hit_check(moved_x, moved_y, Some(item.id)) {
                        //     continue;
                        // }
                        let position = Position {
                            x: moved_x / 32,
                            y: moved_y / 32,
                        };
                        if let Some(s) = self
                            .structures
                            .iter()
                            .find(|s| s.dynamic.contains(&s.components, &position))
                        {
                            if !s.dynamic.movable() {
                                continue;
                            }
                        } else {
                            continue;
                        }
                        pos.x = moved_x;
                        pos.y = moved_y;
                    }
                    ItemResponse::Consume => {
                        to_remove.push(entity);
                    }
                }
            }
        }
        for entity in to_remove {
            entities.delete(entity).unwrap();
        }
    }
}

pub(crate) struct RenderItem<'a> {
    state: &'a FactorishState,
    context: &'a CanvasRenderingContext2d,
}

impl<'a> RenderItem<'a> {
    pub fn new<'b>(
        state: &'b FactorishState,
        context: &'b CanvasRenderingContext2d,
    ) -> RenderItem<'b> {
        RenderItem { state, context }
    }
}

impl<'a, 'b> System<'a> for RenderItem<'b> {
    type SystemData = (ReadStorage<'a, ItemPosition>, ReadStorage<'a, ItemType>);

    fn run(&mut self, (pos, item_type): Self::SystemData) {
        use specs::Join;
        for (pos, item_type) in (&pos, &item_type).join() {
            render_drop_item(self.state, self.context, item_type, pos.x, pos.y).unwrap();
        }
    }
}

pub(crate) struct SerializeItem {
    pub output: Result<Vec<serde_json::Value>, JsValue>,
}

impl<'a> System<'a> for SerializeItem {
    type SystemData = (
        Entities<'a>,
        ReadStorage<'a, ItemType>,
        ReadStorage<'a, ItemPosition>,
    );

    fn run(&mut self, (entities, item_type, pos): Self::SystemData) {
        use specs::Join;
        self.output = (&entities, &item_type, &pos)
            .join()
            .into_iter()
            .map(|(entity, item_type, pos)| {
                serde_json::to_value(DropItem {
                    id: entity.id(),
                    type_: *item_type,
                    x: pos.x,
                    y: pos.y,
                })
            })
            .collect::<serde_json::Result<Vec<serde_json::Value>>>()
            .map_err(|e| js_str!("Serialize error: {}", e));
    }
}

pub(crate) struct DeleteAllItems;

impl<'a> System<'a> for DeleteAllItems {
    type SystemData = Entities<'a>;

    fn run(&mut self, entities: Self::SystemData) {
        use specs::Join;
        for entity in entities.join() {
            entities.delete(entity).unwrap();
        }
    }
}

pub(crate) struct HitCheck {
    x: i32,
    y: i32,
    ignore: Option<Entity>,
    result: bool,
}

impl HitCheck {
    pub fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            ignore: None,
            result: false,
        }
    }

    pub fn result(&self) -> bool {
        self.result
    }
}

impl<'a> System<'a> for HitCheck {
    type SystemData = (Entities<'a>, ReadStorage<'a, ItemPosition>);

    /// Check whether given coordinates hits some object
    fn run(&mut self, (entities, pos): Self::SystemData) {
        use specs::Join;
        self.result = false;
        for (entity, pos) in (&entities, &pos).join() {
            if let Some(ignore_id) = self.ignore {
                if ignore_id == entity {
                    continue;
                }
            }
            if (self.x - pos.x).abs() < DROP_ITEM_SIZE_I
                && (self.y - pos.y).abs() < DROP_ITEM_SIZE_I
            {
                self.result = true;
                return;
            }
        }
    }
}
