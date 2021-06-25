use super::structure::{Burner, DynIterMut, Structure, StructureBundle};
use super::{FactorishState, FrameProcResult, Position};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

#[derive(Serialize, Deserialize)]
pub(crate) struct ElectPole {
    position: Position,
    power: f64,
}

impl ElectPole {
    pub(crate) fn new(position: &Position) -> Self {
        ElectPole {
            position: *position,
            power: 0.,
        }
    }

    const POWER_CAPACITY: f64 = 10.;
}

impl Structure for ElectPole {
    fn name(&self) -> &str {
        "Electric Pole"
    }

    fn position(&self) -> &Position {
        &self.position
    }

    fn draw(
        &self,
        _burner: Option<&Burner>,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        let position = self.position;
        let (x, y) = (position.x as f64 * 32., position.y as f64 * 32.);
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
        _state: &mut FactorishState,
        structures: &mut dyn DynIterMut<Item = StructureBundle>,
        _burner: Option<&mut Burner>,
    ) -> Result<FrameProcResult, ()> {
        for structure in structures.dyn_iter_mut() {
            let target_position = structure.dynamic.position();
            if target_position.distance(&self.position) < 3 {
                if let Some(power) = structure
                    .dynamic
                    .power_outlet(Self::POWER_CAPACITY - self.power)
                {
                    self.power += power;
                }
            }
        }
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
