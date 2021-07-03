use super::{
    burner::Burner,
    draw_direction_arrow,
    factory::Factory,
    items::{render_drop_item, DropItem, ItemType},
    structure::{Energy, Size, Structure, StructureBoxed, StructureBundle, StructureComponents},
    FactorishState, FrameProcResult, Inventory, InventoryTrait, Movable, Position, Rotation,
    TILE_SIZE_I,
};
use serde::{Deserialize, Serialize};
use specs::{Builder, Component, DenseVecStorage, Entity, World, WorldExt};
use wasm_bindgen::prelude::*;
use web_sys::CanvasRenderingContext2d;

#[derive(Serialize, Deserialize, Component)]
pub(crate) struct Inserter {
    cooldown: f64,
    hold_item: Option<ItemType>,
}

const INSERTER_TIME: f64 = 15.;

impl Inserter {
    pub(crate) fn new(world: &mut World, position: Position, rotation: Rotation) -> Entity {
        world
            .create_entity()
            .with(Box::new(Inserter {
                cooldown: 0.,
                hold_item: None,
            }) as StructureBoxed)
            .with(position)
            .with(rotation)
            .with(Inserter {
                cooldown: 0.,
                hold_item: None,
            })
            .build()
    }

    fn get_arm_angles(&self, components: &StructureComponents) -> (f64, f64) {
        let phase = if self.hold_item.is_some() {
            self.cooldown / INSERTER_TIME
        } else {
            (INSERTER_TIME - self.cooldown) / INSERTER_TIME
        };
        if let Some(rotation) = components.rotation {
            (
                rotation.angle_rad() + (phase * 0.8 + 0.5) * std::f64::consts::PI,
                rotation.angle_rad() + ((1. - phase) * 0.8 + 0.2 - 0.5) * std::f64::consts::PI,
            )
        } else {
            (0., 0.)
        }
    }

    pub(crate) fn process_item(
        &mut self,
        entity: Entity,
        state: &mut FactorishState,
        world: &World,
    ) -> Result<FrameProcResult, JsValue> {
        let position = world
            .read_component::<Position>()
            .get(entity)
            .copied()
            .ok_or_else(|| js_str!("Inserter without Position component"))?;
        let rotation = world
            .read_component::<Rotation>()
            .get(entity)
            .copied()
            .ok_or_else(|| js_str!("Inserter without Rotation component"))?;

        let input_position = position.add(rotation.delta_inv());
        let output_position = position.add(rotation.delta());

        if self.hold_item.is_none() {
            if self.cooldown <= 1. {
                self.cooldown = 0.;

                let mut try_hold = |type_| -> bool {
                    if let Some(value) = find_structure_at(world, output_position, |bundle| {
                        if bundle.can_input(&type_) || bundle.components.movable {
                            self.hold_item = Some(type_);
                            self.cooldown += INSERTER_TIME;
                            Some(true)
                        } else {
                            None
                        }
                    }) {
                        value
                    } else {
                        self.hold_item = Some(type_);
                        self.cooldown += INSERTER_TIME;
                        true
                    }
                };

                let mut lets_try_hold = None;
                if let Some(&DropItem { type_, id, .. }) = state.find_item(&input_position) {
                    if try_hold(type_) {
                        state.remove_item(id);
                    } else {
                        // console_log!("fail output_object: {:?}", type_);
                    }
                } else {
                    find_structure_at(world, input_position, |bundle| {
                        lets_try_hold = Some(bundle.dynamic.can_output());
                        Some(())
                    });
                }

                if let Some(ref output_items) = lets_try_hold {
                    if let Some(type_) = (|| {
                        // First, try matching the item that the structure at the output position can accept.
                        if let Some(item) = find_structure_at(world, output_position, |bundle| {
                            for item in output_items {
                                if bundle.dynamic.can_input(&item.0) || bundle.components.movable {
                                    self.hold_item = Some(*item.0);
                                    self.cooldown += INSERTER_TIME;
                                    return Some(Some(item));
                                }
                            }
                            Some(None)
                        }) {
                            return item;
                        } else if let Some(item) = output_items.into_iter().next() {
                            // If there is no structures at the output, anything can output.
                            self.hold_item = Some(*item.0);
                            self.cooldown += INSERTER_TIME;
                            return Some(item);
                        }
                        None
                    })() {
                        if find_structure_at(world, input_position, |bundle| {
                            bundle
                                .components
                                .factory
                                .as_mut()
                                .map(|factory| factory.output_inventory.remove_item(&type_.0))
                        })
                        .is_some()
                        {
                            return Ok(FrameProcResult::InventoryChanged(input_position));
                        } else {
                            console_log!(
                                "We have confirmed that there is input structure, right???"
                            );
                            return Err(js_str!(
                                "We have confirmed that there is input structure, right???"
                            ));
                        }
                    }
                }
                if let Some(pos) = state.selected_structure_inventory {
                    if pos == input_position {
                        return Ok(FrameProcResult::InventoryChanged(input_position));
                    }
                }
            } else {
                self.cooldown -= 1.;
            }
        } else if self.cooldown < 1. {
            self.cooldown = 0.;
            if let Some(item_type) = self.hold_item {
                return Ok(FrameProcResult::CreateItem {
                    item: DropItem::new(
                        &mut state.serial_no,
                        item_type,
                        output_position.x,
                        output_position.y,
                    ),
                    dropper: entity,
                });
            }
        } else {
            self.cooldown -= 1.;
        }
        Ok(FrameProcResult::None)
    }

