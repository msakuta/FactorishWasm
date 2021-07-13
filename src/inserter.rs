use super::{
    draw_direction_arrow,
    dyn_iter::DynIterMut,
    items::{render_drop_item, ItemType},
    structure::Structure,
    DropItem, FactorishState, FrameProcResult, Inventory, InventoryTrait, Position, Rotation,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

#[derive(Serialize, Deserialize)]
pub(crate) struct Inserter {
    position: Position,
    rotation: Rotation,
    cooldown: f64,
    hold_item: Option<ItemType>,
}

const INSERTER_TIME: f64 = 20.;

impl Inserter {
    pub(crate) fn new(x: i32, y: i32, rotation: Rotation) -> Self {
        Inserter {
            position: Position { x, y },
            rotation,
            cooldown: 0.,
            hold_item: None,
        }
    }

    fn get_arm_angles(&self) -> (f64, f64) {
        let phase = if self.hold_item.is_some() {
            self.cooldown / INSERTER_TIME
        } else {
            (INSERTER_TIME - self.cooldown) / INSERTER_TIME
        };
        (
            self.rotation.angle_rad() + (phase * 0.8 + 0.5) * std::f64::consts::PI,
            self.rotation.angle_rad() + ((1. - phase) * 0.8 + 0.2 - 0.5) * std::f64::consts::PI,
        )
    }
}

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
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        let (x, y) = (self.position.x as f64 * 32., self.position.y as f64 * 32.);
        match depth {
            0 => match state.image_inserter.as_ref() {
                Some(img) => {
                    context
                        .draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                            &img.bitmap,
                            0.,
                            0.,
                            32.,
                            32.,
                            x,
                            y,
                            32.,
                            32.,
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
                            &img.bitmap,
                            48.,
                            0.,
                            16.,
                            32.,
                            x,
                            y,
                            16.,
                            32.,
                        )?;
                    context.translate(x + 8., y + 8.)?;
                    context.rotate(-angles.0)?;
                    context.rotate(angles.1)?;
                    context.translate(-(x + 8.), -(y + 20.))?;
                    context
                        .draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                            &img.bitmap,
                            32.,
                            0.,
                            16.,
                            24.,
                            x,
                            y,
                            16.,
                            24.,
                        )?;
                    if let Some(item) = self.hold_item {
                        context.translate(x + 4., y + 4.)?;
                        context.rotate(-angles.1)?;
                        render_drop_item(state, context, &item, 0, 0)?;
                    }
                    context.restore();
                }
                None => return Err(JsValue::from_str("inserter-arm image not available")),
            },
            2 => draw_direction_arrow((x, y), &self.rotation, state, context)?,
            _ => panic!(),
        }

        Ok(())
    }

    fn frame_proc(
        &mut self,
        state: &mut FactorishState,
        structures: &mut dyn DynIterMut<Item = dyn Structure + '_>,
    ) -> Result<FrameProcResult, ()> {
        let input_position = self.position.add(self.rotation.delta_inv());
        let output_position = self.position.add(self.rotation.delta());

        // It is unclear why I need to put explicit lifetimes to avoid compile errors.
        fn find_structure_at<'a, 'b>(
            structures: &'a mut dyn DynIterMut<Item = dyn Structure + 'b>,
            position: Position,
        ) -> Option<&'a mut (dyn Structure + 'b)> {
            structures
                .dyn_iter_mut()
                .find(|structure| *structure.position() == position)
        }

        if self.hold_item.is_none() {
            if self.cooldown <= 1. {
                self.cooldown = 0.;
                let ret = FrameProcResult::None;

                let mut try_hold =
                    |structures: &mut dyn DynIterMut<Item = dyn Structure + '_>, type_| -> bool {
                        if let Some(structure) = find_structure_at(structures, output_position) {
                            // console_log!(
                            //     "found structure to output[{}]: {}, {}, {}",
                            //     structure_idx,
                            //     structure.name(),
                            //     output_position.x,
                            //     output_position.y
                            // );
                            if structure.can_input(&type_) || structure.movable() {
                                // ret = FrameProcResult::InventoryChanged(output_position);
                                self.hold_item = Some(type_);
                                self.cooldown += INSERTER_TIME;
                                true
                            } else {
                                false
                            }
                        } else {
                            self.hold_item = Some(type_);
                            self.cooldown += INSERTER_TIME;
                            true
                        }
                    };

                let mut lets_try_hold = None;
                if let Some(&DropItem { type_, id, .. }) = state.find_item(&input_position) {
                    if try_hold(structures, type_) {
                        state.remove_item(id);
                    } else {
                        // console_log!("fail output_object: {:?}", type_);
                    }
                } else if let Some(structure) = find_structure_at(structures, input_position) {
                    lets_try_hold = Some(structure.can_output());
                    // console_log!("outputting from a structure at {:?}", structure.position());
                    // if let Ok((item, callback)) = structure.output(state, &output_position) {
                    //     lets_try_hold = Some((item, callback));
                    // } else {
                    //     // console_log!("output failed");
                    // }
                }

                if let Some(output_items) = lets_try_hold {
                    if let Some(type_) = (|| {
                        // First, try matching the item that the structure at the output position can accept.
                        if let Some(structure) = find_structure_at(structures, output_position) {
                            // console_log!(
                            //     "found structure to output[{}]: {}, {}, {}",
                            //     structure_idx,
                            //     structure.name(),
                            //     output_position.x,
                            //     output_position.y
                            // );
                            for item in output_items {
                                if structure.can_input(&item.0) || structure.movable() {
                                    // ret = FrameProcResult::InventoryChanged(output_position);
                                    self.hold_item = Some(item.0);
                                    self.cooldown += INSERTER_TIME;
                                    return Some(item);
                                }
                            }
                        } else if let Some(item) = output_items.into_iter().next() {
                            // If there is no structures at the output, anything can output.
                            self.hold_item = Some(item.0);
                            self.cooldown += INSERTER_TIME;
                            return Some(item);
                        }
                        None
                    })() {
                        if let Some(structure) = find_structure_at(structures, input_position) {
                            structure.output(state, &type_.0)?;
                            return Ok(FrameProcResult::InventoryChanged(input_position));
                        } else {
                            console_log!(
                                "We have confirmed that there is input structure, right???"
                            );
                            return Err(());
                        }
                    }
                    // if let Some(pos) = state.selected_structure_inventory {
                    //     if pos == input_position {
                    //         return Ok(FrameProcResult::InventoryChanged(input_position));
                    //         // if let Err(e) = state.on_show_inventory.call2(&window(), &JsValue::from(output_position.x), &JsValue::from(output_position.y)) {
                    //         //     console_log!("on_show_inventory fail: {:?}", e);
                    //         // }
                    //     }
                    // }
                    // console_log!("output succeeded: {:?}", item.type_);
                    // }
                }
                return Ok(ret);
            } else {
                self.cooldown -= 1.;
            }
        } else if self.cooldown < 1. {
            self.cooldown = 0.;
            if let Some(item_type) = self.hold_item {
                let mut try_move = |state: &mut FactorishState| {
                    if let Ok(()) =
                        state.new_object(output_position.x, output_position.y, item_type)
                    {
                        self.cooldown += INSERTER_TIME;
                        self.hold_item = None;
                    }
                };
                if let Some(structure) = find_structure_at(structures, output_position) {
                    if structure
                        .input(&DropItem::new(
                            &mut state.serial_no,
                            item_type,
                            output_position.x,
                            output_position.y,
                        ))
                        .is_ok()
                    {
                        self.cooldown += INSERTER_TIME;
                        self.hold_item = None;
                        return Ok(FrameProcResult::InventoryChanged(output_position));
                    } else if structure.movable() {
                        try_move(state)
                    }
                } else {
                    try_move(state);
                }
            }
        } else {
            self.cooldown -= 1.;
        }
        Ok(FrameProcResult::None)
    }

    fn rotate(&mut self) -> Result<(), ()> {
        self.rotation = self.rotation.next();
        Ok(())
    }

    fn set_rotation(&mut self, rotation: &Rotation) -> Result<(), ()> {
        self.rotation = *rotation;
        Ok(())
    }

    fn destroy_inventory(&mut self) -> Inventory {
        let mut ret = Inventory::new();
        if let Some(item) = self.hold_item {
            ret.add_item(&item);
        }
        ret
    }

    crate::serialize_impl!();
}
