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
    structure::{StructureBundle, StructureDynIter, StructureEntry, StructureId},
    terrain::{
        calculate_back_image, gen_terrain, Chunks, ChunksExt, TerrainParameters, CHUNK_SIZE_I,
    },
    transport_belt::TransportBelt,
    FactorishState, InventoryTrait, Position, PowerWire, Rotation,
    water_well::WaterWell,
};
use std::collections::HashSet;
use wasm_bindgen::prelude::*;

fn wrap_structure(bundle: StructureBundle) -> StructureEntry {
    StructureEntry {
        gen: 0,
        bundle: Some(bundle),
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
        if let Some(position) = structure
            .bundle
            .as_ref()
            .and_then(|bundle| bundle.components.position)
        {
            if let Some(cell) = terrain.get_tile_mut(position) {
                cell.water = false;
            }
            to_update.insert(Position::new(
                position.x.div_euclid(CHUNK_SIZE_I),
                position.y.div_euclid(CHUNK_SIZE_I),
            ));
        }
    }

    // Update back image in only touched chunks
    for chunk_pos in &to_update {
        let mut cells = std::mem::take(&mut terrain.get_mut(chunk_pos).unwrap().cells);
        calculate_back_image(terrain, &chunk_pos, &mut cells);
        if let Some(c) = terrain.get_mut(chunk_pos) {
            c.cells = cells;
        }
    }
}

fn default_scenario(
    terrain_params: &TerrainParameters,
) -> (Vec<StructureEntry>, Chunks, Vec<DropItemEntry>) {
    let structures = vec![
        wrap_structure(TransportBelt::new(Position::new(10, 3), Rotation::Left)),
        wrap_structure(TransportBelt::new(Position::new(11, 3), Rotation::Left)),
        wrap_structure(TransportBelt::new(Position::new(12, 3), Rotation::Left)),
        wrap_structure(OreMine::new(12, 2, Rotation::Bottom)),
        wrap_structure(Furnace::new(&Position::new(8, 3))),
        wrap_structure(Assembler::new(&Position::new(6, 3))),
        wrap_structure(WaterWell::new(Position::new(14, 5))),
        wrap_structure(Boiler::new(&Position::new(13, 5))),
        wrap_structure(SteamEngine::new(Position::new(12, 5))),
    ];
    let mut terrain = gen_terrain(terrain_params);

    update_water(&structures, &mut terrain, &terrain_params);

    (structures, terrain, vec![])
}

fn pipe_bench(
    terrain_params: &TerrainParameters,
) -> (Vec<StructureEntry>, Chunks, Vec<DropItemEntry>) {
    let (mut structures, mut terrain, items) = default_scenario(terrain_params);

    structures.extend((11..=100).map(|x| wrap_structure(Pipe::new(Position::new(x, 10)))));
    structures.extend((10..=99).map(|x| wrap_structure(Pipe::new(Position::new(x, 100)))));
    structures.extend((10..=99).map(|x| wrap_structure(Pipe::new(Position::new(10, x)))));
    structures.extend((11..=100).map(|x| wrap_structure(Pipe::new(Position::new(100, x)))));

    update_water(&structures, &mut terrain, &terrain_params);

    (structures, terrain, items)
}