    pub(crate) fn drop_item(&mut self) -> bool {
        if self.hold_item.is_some() {
            self.cooldown += INSERTER_TIME;
            self.hold_item = None;
            console_log!("Cleared item! {}", self.cooldown);
            return true;
        }
        false
    }

    pub(crate) fn draw(
        &self,
        entity: Entity,
        components: &StructureComponents,
        state: &FactorishState,
        context: &CanvasRenderingContext2d,
        depth: i32,
        _is_toolbar: bool,
    ) -> Result<(), JsValue> {
        let (x, y) = if let Some(position) = &components.position {
            (position.x as f64 * 32., position.y as f64 * 32.)
        } else {
            (0., 0.)
        };
        if depth == 1 {
            match state.image_inserter.as_ref() {
                Some(img) => {
                    let angles = self.get_arm_angles(components);
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
            }
        }
        Ok(())
    }
}

fn find_structure_at<T>(
    world: &World,
    position: Position,
    mut f: impl FnMut(&mut StructureBundle) -> Option<T>,
) -> Option<T> {
    let mut dynamic = world.write_component::<StructureBoxed>();
    let position_components = world.read_component::<Position>();
    let rotation = world.read_component::<Rotation>();
    let size = world.read_component::<Size>();
    let mut burner = world.write_component::<Burner>();
    let mut energy = world.write_component::<Energy>();
    let mut factory = world.write_component::<Factory>();
    let movable = world.read_component::<Movable>();

    use specs::Join;

    (
        &mut dynamic,
        &position_components,
        (&rotation).maybe(),
        (&size).maybe(),
        (&mut burner).maybe(),
        (&mut energy).maybe(),
        (&mut factory).maybe(),
        (&movable).maybe(),
    )
        .join()
        .find(|bundle| *bundle.1 == position)
        .map(|bundle| {
            f(&mut StructureBundle {
                dynamic: bundle.0.as_mut(),
                components: StructureComponents {
                    position: Some(*bundle.1),
                    rotation: bundle.2.copied(),
                    size: bundle.3.copied(),
                    burner: bundle.4,
                    energy: bundle.5,
                    factory: bundle.6,
                    movable: bundle.7.is_some(),
                },
            })
        })
        .flatten()
}

impl Structure for Inserter {
    fn name(&self) -> &str {
        "Inserter"
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
        let (x, y) = if let Some(position) = &components.position {
            (position.x as f64 * 32., position.y as f64 * 32.)
        } else {
            (0., 0.)
        };
        let rotation = components
            .rotation
            .ok_or_else(|| js_str!("Inserter without rotation component"))?;
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
            1 => (),
            2 => draw_direction_arrow((x, y), &rotation, state, context)?,
            _ => panic!(),
        }

        Ok(())
    }

    fn frame_proc(
        &mut self,
        _entity: Entity,
        components: &mut StructureComponents,
        state: &mut FactorishState,
    ) -> Result<FrameProcResult, ()> {
        let position = components.position.as_ref().ok_or(())?;
        let rotation = components.rotation.ok_or(())?;

        Ok(FrameProcResult::None)
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
