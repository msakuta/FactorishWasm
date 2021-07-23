use super::{
    assembler::Assembler,
    boiler::Boiler,
    elect_pole::ElectPole,
    furnace::Furnace,
    ore_mine::OreMine,
    power_network::build_power_networks,
    steam_engine::SteamEngine,
    structure::{Structure, StructureBoxed, StructureDynIter, StructureEntry, StructureId},
    transport_belt::TransportBelt,
    water_well::WaterWell,
    FactorishState, Position, PowerWire, Rotation,
};
use wasm_bindgen::prelude::*;

fn wrap_structure(s: StructureBoxed) -> StructureEntry {
    StructureEntry {
        gen: 0,
        dynamic: Some(s),
    }
}

fn default_scenario() -> Vec<StructureEntry> {
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
    ]
}

fn transport_bench() -> Vec<StructureEntry> {
    let mut structures = default_scenario();

    structures.extend(
        (11..=100).map(|x| wrap_structure(Box::new(TransportBelt::new(x, 10, Rotation::Left)))),
    );
    structures.extend(
        (10..=99).map(|x| wrap_structure(Box::new(TransportBelt::new(x, 100, Rotation::Right)))),
    );
    structures.extend(
        (10..=99).map(|x| wrap_structure(Box::new(TransportBelt::new(10, x, Rotation::Bottom)))),
    );
    structures.extend(
        (11..=100).map(|x| wrap_structure(Box::new(TransportBelt::new(100, x, Rotation::Top)))),
    );

    structures
}

fn electric_bench() -> Vec<StructureEntry> {
    let mut structures = default_scenario();

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

    structures
}

pub(crate) fn select_scenario(name: &str) -> Result<Vec<StructureEntry>, JsValue> {
    match name {
        "default" => Ok(default_scenario()),
        "transport_bench" => Ok(transport_bench()),
        "electric_bench" => Ok(electric_bench()),
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

        Ok(())
    }
}
