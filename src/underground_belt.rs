use super::{
    items::ItemType,
    structure::{ItemResponse, ItemResponseResult, Structure, StructureDynIter, StructureId},
    DropItem, FactorishState, FrameProcResult, Inventory, Position, RotateErr, Rotation, TILE_SIZE,
    TILE_SIZE_I,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

#[derive(Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum UnderDirection {
    ToGround,
    ToSurface,
}

pub(crate) struct UndergroundSection {
    items: Vec<(i32, ItemType)>,
}

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

    fn distance(&self, target: &Position) -> i32 {
        match self.rotation {
            Rotation::Left | Rotation::Right => (target.x - self.position().x).abs(),
            Rotation::Top | Rotation::Bottom => (target.y - self.position().y).abs(),
        }
    }
}

impl Structure for UndergroundBelt {
    fn name(&self) -> &str {
        "Underground Belt"
    }

    fn position(&self) -> &Position {
        &self.position
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
                    0.,
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
        _state: &mut FactorishState,
        structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        if let Some(target) = self.target.and_then(|id| structures.get(id)) {
            let distance = self.distance(target.position());
            let mut delete_me = vec![];
            for (i, item) in self.items.iter_mut().enumerate() {
                item.0 += 1;
                if distance * TILE_SIZE_I < item.0 {
                    delete_me.push(i);
                }
            }
            delete_me
                .into_iter()
                .rev()
                .map(|i| self.items.swap_remove(i));
        }
        Ok(FrameProcResult::None)
    }

    fn movable(&self) -> bool {
        true
    }

    fn rotate(&mut self, _others: &StructureDynIter) -> Result<(), RotateErr> {
        self.rotation = self.rotation.next();
        Ok(())
    }

    fn set_rotation(&mut self, rotation: &Rotation) -> Result<(), ()> {
        self.rotation = *rotation;
        Ok(())
    }

    fn item_response(&mut self, item: &DropItem) -> Result<ItemResponseResult, ()> {
        let vx = self.rotation.delta().0;
        let vy = self.rotation.delta().1;
        let ax = if self.rotation.is_vertial() {
            (item.x as f64 / TILE_SIZE).floor() * TILE_SIZE + TILE_SIZE / 2.
        } else {
            item.x as f64
        };
        let ay = if self.rotation.is_horizontal() {
            (item.y as f64 / TILE_SIZE).floor() * TILE_SIZE + TILE_SIZE / 2.
        } else {
            item.y as f64
        };
        let moved_x = ax as i32 + vx;
        let moved_y = ay as i32 + vy;
        Ok((ItemResponse::Move(moved_x, moved_y), None))
    }

    fn can_input(&self, item_type: &ItemType) -> bool {
        self.direction == UnderDirection::ToGround
    }

    fn can_output(&self, structures: &StructureDynIter) -> Inventory {
        if self.direction == UnderDirection::ToSurface {
            if let Some(target) = self.target.and_then(|id| structures.get(id)) {
                let distance = self.distance(target.position());
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

    crate::serialize_impl!();
}
