use crate::Position;
use serde::{Deserialize, Serialize};
use specs::{Component, DenseVecStorage, Entity, World, WorldExt, WriteStorage};
use wasm_bindgen::prelude::*;

use std::cmp::Eq;

#[derive(Eq, PartialEq, Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) enum FluidType {
    Water,
    Steam,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct FluidBox {
    pub type_: Option<FluidType>,
    pub amount: f64,
    pub max_amount: f64,
    pub input_enable: bool,
    pub output_enable: bool,
    #[serde(skip)]
    pub connect_to: [Option<Entity>; 4],
    pub filter: Option<FluidType>, // permits undefined
}

type Connection = (Entity, Entity, u8);

#[derive(Clone, Copy, Debug)]
enum FluidBoxType {
    Input,
    Output,
    Buffer,
}

type FluidBundle<'a> = (
    Entity,
    Option<&'a mut InputFluidBox>,
    Option<&'a mut OutputFluidBox>,
    Option<&'a mut BufferFluidBox>,
);

impl FluidBox {
    pub(crate) fn new(input_enable: bool, output_enable: bool) -> Self {
        Self {
            type_: None,
            amount: 0.,
            max_amount: 100.,
            input_enable,
            output_enable,
            connect_to: [None; 4],
            filter: None,
        }
    }

    pub(crate) fn desc(&self) -> String {
        let amount_ratio = self.amount / self.max_amount * 100.;
        // Progress bar
        format!("{}{}{}{}",
            format!("{}: {:.0}%<br>", self.type_.map(|v| format!("{:?}", v)).unwrap_or_else(|| "None".to_string()), amount_ratio),
            "<div style='position: relative; width: 100px; height: 10px; background-color: #001f1f; margin: 2px; border: 1px solid #3f3f3f'>",
            format!("<div style='position: absolute; width: {}px; height: 10px; background-color: #ff00ff'></div></div>",
                amount_ratio),
            format!("Connect to: {:?}", self.connect_to),
            )
    }

    fn list_connections(world: &World) -> Vec<Connection> {
        use specs::Join;
        let entities = world.entities();
        let positions = world.read_component::<Position>();
        let ifb = world.read_component::<InputFluidBox>();
        let ofb = world.read_component::<OutputFluidBox>();
        let bfb = world.read_component::<BufferFluidBox>();
        let mut ret = vec![];
        for (entity, position, ifb2, ofb2, bfb2) in (
            &entities,
            &positions,
            (&ifb).maybe(),
            (&ofb).maybe(),
            (&bfb).maybe(),
        )
            .join()
        {
            if ifb2.is_none() && ofb2.is_none() && bfb2.is_none() {
                continue;
            }
            let mut has_fluid_box = |x, y, idx| {
                if let Some(bundle) = (
                    &entities,
                    &positions,
                    (&ifb).maybe(),
                    (&ofb).maybe(),
                    (&bfb).maybe(),
                )
                    .join()
                    .find(|(_, position, _, _, _)| **position == Position { x, y })
                {
                    if bundle.2.is_some() || bundle.3.is_some() || bundle.4.is_some() {
                        ret.push((entity, bundle.0, idx));
                    }
                }
            };

            // Fluid containers connect to other containers
            let Position { x, y } = *position;
            has_fluid_box(x - 1, y, 0);
            has_fluid_box(x, y - 1, 1);
            has_fluid_box(x + 1, y, 2);
            has_fluid_box(x, y + 1, 3);
        }
        ret
    }

    pub(crate) fn update_connections(
        &mut self,
        entity: Entity,
        connections: &[Connection],
    ) -> Result<(), JsValue> {
        self.connect_to = [None; 4];
        for (_, to, idx) in connections.iter().filter(|(from, _, _)| *from == entity) {
            self.connect_to[*idx as usize] = Some(*to);
        }
        Ok(())
    }

