use super::{
    assembler::Assembler,
    boiler::Boiler,
    chest::Chest,
    drop_items::{build_index, DropItemEntry},
    elect_pole::ElectPole,
    furnace::Furnace,
    inserter::Inserter,
    items::ItemType,
    ore_mine::OreMine,
    pipe::Pipe,
    power_network::build_power_networks,
    steam_engine::SteamEngine,
    structure::{Structure, StructureBoxed, StructureDynIter, StructureEntry, StructureId},
    terrain::{
        calculate_back_image, gen_terrain, Chunks, ChunksExt, TerrainParameters, CHUNK_SIZE_I,
    },
    transport_belt::TransportBelt,
    water_well::WaterWell,
    FactorishState, InventoryTrait, Position, PowerWire, Rotation,
};
use std::collections::HashSet;
use wasm_bindgen::prelude::*;

fn wrap_structure(s: StructureBoxed) -> StructureEntry {
    StructureEntry {
        gen: 0,
        dynamic: Some(s),
    }
}

/// Avoid having water beneath a structure by filling water cells
fn update_water(
    structures: &[StructureEntry],
    terrain: &mut Chunks,
    _terrain_params: &TerrainParameters,
) {
    let mut to_update = HashSet::new();
    for structure in structures {
        if let Some(dynamic) = structure.dynamic.as_deref() {
            let position = *dynamic.position();
            terrain
                .get_tile_mut(position)
                .map(|cell| cell.water = false);
            to_update.insert(Position::new(
                position.x.div_euclid(CHUNK_SIZE_I),
                position.y.div_euclid(CHUNK_SIZE_I),
            ));
        }
    }

    // Update back image in only touched chunks
    for chunk_pos in &to_update {
        let mut chunk = std::mem::take(terrain.get_mut(chunk_pos).unwrap());
        calculate_back_image(terrain, &chunk_pos, &mut chunk);
        terrain.insert(*chunk_pos, chunk);
    }
}

fn default_scenario(
    terrain_params: &TerrainParameters,
) -> (Vec<StructureEntry>, Chunks, Vec<DropItemEntry>) {
    (
        vec![
            wrap_structure(Box::new(TransportBelt::new(10, 3, Rotation::Left))),
            wrap_structure(Box::new(TransportBelt::new(11, 3, Rotation::Left))),
            wrap_structure(Box::new(TransportBelt::new(12, 3, Rotation::Left))),
            wrap_structure(Box::new(OreMine::new(12, 2, Rotation::Bottom))),
            wrap_structure(Box::new(Furnace::new(&Position::new(8, 3)))),
            wrap_structure(Box::new(Assembler::new(&Position::new(6, 3)))),
            wrap_structure(Box::new(WaterWell::new(&Position::new(14, 5)))),
            wrap_structure(Box::new(Boiler::new(&Position::new(13, 5)))),
            wrap_structure(Box::new(SteamEngine::new(&Position::new(12, 5)))),
        ],
        gen_terrain(terrain_params),
        vec![],
    )
}

fn pipe_bench(
    terrain_params: &TerrainParameters,
) -> (Vec<StructureEntry>, Chunks, Vec<DropItemEntry>) {
    let (mut structures, mut terrain, items) = default_scenario(terrain_params);

    structures
        .extend((11..=100).map(|x| wrap_structure(Box::new(Pipe::new(&Position::new(x, 10))))));
    structures
        .extend((10..=99).map(|x| wrap_structure(Box::new(Pipe::new(&Position::new(x, 100))))));
    structures
        .extend((10..=99).map(|x| wrap_structure(Box::new(Pipe::new(&Position::new(10, x))))));
    structures
        .extend((11..=100).map(|x| wrap_structure(Box::new(Pipe::new(&Position::new(100, x))))));

    update_water(&structures, &mut terrain, &terrain_params);

    (structures, terrain, items)
}

fn inserter_bench(
    terrain_params: &TerrainParameters,
) -> (Vec<StructureEntry>, Chunks, Vec<DropItemEntry>) {
    let (mut structures, mut terrain, items) = default_scenario(terrain_params);

    structures.extend((10..=100).map(|x| {
        if x % 2 == 0 {
            wrap_structure(Box::new({
                let mut chest = Chest::new(&Position::new(x, 10));
                chest
                    .inventory_mut(true)
                    .map(|inv| inv.add_item(&ItemType::IronOre));
                chest
            }))
        } else {
            wrap_structure(Box::new(Inserter::new(x, 10, Rotation::Right)))
        }
    }));
    structures.extend((10..=100).map(|x| {
        wrap_structure(if x % 2 == 0 {
            Box::new({
                let mut chest = Chest::new(&Position::new(x, 100));
                chest
                    .inventory_mut(true)
                    .map(|inv| inv.add_item(&ItemType::CoalOre));
                chest
            })
        } else {
            Box::new(Inserter::new(x, 100, Rotation::Left)) as Box<dyn Structure>
        })
    }));
    structures.extend((11..=99).map(|x| {
        if x % 2 == 0 {
            wrap_structure(Box::new(Chest::new(&Position::new(10, x))) as Box<dyn Structure>)
        } else {
            wrap_structure(Box::new(Inserter::new(10, x, Rotation::Top)) as Box<dyn Structure>)
        }
    }));
    structures.extend((11..=99).map(|x| {
        wrap_structure(if x % 2 == 0 {
            Box::new(Chest::new(&Position::new(100, x))) as Box<dyn Structure>
        } else {
            Box::new(Inserter::new(100, x, Rotation::Bottom)) as Box<dyn Structure>
        })
    }));

    update_water(&structures, &mut terrain, &terrain_params);

    (structures, terrain, items)
}

