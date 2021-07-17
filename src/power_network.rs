use super::{
    structure::{StructureDynIter, StructureId},
    Position, PowerWire,
};
use std::collections::HashMap;

#[derive(Debug)]
pub(crate) struct PowerNetwork {
    pub wires: Vec<(StructureId, StructureId)>,
    pub sources: Vec<StructureId>,
    pub sinks: Vec<StructureId>,
}

pub(crate) fn build_power_networks(
    structures: &StructureDynIter,
    power_wires: &[PowerWire],
) -> Vec<PowerNetwork> {
    let mut checked = HashMap::<PowerWire, ()>::new();
    let mut ret = vec![];

    for (id, s) in structures.dyn_iter_id() {
        if !s.power_sink() && !s.power_source() {
            continue;
        }
        let mut expand_list = HashMap::<Position, Vec<PowerWire>>::new();
        let mut wires = vec![];
        let mut sources = vec![];
        let mut sinks = vec![];
        if s.power_source() {
            sources.push(id);
        }
        if s.power_sink() {
            sinks.push(id);
        }

        let mut check_struct = |position: &Position| {
            if let Some((id, s)) = structures
                .dyn_iter_id()
                .find(|(_, structure)| *structure.position() == *position)
            {
                if s.power_source() {
                    sources.push(id);
                }
                if s.power_sink() {
                    sinks.push(id);
                }
                Some(id)
            } else {
                None
            }
        };

        let mut lr = None;
        for wire in power_wires {
            if checked.contains_key(wire) {
                continue;
            }
            let left = if wire.0 == *s.position() {
                expand_list.insert(wire.1, vec![*wire]);
                checked.insert(*wire, ());
                check_struct(&wire.1)
            } else {
                None
            };
            let right = if wire.1 == *s.position() {
                expand_list.insert(wire.0, vec![*wire]);
                checked.insert(*wire, ());
                check_struct(&wire.0)
            } else {
                None
            };
            if let Some((left, right)) = left.zip(right) {
                lr = Some((left, right));
                break;
            }
        }

        if let Some(lr) = lr {
            wires.push(lr);
        }
        // Simple Dijkstra
        while !expand_list.is_empty() {
            let mut next_expand = HashMap::<Position, Vec<PowerWire>>::new();
            for check in &expand_list {
                for wire in power_wires.iter() {
                    if checked.get(wire).is_some() {
                        continue;
                    }
                    if wire.0 == *check.0 {
                        next_expand.insert(wire.1, vec![*wire]);
                        checked.insert(*wire, ());
                        check_struct(&wire.1);
                    } else if wire.1 == *check.0 {
                        next_expand.insert(wire.0, vec![*wire]);
                        checked.insert(*wire, ());
                        check_struct(&wire.1);
                    }
                }
            }
            expand_list = next_expand;
        }

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