    fn simulate<'a>(
        &mut self,
        self_entity: Entity,
        self_type: FluidBoxType,
        fluid_bundles: &mut [FluidBundle<'a>],
    ) {
        let mut _biggest_flow_idx = -1;
        let mut biggest_flow_amount = 1e-3; // At least this amount of flow is required for displaying flow direction
                                            // In an unlikely event, a fluid box without either input or output ports has nothing to do
        if self.amount == 0. || !self.input_enable && !self.output_enable {
            return;
        }
        let connect_to = self.connect_to;
        for (i, connect) in connect_to.iter().copied().enumerate() {
            let connect = if let Some(connect) = connect {
                connect
            } else {
                continue;
            };
            // let dir_idx = i % 4;
            // let pos = Position {
            //     x: position.x + rel_dir[dir_idx][0],
            //     y: position.y + rel_dir[dir_idx][1],
            // };
            // if pos.x < 0 || state.width <= pos.x as u32 || pos.y < 0 || state.height <= pos.y as u32
            // {
            //     continue;
            // }
            // if let Some(structure) = structures
            //     .map(|s| s)
            //     .find(|s| s.components.position == Some(pos))
            // {
            let mut process_fluid_box =
                |self_box: &mut FluidBox, fluid_box: &mut FluidBox, entity: Entity, type_| {
                    // Different types of fluids won't mix
                    if 0. < fluid_box.amount
                        && fluid_box.type_ != self_box.type_
                        && fluid_box.type_.is_some()
                    {
                        return;
                    }
                    let pressure = fluid_box.amount - self_box.amount;
                    let flow = pressure * 0.1;
                    // Check input/output valve state
                    if if flow < 0. {
                        !self_box.output_enable
                            || !fluid_box.input_enable
                            || fluid_box.filter.is_some() && fluid_box.filter != self_box.type_
                    } else {
                        !self_box.input_enable
                            || !fluid_box.output_enable
                            || self_box.filter.is_some() && self_box.filter != fluid_box.type_
                    } {
                        return;
                    }
                    fluid_box.amount -= flow;
                    self_box.amount += flow;
                    if flow < 0. {
                        fluid_box.type_ = self_box.type_;
                    } else {
                        self_box.type_ = fluid_box.type_;
                    }
                    if biggest_flow_amount < flow.abs() {
                        biggest_flow_amount = flow;
                        _biggest_flow_idx = i as isize;
                    }
                };

            if let Some(input_fluid_box) =
                fluid_bundles.iter_mut().find(|bundle| bundle.0 == connect)
            {
                if let Some(input_fluid_box) = &mut input_fluid_box.1 {
                    process_fluid_box(self, &mut input_fluid_box.0, connect, FluidBoxType::Input);
                }
            }

            if let Some(buffer_fluid_box) =
                fluid_bundles.iter_mut().find(|bundle| bundle.0 == connect)
            {
                if let Some(buffer_fluid_box) = &mut buffer_fluid_box.3 {
                    process_fluid_box(self, &mut buffer_fluid_box.0, connect, FluidBoxType::Buffer);
                }
            }
        }
    }

    pub(crate) fn fluid_simulation(world: &World) -> Result<(), JsValue> {
        use specs::Join;
        let connections = FluidBox::list_connections(&world);
        let entities = world.entities();
        let mut ifb = world.write_component::<InputFluidBox>();
        let mut ofb = world.write_component::<OutputFluidBox>();
        let mut bfb = world.write_component::<BufferFluidBox>();
        for (entity, output_fluid_box) in (&entities, &mut ofb).join() {
            output_fluid_box
                .0
                .update_connections(entity, &connections)?;
        }
        for (entity, input_fluid_box) in (&entities, &mut ifb).join() {
            input_fluid_box.0.update_connections(entity, &connections)?;
        }
        for (entity, buffer_fluid_box) in (&entities, &mut bfb).join() {
            buffer_fluid_box
                .0
                .update_connections(entity, &connections)?;
        }
        let mut fluid_bundle = (
            &entities,
            (&mut ifb).maybe(),
            (&mut ofb).maybe(),
            (&mut bfb).maybe(),
        )
            .join()
            .collect::<Vec<_>>();
        for i in 0..fluid_bundle.len() {
            let (first, second) = fluid_bundle.split_at_mut(i);
            if !second.is_empty() {
                let (center, last) = second.split_at_mut(1);
                let entry = &mut center[0];
                if let Some(ofb) = &mut entry.2 {
                    ofb.0.simulate(entry.0, FluidBoxType::Output, first);
                    ofb.0.simulate(entry.0, FluidBoxType::Output, last);
                }
                if let Some(bfb) = &mut entry.3 {
                    bfb.0.simulate(entry.0, FluidBoxType::Buffer, first);
                    bfb.0.simulate(entry.0, FluidBoxType::Buffer, last);
                }
            }
        }
        Ok(())
    }
}

#[derive(Serialize, Deserialize, Component)]
pub(crate) struct InputFluidBox(pub FluidBox);

#[derive(Serialize, Deserialize, Component)]
pub(crate) struct OutputFluidBox(pub FluidBox);

#[derive(Serialize, Deserialize, Component)]
pub(crate) struct BufferFluidBox(pub FluidBox);