fn inserter_bench(
    terrain_params: &TerrainParameters,
) -> (Vec<StructureEntry>, Chunks, Vec<DropItemEntry>) {
    let (mut structures, mut terrain, items) = default_scenario(terrain_params);

    structures.extend((10..=100).map(|x| {
        if x % 2 == 0 {
            wrap_structure({
                let mut chest = Chest::new(Position::new(x, 10));
                if let Some(inv) = chest.inventory_mut(true) {
                    inv.add_item(&ItemType::IronOre);
                }
                chest
            })
        } else {
            wrap_structure(Inserter::new(Position::new(x, 10), Rotation::Right))
        }
    }));
    structures.extend((10..=100).map(|x| {
        wrap_structure(if x % 2 == 0 {
            let mut chest = Chest::new(Position::new(x, 100));
            if let Some(inv) = chest.inventory_mut(true) {
                inv.add_item(&ItemType::CoalOre);
            }
            chest
        } else {
            Inserter::new(Position::new(x, 100), Rotation::Left)
        })
    }));
    structures.extend((11..=99).map(|x| {
        if x % 2 == 0 {
            wrap_structure(Chest::new(Position::new(10, x)))
        } else {
            wrap_structure(Inserter::new(Position::new(10, x), Rotation::Top))
        }
    }));
    structures.extend((11..=99).map(|x| {
        wrap_structure(if x % 2 == 0 {
            Chest::new(Position::new(100, x))
        } else {
            Inserter::new(Position::new(100, x), Rotation::Bottom)
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
        (11..=100)
            .map(|x| wrap_structure(TransportBelt::new(Position::new(x, 10), Rotation::Left))),
    );
    items.extend((11..=100).map(|x| DropItemEntry::new(ItemType::CoalOre, &Position::new(x, 10))));
    structures.extend(
        (10..=99)
            .map(|x| wrap_structure(TransportBelt::new(Position::new(x, 100), Rotation::Right))),
    );
    items.extend((10..=99).map(|x| DropItemEntry::new(ItemType::IronOre, &Position::new(x, 100))));
    structures.extend(
        (10..=99)
            .map(|x| wrap_structure(TransportBelt::new(Position::new(10, x), Rotation::Bottom))),
    );
    items.extend((10..=99).map(|x| DropItemEntry::new(ItemType::CopperOre, &Position::new(10, x))));
    structures.extend(
        (11..=100)
            .map(|x| wrap_structure(TransportBelt::new(Position::new(100, x), Rotation::Top))),
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

    structures.extend((10..=100).map(|x| {
        if x % 2 == 0 {
            wrap_structure(Assembler::new(&Position::new(x, 10)))
        } else {
            wrap_structure(ElectPole::new(Position::new(x, 10)))
        }
    }));
    structures.extend((10..=100).map(|x| {
        wrap_structure(if x % 2 == 0 {
            Assembler::new(&Position::new(x, 100))
        } else {
            ElectPole::new(Position::new(x, 100))
        })
    }));
    structures.extend((11..=99).map(|x| {
        if x % 2 == 0 {
            wrap_structure(Assembler::new(&Position::new(10, x)))
        } else {
            wrap_structure(ElectPole::new(Position::new(10, x)))
        }
    }));
    structures.extend((11..=99).map(|x| {
        wrap_structure(if x % 2 == 0 {
            Assembler::new(&Position::new(100, x))
        } else {
            ElectPole::new(Position::new(100, x))
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
            .filter_map(|s| s.bundle.as_ref().and_then(|d| d.components.position))
            .collect::<Vec<_>>();
        for position in positions {
            self.update_fluid_connections(&position).unwrap();
        }

        for bundle in self.structures.iter_mut().filter_map(|s| s.bundle.as_mut()) {
            if let Some(factory) = bundle.components.factory.as_mut() {
                bundle.dynamic.select_recipe(factory, 0).ok();
            }
        }

        let structures = std::mem::take(&mut self.structures);
        for i in 0..structures.len() {
            for j in i + 1..structures.len() {
                let structure1 = structures[i].bundle.as_ref().unwrap();
                let structure2 = structures[j].bundle.as_ref().unwrap();
                if (structure1.dynamic.power_sink() && structure2.dynamic.power_source()
                    || structure1.dynamic.power_source() && structure2.dynamic.power_sink())
                    && structure1
                        .components
                        .position
                        .zip(structure2.components.position)
                        .map(|(p1, p2)| {
                            p1.distance(&p2)
                                <= structure1
                                    .dynamic
                                    .wire_reach()
                                    .min(structure2.dynamic.wire_reach())
                                    as i32
                        })
                        .unwrap_or(false)
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
                .filter(|s| s.dynamic.name() == "Assembler")
                .count()
        );
        console_log!(
            "ElectPole: {}",
            self.structure_iter()
                .filter(|s| s.dynamic.name() == "Electric Pole")
                .count()
        );

        for i in 0..self.structures.len() {
            let (s, others) = StructureDynIter::new(&mut self.structures, i)?;
            let id = StructureId {
                id: i as u32,
                gen: s.gen,
            };
            s.bundle
                .as_mut()
                .map(|bundle| {
                    bundle
                        .dynamic
                        .on_construction_self(id, &mut bundle.components, &others, true)
                })
                .unwrap_or(Ok(()))?;
        }

        self.drop_items_index = build_index(&self.drop_items);

        Ok(())
    }
}
