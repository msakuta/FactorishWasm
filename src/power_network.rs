use super::{
    structure::{StructureDynIter, StructureId},
    Position, PowerWire,
};
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
pub(crate) struct PowerNetwork {
    pub wires: Vec<(StructureId, StructureId)>,
    pub sources: HashSet<StructureId>,
    pub sinks: HashSet<StructureId>,
}

pub(crate) fn build_power_networks(
    structures: &StructureDynIter,
    power_wires: &[PowerWire],
) -> Vec<PowerNetwork> {
    let structure_positions = structures
        .dyn_iter_id()
        .map(|(id, s)| (*s.position(), id))
        .collect::<HashMap<_, _>>();
    let mut left_wires = power_wires
        .iter()
        .map(|w| (structure_positions[&w.0], structure_positions[&w.1]))
        .collect::<HashSet<_>>();
    let mut ret = vec![];

    for (id, s) in structures.dyn_iter_id() {
        if !s.power_sink() && !s.power_source() {
            continue;
        }
        let mut expand_list = HashMap::<StructureId, Vec<(StructureId, StructureId)>>::new();
        let mut wires = vec![];
        let mut sources = HashSet::new();
        let mut sinks = HashSet::new();
        if s.power_source() {
            sources.insert(id);
        }
        if s.power_sink() {
            sinks.insert(id);
        }

        let mut check_struct = |id: StructureId| {
            if let Some(s) = structures.get(id) {
                if s.power_source() {
                    sources.insert(id);
                }
                if s.power_sink() {
                    sinks.insert(id);
                }
            }
        };

        console_log!(
            "before expand_list: {:?} for {:?} ({:?})",
            left_wires.len(),
            id,
            s.name()
        );

        expand_list.insert(id, vec![]);

        // Simple Dijkstra
        while !expand_list.is_empty() {
            let mut next_expand = HashMap::<StructureId, Vec<(StructureId, StructureId)>>::new();
            for (id, check) in expand_list {
                check_struct(id);
                console_log!("checking expand_list: {:?}, {:?}", id, check);
                while let Some(wire) = left_wires.iter().find(|w| w.0 == id || w.1 == id).copied() {
                    console_log!("found wire within expand_list: {:?}", wire);
                    next_expand.insert(if wire.0 == id { wire.1 } else { wire.0 }, vec![wire]);
                    left_wires.remove(&wire);
                    console_log!(
                        "left next_expand: {:?} left_wires: {:?}",
                        next_expand,
                        left_wires.len()
                    );
                    wires.push(wire);
                }
            }
            expand_list = next_expand;
        }

        console_log!(
            "resulting wires: {}, sources: {}, sinks: {}",
            wires.len(),
            sources.len(),
            sinks.len()
        );

        if !sources.is_empty() && !sinks.is_empty() {
            ret.push(PowerNetwork {
                wires,
                sources,
                sinks,
            });
        }
    }
    ret
}