fn transport_bench(
    terrain_params: &TerrainParameters,
) -> (Vec<StructureEntry>, Chunks, Vec<DropItemEntry>) {
    let (mut structures, mut terrain, mut items) = default_scenario(terrain_params);

    structures.extend(
        (11..=100).map(|x| wrap_structure(Box::new(TransportBelt::new(x, 10, Rotation::Left)))),
    );
    items.extend((11..=100).map(|x| DropItemEntry::new(ItemType::CoalOre, &Position::new(x, 10))));
    structures.extend(
        (10..=99).map(|x| wrap_structure(Box::new(TransportBelt::new(x, 100, Rotation::Right)))),
    );
    items.extend((10..=99).map(|x| DropItemEntry::new(ItemType::IronOre, &Position::new(x, 100))));
    structures.extend(
        (10..=99).map(|x| wrap_structure(Box::new(TransportBelt::new(10, x, Rotation::Bottom)))),
    );
    items.extend((10..=99).map(|x| DropItemEntry::new(ItemType::CopperOre, &Position::new(10, x))));
    structures.extend(
        (11..=100).map(|x| wrap_structure(Box::new(TransportBelt::new(100, x, Rotation::Top)))),
    );
    items
        .extend((11..=100).map(|x| DropItemEntry::new(ItemType::StoneOre, &Position::new(100, x))));

    update_water(&structures, &mut terrain, &terrain_params);

    (structures, terrain, items)
}

fn electric_bench(
    terrain_params: &TerrainParameters,
) -> (Vec<StructureEntry>, Chunks, Vec<DropItemEntry>) {
    let (mut structures, mut terrain, items) = default_scenario(terrain_params);

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
    structures.extend((11..=99).map(|x| {
        if x % 2 == 0 {
            wrap_structure(Box::new(Assembler::new(&Position::new(10, x))) as Box<dyn Structure>)
        } else {
            wrap_structure(Box::new(ElectPole::new(&Position::new(10, x))) as Box<dyn Structure>)
        }
    }));
    structures.extend((11..=99).map(|x| {
        wrap_structure(if x % 2 == 0 {
            Box::new(Assembler::new(&Position::new(100, x))) as Box<dyn Structure>
        } else {
            Box::new(ElectPole::new(&Position::new(100, x))) as Box<dyn Structure>
        })
    }));

    update_water(&structures, &mut terrain, &terrain_params);

    (structures, terrain, items)
}

pub(crate) fn select_scenario(
    name: &str,
    terrain_params: &TerrainParameters,
) -> Result<(Vec<StructureEntry>, Chunks, Vec<DropItemEntry>), JsValue> {
    match name {
        "default" => Ok(default_scenario(terrain_params)),
        "pipe_bench" => Ok(pipe_bench(terrain_params)),
        "inserter_bench" => Ok(inserter_bench(terrain_params)),
        "transport_bench" => Ok(transport_bench(terrain_params)),
        "electric_bench" => Ok(electric_bench(terrain_params)),
        _ => js_err!("Scenario name not valid: {}", name),
    }
}

impl FactorishState {
    pub(super) fn update_cache(&mut self) -> Result<(), JsValue> {
        let positions = self
            .structures
            .iter()
            .map(|s| s.dynamic.as_deref().map(|d| *d.position()))
            .flatten()
            .collect::<Vec<_>>();
        for position in positions {
            self.update_fluid_connections(&position).unwrap();
        }

        for s in self
            .structures
            .iter_mut()
            .filter_map(|s| s.dynamic.as_deref_mut())
        {
            s.select_recipe(0).ok();
        }

        let structures = std::mem::take(&mut self.structures);
        for i in 0..structures.len() {
            for j in i + 1..structures.len() {
                let structure1 = structures[i].dynamic.as_deref().unwrap();
                let structure2 = structures[j].dynamic.as_deref().unwrap();
                if (structure1.power_sink() && structure2.power_source()
                    || structure1.power_source() && structure2.power_sink())
                    && structure1.position().distance(structure2.position())
                        <= structure1.wire_reach().min(structure2.wire_reach()) as i32
                {
                    let add = PowerWire(
                        StructureId {
                            id: i as u32,
                            gen: 0,
                        },
                        StructureId {
                            id: j as u32,
                            gen: 0,
                        },
                    );
                    if self.power_wires.iter().find(|p| **p == add).is_none() {
                        self.power_wires.push(add);
                    }
                }
            }
        }
        self.structures = structures;

        self.power_networks = build_power_networks(
            &StructureDynIter::new_all(&mut self.structures),
            &self.power_wires,
        );
        console_log!(
            "power: {:?}",
            self.power_networks
                .iter()
                .map(|nw| format!(
                    "wires {} sources {} sinks {}",
                    nw.wires.len(),
                    nw.sources.len(),
                    nw.sinks.len()
                ))
                .collect::<Vec<_>>()
        );
        console_log!(
            "Assemblers: {}",
            self.structure_iter()
                .filter(|s| s.name() == "Assembler")
                .count()
        );
        console_log!(
            "ElectPole: {}",
            self.structure_iter()
                .filter(|s| s.name() == "Electric Pole")
                .count()
        );

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

        self.drop_items_index = build_index(&self.drop_items);

        Ok(())
    }
}
