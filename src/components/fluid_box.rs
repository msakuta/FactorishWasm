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

type FluidFlow = (Entity, FluidBoxType, f64, Option<FluidType>);

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

    fn simulate(
        &self,
        self_entity: Entity,
        self_type: FluidBoxType,
        input_fluid_box_storage: &WriteStorage<InputFluidBox>,
        buffer_fluid_box_storage: &WriteStorage<BufferFluidBox>,
    ) -> Vec<FluidFlow> {
        let mut _biggest_flow_idx = -1;
        let mut biggest_flow_amount = 1e-3; // At least this amount of flow is required for displaying flow direction
                                            // In an unlikely event, a fluid box without either input or output ports has nothing to do
        let mut ret = vec![];
        if self.amount == 0. || !self.input_enable && !self.output_enable {
            return ret;
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
                |self_box: &FluidBox, fluid_box: &FluidBox, entity: Entity, type_| {
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
                    let fluid_type = if flow < 0. {
                        self_box.type_
                    } else {
                        fluid_box.type_
                    };
                    ret.push((entity, type_, -flow, fluid_type));
                    ret.push((self_entity, self_type, flow, fluid_type));
                    if biggest_flow_amount < flow.abs() {
                        biggest_flow_amount = flow;
                        _biggest_flow_idx = i as isize;
                    }
                };

            if let Some(input_fluid_box) = input_fluid_box_storage.get(connect) {
                process_fluid_box(self, &input_fluid_box.0, connect, FluidBoxType::Input);
            }

            if let Some(input_fluid_box) = buffer_fluid_box_storage.get(connect) {
                process_fluid_box(self, &input_fluid_box.0, connect, FluidBoxType::Buffer);
            }
        }
        ret
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
        let mut flows = vec![];
        for (entity, output_fluid_box) in (&entities, &ofb).join() {
            flows.extend(
                output_fluid_box
                    .0
                    .simulate(entity, FluidBoxType::Output, &ifb, &bfb),
            );
        }
        for (entity, buffer_fluid_box) in (&entities, &bfb).join() {
            flows.extend(
                buffer_fluid_box
                    .0
                    .simulate(entity, FluidBoxType::Buffer, &ifb, &bfb),
            );
        }
        for (entity, fb_type, flow, fluid_type) in flows {
            let fb = match fb_type {
                FluidBoxType::Input => ifb.get_mut(entity).map(|i| &mut i.0),
                FluidBoxType::Output => ofb.get_mut(entity).map(|i| &mut i.0),
                FluidBoxType::Buffer => bfb.get_mut(entity).map(|i| &mut i.0),
            };
            if let Some(fb) = fb {
                fb.amount += flow;
                fb.type_ = fluid_type;
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
