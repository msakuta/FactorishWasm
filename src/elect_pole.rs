use super::structure::{DynIterMut, Position, Structure, StructureBundle, StructureComponents};
use super::{FactorishState, FrameProcResult};
use serde::{Deserialize, Serialize};
use specs::{Builder, Entity, World, WorldExt};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

#[derive(Serialize, Deserialize)]
pub(crate) struct ElectPole {
    power: f64,
}

impl ElectPole {
    pub(crate) fn new(world: &mut World, position: Position) -> Entity {
        world
            .create_entity()
            .with(Box::new(ElectPole { power: 0. }) as Box<dyn Structure + Send + Sync>)
            .with(position)
            .build()
    }

    const POWER_CAPACITY: f64 = 10.;
}

impl Structure for ElectPole {
    fn name(&self) -> &str {
        "Electric Pole"
    }

    fn draw(
        &self,
        entity: Entity,
        components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        let (x, y) = if let Some(position) = &components.position {
            (position.x as f64 * 32., position.y as f64 * 32.)
        } else {
            (0., 0.)
        };
        match state.image_elect_pole.as_ref() {
            Some(img) => {
                // let (front, mid) = state.structures.split_at_mut(i);
                // let (center, last) = mid
                //     .split_first_mut()
                //     .ok_or(JsValue::from_str("Structures split fail"))?;

                // We could split and chain like above, but we don't have to, as long as we deal with immutable
                // references.
                context.draw_image_with_image_bitmap(&img.bitmap, x, y)?;
            }
            None => return Err(JsValue::from_str("elect-pole image not available")),
        }
        Ok(())
    }

    fn frame_proc(
        &mut self,
        components: &mut StructureComponents,
        _state: &mut FactorishState,
    ) -> Result<FrameProcResult, ()> {
        let position = components.position.as_ref().ok_or(())?;
        // for structure in structures.dyn_iter_mut() {
        //     if let Some(target_position) = structure.components.position.as_ref() {
        //         if target_position.distance(position) < 3 {
        //             if let Some(power) = structure
        //                 .dynamic
        //                 .power_outlet(Self::POWER_CAPACITY - self.power)
        //             {
        //                 self.power += power;
        //             }
        //         }
        //     };
        // }
        Ok(FrameProcResult::None)
    }

    fn power_sink(&self) -> bool {
        true
    }

    fn power_source(&self) -> bool {
        true
    }

    fn power_outlet(&mut self, demand: f64) -> Option<f64> {
        let power = demand.min(self.power);
        self.power -= power;
        Some(power)
    }

    fn wire_reach(&self) -> u32 {
        5
    }

    crate::serialize_impl!();
}
