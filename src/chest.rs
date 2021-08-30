use super::{
    drop_items::DropItem,
    gl::utils::{enable_buffer, Flatten},
    inventory::InventoryType,
    items::ItemType,
    structure::{
        ItemResponse, ItemResponseResult, Structure, StructureBundle, StructureComponents,
        StructureDynIter,
    },
    FactorishState, FrameProcResult, Inventory, InventoryTrait, Position,
};
use cgmath::{Matrix3, Matrix4, Vector3};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use web_sys::{CanvasRenderingContext2d, WebGlRenderingContext as GL};

const CHEST_CAPACITY: usize = 100;

#[derive(Serialize, Deserialize)]
pub(crate) struct Chest {
    inventory: Inventory,
}

impl Chest {
    pub(crate) fn new(position: Position) -> StructureBundle {
        StructureBundle {
            dynamic: Box::new(Chest {
                inventory: Inventory::new(),
            }),
            components: StructureComponents::new_with_position(position),
        }
    }
}

impl Structure for Chest {
    fn name(&self) -> &'static str {
        "Chest"
    }

    fn draw(
        &self,
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
        match state.image_chest.as_ref() {
            Some(img) => {
                context.draw_image_with_image_bitmap(&img.bitmap, x, y)?;
                Ok(())
            }
            None => Err(JsValue::from_str("chest image not available")),
        }
    }

    fn draw_gl(
        &self,
        components: &StructureComponents,
        state: &FactorishState,
        gl: &GL,
        depth: i32,
        is_ghost: bool,
    ) -> Result<(), JsValue> {
        if depth != 0 {
            return Ok(());
        }
        let position = components
            .position
            .ok_or_else(|| js_str!("OreMine without Position"))?;
        let (x, y) = (
            position.x as f32 + state.viewport.x as f32,
            position.y as f32 + state.viewport.y as f32,
        );
        let shader = state
            .assets
            .textured_shader
            .as_ref()
            .ok_or_else(|| js_str!("Shader not found"))?;
        gl.use_program(Some(&shader.program));
        gl.uniform1f(shader.alpha_loc.as_ref(), if is_ghost { 0.5 } else { 1. });
        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_chest));
        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::from_nonuniform_scale(1., 1.).flatten(),
        );

        enable_buffer(&gl, &state.assets.screen_buffer, 2, shader.vertex_position);
        gl.uniform_matrix4fv_with_f32_array(
            shader.transform_loc.as_ref(),
            false,
            (state.get_world_transform()?
                * Matrix4::from_scale(2.)
                * Matrix4::from_translation(Vector3::new(x, y, 0.)))
            .flatten(),
        );
        gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

        Ok(())
    }

    fn desc(&self, _components: &StructureComponents, _state: &FactorishState) -> String {
        format!(
            "Items: \n{}",
            self.inventory
                .iter()
                .map(|item| format!("{:?}: {}<br>", item.0, item.1))
                .fold(String::from(""), |accum, item| accum + &item)
        )
    }

    fn item_response(
        &mut self,
        components: &mut StructureComponents,
        _item: &DropItem,
    ) -> Result<ItemResponseResult, JsValue> {
        if 0 < self.add_inventory(InventoryType::Storage, &_item.type_, 1) {
            Ok((
                ItemResponse::Consume,
                Some(FrameProcResult::InventoryChanged(
                    components
                        .position
                        .ok_or_else(|| js_str!("Chest without Position components"))?,
                )),
            ))
        } else {
            Ok((ItemResponse::None, None))
        }
    }

    fn input(&mut self, components: &mut StructureComponents, o: &DropItem) -> Result<(), JsValue> {
        self.item_response(components, o)
            .map(|_| ())
            .map_err(|_| JsValue::from_str("ItemResponse failed"))
    }

    /// Chest can put any item
    fn can_input(&self, _components: &StructureComponents, _o: &ItemType) -> bool {
        self.inventory.len() < CHEST_CAPACITY
    }

    fn can_output(
        &self,
        _components: &StructureComponents,
        _structures: &StructureDynIter,
    ) -> Inventory {
        self.inventory.clone()
    }

    fn output(&mut self, _state: &mut FactorishState, item_type: &ItemType) -> Result<(), ()> {
        if self.inventory.remove_item(item_type) {
            Ok(())
        } else {
            Err(())
        }
    }

    fn inventory(&self, invtype: InventoryType) -> Option<&Inventory> {
        match invtype {
            InventoryType::Storage => Some(&self.inventory),
            _ => None,
        }
    }

    fn inventory_mut(&mut self, invtype: InventoryType) -> Option<&mut Inventory> {
        match invtype {
            InventoryType::Storage => Some(&mut self.inventory),
            _ => None,
        }
    }

    super::serialize_impl!();
}
