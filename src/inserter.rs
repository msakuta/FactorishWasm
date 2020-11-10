use super::structure::Structure;
use super::{draw_direction_arrow, DropItem, FactorishState, FrameProcResult, Position, Rotation};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

pub(crate) struct Inserter {
    position: Position,
    rotation: Rotation,
    cooldown: f64,
}

impl Inserter {
    pub(crate) fn new(x: i32, y: i32, rotation: Rotation) -> Self {
        Inserter {
            position: Position { x, y },
            rotation,
            cooldown: 0.,
        }
    }

    fn get_arm_angles(&self) -> (f64, f64) {
        (
            self.rotation.angle_rad()
                + (2. * self.cooldown.min(INSERTER_TIME - self.cooldown) / INSERTER_TIME * 0.8
                    + 0.2
                    - 0.5)
                    * std::f64::consts::PI,
            self.rotation.angle_rad()
                + (2. * self.cooldown.max(INSERTER_TIME - self.cooldown) / INSERTER_TIME * 0.8
                    + 0.2
                    - 0.5)
                    * std::f64::consts::PI,
        )
    }
}

const INSERTER_TIME: f64 = 20.;

impl Structure for Inserter {
    fn name(&self) -> &str {
        "Inserter"
    }

    fn position(&self) -> &Position {
        &self.position
    }

    fn draw(
        &self,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
    ) -> Result<(), JsValue> {
        let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
        match depth {
            0 => match state.image_inserter.as_ref() {
                Some(img) => {
                    context
                        .draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                            img, 0., 0., 32., 32., x, y, 32., 32.,
                        )?;
                }
                None => return Err(JsValue::from_str("inserter image not available")),
            },
            1 => match state.image_inserter.as_ref() {
                Some(img) => {
                    let angles = self.get_arm_angles();
                    context.save();
                    context.translate(x + 16., y + 16.)?;
                    context.rotate(angles.0)?;
                    context.translate(-(x + 8.), -(y + 20.))?;
                    context
                        .draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                            img, 48., 0., 16., 32., x, y, 16., 32.,
                        )?;
                    context.translate(x + 8., y + 8.)?;
                    context.rotate(-angles.0)?;
                    context.rotate(angles.1)?;
                    context.translate(-(x + 8.), -(y + 20.))?;
                    context
                        .draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                            img, 32., 0., 16., 24., x, y, 16., 24.,
                        )?;
                    context.restore();
                }
                None => return Err(JsValue::from_str("inserter-arm image not available")),
            },
            2 => draw_direction_arrow((x, y), &self.rotation, state, context)?,
            _ => assert!(false),
        }

        Ok(())
    }

    fn frame_proc(
        &mut self,
        state: &mut FactorishState,
        structures: &mut dyn Iterator<Item = &mut Box<dyn Structure>>,
    ) -> Result<FrameProcResult, ()> {
        if self.cooldown <= 1. {
            self.cooldown = 0.;
            let input_position = self.position.add(self.rotation.delta_inv());
            let output_position = self.position.add(self.rotation.delta());
            let mut ret = FrameProcResult::None;

            let mut try_output = |state: &mut FactorishState,
                                  structures: &mut dyn Iterator<Item = &mut Box<dyn Structure>>,
                                  type_|
             -> bool {
                let mut try_move = |state: &mut FactorishState| {
                    if let Ok(()) = state.new_object(output_position.x, output_position.y, type_) {
                        self.cooldown += INSERTER_TIME;
                        true
                    } else {
                        false
                    }
                };
                if let Some((_structure_idx, structure)) = structures
                    .enumerate()
                    .find(|(_idx, structure)| *structure.position() == output_position)
                {
                    // console_log!(
                    //     "found structure to output[{}]: {}, {}, {}",
                    //     structure_idx,
                    //     structure.name(),
                    //     output_position.x,
                    //     output_position.y
                    // );
                    if structure
                        .input(&DropItem::new(
                            &mut state.serial_no,
                            type_,
                            output_position.x,
                            output_position.y,
                        ))
                        .is_ok()
                    {
                        ret = FrameProcResult::InventoryChanged(output_position);
                        self.cooldown += INSERTER_TIME;
                        true
                    } else if structure.movable() {
                        try_move(state)
                    } else {
                        false
                    }
                } else {
                    try_move(state)
                }
            };

            if let Some(&DropItem { type_, id, .. }) = state.find_item(&input_position) {
                if try_output(state, structures, type_) {
                    state.remove_item(id);
                } else {
                    // console_log!("fail output_object: {:?}", type_);
                }
            } else if let Some((_, structure)) = structures.enumerate().find(|(_, s)| {
                s.position().x == input_position.x && s.position().y == input_position.y
            }) {
                // console_log!("outputting from a structure at {:?}", structure.position());
                if let Ok((item, callback)) = structure.output(state, &output_position) {
                    if try_output(state, structures, item.type_) {
                        callback(&item);
                        if let Some(pos) = state.selected_structure_inventory {
                            if pos == input_position {
                                return Ok(FrameProcResult::InventoryChanged(input_position));
                                // if let Err(e) = state.on_show_inventory.call2(&window(), &JsValue::from(output_position.x), &JsValue::from(output_position.y)) {
                                //     console_log!("on_show_inventory fail: {:?}", e);
                                // }
                            }
                        }
                        // console_log!("output succeeded: {:?}", item.type_);
                    }
                } else {
                    // console_log!("output failed");
                }
            }
            return Ok(ret);
        } else {
            self.cooldown -= 1.;
        }
        Ok(FrameProcResult::None)
    }

    fn rotate(&mut self) -> Result<(), ()> {
        self.rotation.next();
        Ok(())
    }

    fn set_rotation(&mut self, rotation: &Rotation) -> Result<(), ()> {
        self.rotation = *rotation;
        Ok(())
    }
}
