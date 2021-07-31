use super::{
    items::ItemType,
    structure::{ItemResponse, ItemResponseResult, Structure, StructureDynIter, StructureId},
    transport_belt::TransportBelt,
    DropItem, FactorishState, FrameProcResult, Inventory, Position, RotateErr, Rotation, TILE_SIZE,
    TILE_SIZE_I,
};
use rotate_enum::RotateEnum;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

const UNDERGROUND_REACH: i32 = 3;

#[derive(Serialize, Deserialize, PartialEq, Eq, Clone, Copy, RotateEnum)]
pub(crate) enum UnderDirection {
    ToGround,
    ToSurface,
}

use UnderDirection::*;

#[derive(Serialize, Deserialize)]
pub(crate) struct UndergroundBelt {
    position: Position,
    rotation: Rotation,
    direction: UnderDirection,
    target: Option<StructureId>,
    items: Vec<(i32, ItemType)>,
}

impl UndergroundBelt {
    pub(crate) fn new(x: i32, y: i32, rotation: Rotation, direction: UnderDirection) -> Self {
        Self {
            position: Position { x, y },
            rotation,
            direction,
            target: None,
            items: vec![],
        }
    }

    /// Distance to possibly connecting underground belt.
    fn distance(&self, target: &Position) -> Option<i32> {
        let src = self.position;
        if !match self.rotation {
            Rotation::Left | Rotation::Right => target.y == src.y,
            Rotation::Top | Rotation::Bottom => target.x == src.x,
        } {
            return None;
        }
        let dx = target.x - src.x;
        let dy = target.y - src.y;
        Some(match self.rotation {
            Rotation::Left => -dx,
            Rotation::Right => dx,
            Rotation::Top => -dy,
            Rotation::Bottom => dy,
        })
    }
}

impl Structure for UndergroundBelt {
    fn name(&self) -> &str {
        "Underground Belt"
    }

    fn position(&self) -> &Position {
        &self.position
    }

    fn rotation(&self) -> Option<Rotation> {
        Some(self.rotation)
    }

    fn draw(
        &self,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        };
        match state.image_underground_belt.as_ref() {
            Some(img) => {
                context.save();
                context.draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                    &img.bitmap,
                    match self.rotation {
                        Rotation::Left => 0.,
                        Rotation::Top => 1.,
                        Rotation::Right => 2.,
                        Rotation::Bottom => 3.,
                    } * TILE_SIZE,
                    match self.direction {
                        ToGround => 0.,
                        ToSurface => 64.,
                    },
                    TILE_SIZE,
                    TILE_SIZE * 2.,
                    self.position.x as f64 * TILE_SIZE,
                    self.position.y as f64 * TILE_SIZE - TILE_SIZE,
                    TILE_SIZE,
                    TILE_SIZE * 2.,
                )?;
                context.restore();
            }
            None => return Err(JsValue::from_str("belt image not available")),
        }

        Ok(())
    }

    fn frame_proc(
        &mut self,
        _me: StructureId,
        state: &mut FactorishState,
        structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        if self.direction == ToSurface {
            return Ok(FrameProcResult::None);
        }
        if let Some((target, distance)) = self
            .target
            .and_then(|id| structures.get(id))
            .and_then(|target| Some((*target.position(), self.distance(target.position())?)))
        {
            let mut delete_me = vec![];
            for (i, item) in self.items.iter_mut().enumerate() {
                if distance * TILE_SIZE_I < item.0 {
                    delete_me.push(i);
                } else {
                    item.0 += 1;
                }
            }
            for i in delete_me.into_iter().rev() {
                let item = self.items[i];
                match state.new_object(&target, item.1) {
                    Ok(()) => {
                        self.items.swap_remove(i);
                    }
                    Err(_) => (),
                }
            }
        }
        Ok(FrameProcResult::None)
    }

    fn movable(&self) -> bool {
        true
    }

    fn rotate(&mut self, _others: &StructureDynIter) -> Result<(), RotateErr> {
        self.direction = self.direction.next();
        Ok(())
    }

    fn set_rotation(&mut self, rotation: &Rotation) -> Result<(), ()> {
        self.rotation = *rotation;
        Ok(())
    }

    fn item_response(&mut self, item: &DropItem) -> Result<ItemResponseResult, ()> {
        if self.direction == ToGround {
            if self.target.is_some() {
                self.items.push((0, item.type_));
                Ok((ItemResponse::Consume, None))
            } else {
                Err(())
            }
        } else {
            TransportBelt::transport_item(self.rotation.next().next(), item)
        }
    }

    fn can_input(&self, _item_type: &ItemType) -> bool {
        self.direction == ToGround
    }

    fn can_output(&self, structures: &StructureDynIter) -> Inventory {
        if self.direction == ToSurface {
            if let Some(distance) = self
                .target
                .and_then(|id| structures.get(id))
                .and_then(|target| self.distance(target.position()))
            {
                return self
                    .items
                    .iter()
                    .filter_map(|item| {
                        if distance * TILE_SIZE_I < item.0 {
                            Some((item.1, 1))
                        } else {
                            None
                        }
                    })
                    .collect();
            }
        }
        Inventory::new()
    }

    fn desc(&self, _state: &FactorishState) -> String {
        format!("Connection: {:?}<br>Items: {:?}", self.target, self.items)
    }

    fn on_construction(
        &mut self,
        other_id: StructureId,
        other: &dyn Structure,
        others: &StructureDynIter,
        construct: bool,
    ) -> Result<(), JsValue> {
        if !construct {
            // This resetting is not strictly necessary with generational id
            if self.target == Some(other_id) {
                self.target = None;
            }
            return Ok(());
        }
        if other.name() != self.name() {
            return Ok(());
        }
        if other.rotation() != Some(self.rotation.next().next()) {
            return Ok(());
        }
        let opos = *other.position();
        let d = if let Some(d) = self.distance(&opos) {
            d
        } else {
            return Ok(());
        };

        if 0 < d && d < UNDERGROUND_REACH {
            return Ok(());
        }

        // If there is already an underground belt with shorter distance, don't connect to the new one.
        if let Some(target) = self.target.and_then(|target| others.get(target)) {
            let target_pos = target.position();
            if let Some(target_d) = self.distance(target_pos) {
                if target_d < d {
                    return Ok(());
                }
            }
        }

        self.target = Some(other_id);

        Ok(())
    }

    fn on_construction_self(
        &mut self,
        _id: StructureId,
        others: &StructureDynIter,
        _construct: bool,
    ) -> Result<(), JsValue> {
        if let Some((id, _)) = others.dyn_iter_id().find(|(_, other)| {
            if other.name() != self.name() || other.rotation() != Some(self.rotation.next().next())
            {
                return false;
            }

            let opos = *other.position();
            let d = if let Some(d) = self.distance(&opos) {
                d
            } else {
                return false;
            };

            // If there is already an underground belt with shorter distance, don't connect to the new one.
            if let Some(target) = self.target.and_then(|target| others.get(target)) {
                let target_pos = target.position();
                if let Some(target_d) = self.distance(target_pos) {
                    if target_d < d {
                        return false;
                    }
                }
            }

            true
        }) {
            self.target = Some(id);
        }
        Ok(())
    }

    crate::serialize_impl!();
}
