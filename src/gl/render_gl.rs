use super::utils::enable_buffer;
use crate::{
    apply_bounds, performance, Cell, FactorishState, Ore, OreValue, Position, CHUNK_SIZE,
    CHUNK_SIZE_I, TILE_SIZE,
};
use cgmath::{Matrix3, Matrix4, Vector2, Vector3};
use wasm_bindgen::prelude::*;
use web_sys::WebGlRenderingContext as GL;

#[wasm_bindgen]
impl FactorishState {
    pub fn render_gl_init(&mut self, gl: GL) -> Result<(), JsValue> {
        self.assets.prepare(gl)
    }

    pub fn render_gl(&mut self, context: GL) -> Result<(), JsValue> {
        // let context = get_context()?;
        let start_render = performance().now();

        // context.clear_color((self.sim_time % 1.) as f32, 0.0, 0.5, 1.0);
        context.clear(GL::COLOR_BUFFER_BIT);

        context.enable(GL::BLEND);
        context.disable(GL::DEPTH_TEST);

        let world_transform = (Matrix4::from_translation(Vector3::new(-1., 1., 0.))
            * Matrix4::from_nonuniform_scale(
                TILE_SIZE / self.viewport_width,
                TILE_SIZE / self.viewport_height,
                1.,
            )
            * Matrix4::from_scale(self.viewport.scale)
            * Matrix4::from_nonuniform_scale(1., -1., 1.))
        .cast::<f32>()
        .ok_or_else(|| js_str!("world transform cast failed"))?;

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
        context.use_program(Some(&shader.program));
        context.active_texture(GL::TEXTURE0);

        context.uniform1i(shader.texture_loc.as_ref(), 0);

        context.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            <Matrix3<f32> as AsRef<[f32; 9]>>::as_ref(&back_texture_transform),
        );
        context.bind_texture(GL::TEXTURE_2D, Some(&self.assets.tex_dirt));
        enable_buffer(
            &context,
            &self.assets.screen_buffer,
            2,
            shader.vertex_position,
        );
        context.uniform_matrix4fv_with_f32_array(
            shader.transform_loc.as_ref(),
            false,
            <Matrix4<f32> as AsRef<[f32; 16]>>::as_ref(
                &(Matrix4::from_translation(Vector3::new(-1., -1., 0.)) * Matrix4::from_scale(2.)),
            ),
        );
        context.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

        context.enable(GL::BLEND);
        context.blend_equation(GL::FUNC_ADD);
        context.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);

        let (left, top, right, bottom) = apply_bounds(
            &self.bounds,
            &self.viewport,
            self.viewport_width,
            self.viewport_height,
        );

        // let (dx, dy) = (x as f64 * 32., y as f64 * 32.);
        // if cell.water || cell.image != 0 {
        //     let srcx = cell.image % 4;
        //     let srcy = cell.image / 4;
        //     context.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
        //         &back_tiles.bitmap, (srcx * 32) as f64, (srcy * 32) as f64, 32., 32., dx, dy, 32., 32.)?;
        // } else {
        // context.draw_image_with_image_bitmap(&img.bitmap, dx, dy)?;
        // if let Some(weeds) = &self.image_weeds {
        //     if 0 < cell.grass_image {
        //         context.draw_image_with_image_bitmap_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
        //             &weeds.bitmap,
        //             (cell.grass_image * 32) as f64, 0., 32., 32., dx, dy, 32., 32.)?;
        //     }
        // } else {
        //     console_log!("Weed image not found");
        // }
        // }

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
            (left, top, right, bottom),
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
            (left, top, right, bottom),
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
                (left, top, right, bottom),
            )
        };

        scan_ore(Ore::Iron, &self.assets.tex_iron)?;
        scan_ore(Ore::Coal, &self.assets.tex_coal)?;
        scan_ore(Ore::Copper, &self.assets.tex_copper)?;
        scan_ore(Ore::Stone, &self.assets.tex_stone)?;

        console_log!("drawn: {}, bounds: {:?}", draws, (left, top, right, bottom));

        self.perf_render.add(performance().now() - start_render);

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
}
