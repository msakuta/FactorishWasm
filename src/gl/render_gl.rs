use super::{
    assets::MAX_SPRITES,
    shader_bundle::ShaderBundle,
    utils::{enable_buffer, vertex_buffer_sub_data},
};
use crate::{
    apply_bounds, performance, Cell, FactorishState, Ore, OreValue, Position, CHUNK_SIZE,
    CHUNK_SIZE_I, TILE_SIZE,
};
use cgmath::{Matrix3, Matrix4, Vector2, Vector3};
use std::future::Future;
use wasm_bindgen::prelude::*;
use web_sys::{WebGlRenderingContext as GL, WebGlShader, WebGlTexture};

#[wasm_bindgen]
impl FactorishState {
    pub fn render_gl_init(&mut self, gl: GL) -> Result<(), JsValue> {
        self.assets.prepare(gl)
    }

    pub fn render_gl(&mut self, gl: GL) -> Result<(), JsValue> {
        // let context = get_context()?;
        let start_render = performance().now();

        // context.clear_color((self.sim_time % 1.) as f32, 0.0, 0.5, 1.0);
        gl.clear(GL::COLOR_BUFFER_BIT);

        gl.enable(GL::BLEND);
        gl.disable(GL::DEPTH_TEST);

        let back_texture_transform =
            (Matrix3::from_translation(Vector2::new(
                -self.viewport.x,
                self.viewport_height / self.viewport.scale / TILE_SIZE - self.viewport.y,
            )) * Matrix3::from_nonuniform_scale(
                self.viewport_width / self.viewport.scale,
                self.viewport_height / self.viewport.scale,
            ) * Matrix3::from_nonuniform_scale(1. / TILE_SIZE, -1. / TILE_SIZE))
            //  * Matrix3::from_translation(Vector2::new(-2. * self.viewport.x * self.viewport.scale / TILE_SIZE, 2. * self.viewport.y * self.viewport.scale / TILE_SIZE)))
            .cast::<f32>()
            .ok_or_else(|| js_str!("world transform cast failed"))?;

        let assets = &self.assets;

        let shader = assets
            .textured_shader
            .as_ref()
            .ok_or_else(|| js_str!("Shader bundle not found!"))?;
        gl.use_program(Some(&shader.program));
        gl.active_texture(GL::TEXTURE0);

        gl.uniform1i(shader.texture_loc.as_ref(), 0);

        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            <Matrix3<f32> as AsRef<[f32; 9]>>::as_ref(&back_texture_transform),
        );
        gl.bind_texture(GL::TEXTURE_2D, Some(&self.assets.tex_dirt));
        enable_buffer(&gl, &self.assets.screen_buffer, 2, shader.vertex_position);
        gl.uniform_matrix4fv_with_f32_array(
            shader.transform_loc.as_ref(),
            false,
            <Matrix4<f32> as AsRef<[f32; 16]>>::as_ref(
                &(Matrix4::from_translation(Vector3::new(-1., -1., 0.)) * Matrix4::from_scale(2.)),
            ),
        );
        gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

        if self.assets.instanced_arrays_ext.is_some() {
            let mut positions = vec![];
            self.render_cells(
                |x, y, cell| {
                    if cell.image != 0 && positions.len() < MAX_SPRITES {
                        positions.push(1. * x as f32);
                        positions.push(1. * y as f32);
                    }
                    Ok(())
                },
                apply_bounds(
                    &self.bounds,
                    &self.viewport,
                    self.viewport_width,
                    self.viewport_height,
                ),
            )?;
            self.render_sprites_gl_instancing(&gl, &self.assets.tex_back, &positions)?;
        } else {
            self.render_sprites_gl(&gl, shader)?;
        }

        self.perf_render.add(performance().now() - start_render);

