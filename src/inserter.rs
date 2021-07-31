use super::{
    draw_direction_arrow,
    drop_items::DropItem,
    items::{render_drop_item, ItemType},
    structure::{RotateErr, Structure, StructureDynIter, StructureId},
    FactorishState, FrameProcResult, Inventory, InventoryTrait, Position, Rotation,
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
    #[serde(skip)]
    input_structure: Option<StructureId>,
    #[serde(skip)]
    output_structure: Option<StructureId>,
}

const INSERTER_TIME: f64 = 20.;

impl Inserter {
    pub(crate) fn new(x: i32, y: i32, rotation: Rotation) -> Self {
        Inserter {
            position: Position { x, y },
            rotation,
            cooldown: 0.,
            hold_item: None,
            input_structure: None,
            output_structure: None,
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

    fn on_construction_common(
        &mut self,
        other_id: StructureId,
        other: &dyn Structure,
        construct: bool,
    ) -> Result<(), JsValue> {
        let input_position = self.position.add(self.rotation.delta_inv());
        let output_position = self.position.add(self.rotation.delta());
        if *other.position() == input_position {
            self.input_structure = if construct { Some(other_id) } else { None };
            console_log!(
                "Inserter{:?}: {} input_structure {:?}",
                self.position,
                if construct { "set" } else { "unset" },
                other_id
            );
        }
        if *other.position() == output_position {
            self.output_structure = if construct { Some(other_id) } else { None };
            console_log!(
                "Inserter{:?}: {} output_structure {:?}",
                self.position,
                if construct { "set" } else { "unset" },
                other_id
            );
        }
        Ok(())
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
        _me: StructureId,
        state: &mut FactorishState,
        structures: &mut StructureDynIter,
    ) -> Result<FrameProcResult, ()> {
        let input_position = self.position.add(self.rotation.delta_inv());
        let output_position = self.position.add(self.rotation.delta());

        if self.hold_item.is_none() {
            if self.cooldown <= 1. {
                self.cooldown = 0.;
                let ret = FrameProcResult::None;

                let mut try_hold = |structures: &mut StructureDynIter, type_| -> bool {
                    if let Some(structure) = self.output_structure.and_then(|id| structures.get(id))
                    {
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
                if let Some((id, &DropItem { type_, .. })) = state.find_item(&input_position) {
                    if try_hold(structures, type_) {
                        state.remove_item(id);
                    } else {
                        // console_log!("fail output_object: {:?}", type_);
                    }
                } else if let Some(structure) = self
                    .input_structure
                    .and_then(|id| structures.exclude_id(id).ok())
                    .flatten()
                {
                    lets_try_hold = Some(structure.can_output(structures));
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
                        if let Some(structure) =
                            self.output_structure.map(|id| structures.get(id)).flatten()
                        {
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
                        if let Some(structure) = self
                            .input_structure
                            .map(|id| structures.get_mut(id))
                            .flatten()
                        {
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
                let Self {
                    cooldown,
                    hold_item,
                    output_structure,
                    ..
                } = self;
                let mut try_move = |state: &mut FactorishState| {
                    if let Ok(()) = state.new_object(&output_position, item_type) {
                        *cooldown += INSERTER_TIME;
                        *hold_item = None;
                    }
                };
                if let Some(structure) = output_structure.map(|id| structures.get_mut(id)).flatten()
                {
                    if structure
                        .input(&DropItem::new(
                            item_type,
                            output_position.x,
                            output_position.y,
                        ))
                        .is_ok()
                    {
                        *cooldown += INSERTER_TIME;
                        *hold_item = None;
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

    fn on_construction(
        &mut self,
        other_id: StructureId,
        other: &dyn Structure,
        _others: &StructureDynIter,
        construct: bool,
    ) -> Result<(), JsValue> {
        self.on_construction_common(other_id, other, construct)
    }

    fn on_construction_self(
        &mut self,
        _self_id: StructureId,
        others: &StructureDynIter,
        construct: bool,
    ) -> Result<(), JsValue> {
        for (id, s) in others.dyn_iter_id() {
            self.on_construction_common(id, s, construct)?;
        }
        Ok(())
    }

    fn rotate(&mut self, others: &StructureDynIter) -> Result<(), RotateErr> {
        self.rotation = self.rotation.next();
        for (id, s) in others.dyn_iter_id() {
            self.on_construction_common(id, s, true)
                .map_err(|e| RotateErr::Other(e))?;
        }
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