        Ok(())
    }

    fn get_world_transform(&self) -> Result<Matrix4<f32>, JsValue> {
        (Matrix4::from_translation(Vector3::new(-1., 1., 0.))
            * Matrix4::from_nonuniform_scale(
                TILE_SIZE / self.viewport_width,
                TILE_SIZE / self.viewport_height,
                1.,
            )
            * Matrix4::from_scale(self.viewport.scale)
            * Matrix4::from_nonuniform_scale(1., -1., 1.))
        .cast::<f32>()
        .ok_or_else(|| js_str!("world transform cast failed"))
    }

    fn render_sprites_gl(&self, context: &GL, shader: &ShaderBundle) -> Result<(), JsValue> {
        let world_transform = self.get_world_transform()?;

        context.enable(GL::BLEND);
        context.blend_equation(GL::FUNC_ADD);
        context.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);

        let bounds = apply_bounds(
            &self.bounds,
            &self.viewport,
            self.viewport_width,
            self.viewport_height,
        );

        let mut draws = 0;

        enable_buffer(
            &context,
            &self.assets.rect_buffer,
            2,
            shader.vertex_position,
        );

        let apply_transform = |x, y| {
            context.uniform_matrix4fv_with_f32_array(
                shader.transform_loc.as_ref(),
                false,
                <Matrix4<f32> as AsRef<[f32; 16]>>::as_ref(
                    &(world_transform
                        * Matrix4::from_translation(
                            Vector3::new(
                                2. * (self.viewport.x + x as f64),
                                2. * (self.viewport.y + y as f64),
                                0.,
                            )
                            .cast::<f32>()
                            .unwrap(),
                        )),
                ),
            );
        };

        let apply_texture_transform = |scale_x, scale_y, trans_x, trans_y| {
            context.uniform_matrix3fv_with_f32_array(
                shader.tex_transform_loc.as_ref(),
                false,
                <Matrix3<f32> as AsRef<[f32; 9]>>::as_ref(
                    &(Matrix3::from_nonuniform_scale(scale_x, scale_y)
                        * Matrix3::from_translation(Vector2::new(trans_x, trans_y))
                        * Matrix3::from_translation(Vector2::new(0.5, 0.5))
                        * Matrix3::from_scale(0.5)),
                ),
            );
        };

        context.bind_texture(GL::TEXTURE_2D, Some(&self.assets.tex_back));
        self.render_cells(
            |x, y, cell| {
                if cell.image == 0 {
                    return Ok(());
                }
                let srcx = cell.image % 4;
                let srcy = cell.image / 4;

                apply_transform(x, y);
                apply_texture_transform(0.25, 0.125, srcx as f32, srcy as f32);
                context.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
                draws += 1;
                // cell_draws += 1;
                Ok(())
            },
            bounds,
        )?;

        context.bind_texture(GL::TEXTURE_2D, Some(&self.assets.tex_weeds));
        self.render_cells(
            |x, y, cell| {
                if cell.grass_image == 0 {
                    return Ok(());
                }

                apply_transform(x, y);
                apply_texture_transform(1. / 7., 1., cell.grass_image as f32 + 1., 0.);
                context.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
                draws += 1;
                // cell_draws += 1;
                Ok(())
            },
            bounds,
        )?;

        let mut draw_ore = |x, y, ore: u32| -> Result<(), JsValue> {
            if 0 < ore {
                let idx = (ore / 10).min(3);
                apply_transform(x, y);
                apply_texture_transform(1. / 4., 1., idx as f32, 0.);
                context.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
                draws += 1;
            }
            Ok(())
        };

        let mut scan_ore = |ore, tex| -> Result<(), JsValue> {
            context.bind_texture(GL::TEXTURE_2D, Some(tex));
            self.render_cells(
                |x, y, cell| {
                    if let Some(OreValue(cell_ore, v)) = cell.ore {
                        if cell_ore == ore {
                            draw_ore(x, y, v)?;
                        }
                    };
                    Ok(())
                },
                bounds,
            )
        };

        scan_ore(Ore::Iron, &self.assets.tex_iron)?;
        scan_ore(Ore::Coal, &self.assets.tex_coal)?;
        scan_ore(Ore::Copper, &self.assets.tex_copper)?;
        scan_ore(Ore::Stone, &self.assets.tex_stone)?;

        console_log!("drawn: {}, bounds: {:?}", draws, bounds);

        Ok(())
    }

    fn render_cells(
        &self,
        mut draw: impl FnMut(i32, i32, &Cell) -> Result<(), JsValue>,
        (left, top, right, bottom): (i32, i32, i32, i32),
    ) -> Result<(), JsValue> {
        for y in top..=bottom {
            for x in left..=right {
                let chunk_pos =
                    Position::new(x.div_euclid(CHUNK_SIZE_I), y.div_euclid(CHUNK_SIZE_I));
                let chunk = self.board.get(&chunk_pos);
                let chunk = if let Some(chunk) = chunk {
                    chunk
                } else {
                    continue;
                };
                let (mx, my) = (x as usize % CHUNK_SIZE, y as usize % CHUNK_SIZE);
                let cell = &chunk.cells[(mx + my * CHUNK_SIZE) as usize];

                draw(x, y, cell)?;
                // cell_draws += 1;
            }
        }
        Ok(())
    }

    /// Render particles if the device supports instancing. It is much faster with fewer calls to the API.
    /// Note that there are no loops at all in this function.
    fn render_sprites_gl_instancing(
        &self,
        gl: &GL,
        texture: &WebGlTexture,
        sprites_buf: &[f32],
    ) -> Result<(), JsValue> {
        let world_transform = self.get_world_transform()?
            * Matrix4::from_scale(2.)
            * Matrix4::from_translation(
                Vector3::new(self.viewport.x, self.viewport.y, 0.)
                    .cast::<f32>()
                    .unwrap(),
            )
            * Matrix4::from_scale(2.);

        let instanced_arrays_ext = self
            .assets
            .instanced_arrays_ext
            .as_ref()
            .ok_or_else(|| JsValue::from_str("Instanced arrays not supported"))?;

        let shader = self
            .assets
            .textured_instancing_shader
            .as_ref()
            .ok_or_else(|| JsValue::from_str("Could not find textured_instancing_shader"))?;
        if shader.attrib_position_loc < 0 {
            return Err(JsValue::from_str("matrix location was not found"));
        }

        gl.use_program(Some(&shader.program));

        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(texture));

        let scale = Matrix4::from_nonuniform_scale(TILE_SIZE as f32, -TILE_SIZE as f32, 1.);

        gl.uniform_matrix4fv_with_f32_array(
            shader.transform_loc.as_ref(),
            false,
            <Matrix4<f32> as AsRef<[f32; 16]>>::as_ref(&(world_transform)),
        );

        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            <Matrix3<f32> as AsRef<[f32; 9]>>::as_ref(&Matrix3::from_scale(1.)),
        );

        enable_buffer(gl, &self.assets.rect_buffer, 2, shader.vertex_position);

        gl.bind_buffer(GL::ARRAY_BUFFER, self.assets.sprites_buffer.as_ref());
        vertex_buffer_sub_data(gl, &sprites_buf[..sprites_buf.len().min(MAX_SPRITES)]);

        let stride = 2 * 4;
        gl.vertex_attrib_pointer_with_i32(
            shader.attrib_position_loc as u32,
            2,
            GL::FLOAT,
            false,
            stride,
            0,
        );

        instanced_arrays_ext.vertex_attrib_divisor_angle(shader.attrib_position_loc as u32, 1);
        gl.enable_vertex_attrib_array(shader.attrib_position_loc as u32);

        instanced_arrays_ext.draw_arrays_instanced_angle(
            GL::TRIANGLE_FAN,
            0,                                             // offset
            4,                                             // num vertices per instance
            sprites_buf.len().min(MAX_SPRITES) as i32 / 2, // num instances
        )?;

        // console_log!("drawn {} instances: {:?}", sprites_buf.len(), &sprites_buf[..10]);

        Ok(())
    }
}
