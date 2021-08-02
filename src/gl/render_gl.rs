use super::{
    shader_bundle::ShaderBundle,
    utils::{enable_buffer, vertex_buffer_data},
};
use crate::{
    apply_bounds, performance, Cell, FactorishState, Ore, OreValue, Position, CHUNK_SIZE,
    CHUNK_SIZE_I, TILE_SIZE,
};
use cgmath::{Matrix3, Matrix4, Vector2, Vector3};
use wasm_bindgen::prelude::*;
use web_sys::{WebGlProgram, WebGlRenderingContext as GL, WebGlShader, WebGlTexture};

#[wasm_bindgen]
impl FactorishState {
    pub fn render_gl_init(&mut self, context: GL) -> Result<(), JsValue> {
        // let vert_shader = compile_shader(
        //     &context,
        //     GL::VERTEX_SHADER,
        //     r#"
        //     attribute vec2 vertexData;
        //     uniform mat4 transform;
        //     uniform mat3 texTransform;
        //     varying vec2 texCoords;
        //     void main() {
        //         gl_Position = transform * vec4(vertexData.xy, 0.01, 1.0);

        //         texCoords = (texTransform * vec3((vertexData.xy - 1.) * 0.5, 1.)).xy;
        //     }
        // "#,
        // )?;
        // let frag_shader = compile_shader(
        //     &context,
        //     GL::FRAGMENT_SHADER,
        //     r#"
        //     precision mediump float;

        //     varying vec2 texCoords;

        //     uniform sampler2D texture;
        //     uniform float alpha;

        //     void main() {
        //         // vec4 texColor = texture2D( texture, vec2(texCoords.x, texCoords.y) );
        //         // gl_FragColor = vec4(texColor.rgb, texColor.a * alpha);
        //         gl_FragColor = vec4(1, 1, 1, 1);
        //     }
        // "#,
        // )?;
        // let program = link_program(&context, &vert_shader, &frag_shader)?;
        // context.use_program(Some(&program));

        // let shader = ShaderBundle::new(&context, program);

        // context.active_texture(GL::TEXTURE0);

        // context.uniform1i(shader.texture_loc.as_ref(), 0);
        // context.uniform1f(shader.alpha_loc.as_ref(), 1.);

        context.enable(GL::BLEND);
        context.blend_equation(GL::FUNC_ADD);
        context.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);

        // self.assets.sprite_shader = Some(shader);

        let vert_shader = compile_shader(
            &context,
            GL::VERTEX_SHADER,
            r#"
            attribute vec2 vertexData;
            uniform mat4 transform;
            uniform mat3 texTransform;
            varying vec2 texCoords;
            void main() {
                gl_Position = transform * vec4(vertexData.xy, 0., 1.0);

                texCoords = (texTransform * vec3(vertexData.xy, 1.)).xy;
            }
        "#,
        )?;
        let frag_shader = compile_shader(
            &context,
            GL::FRAGMENT_SHADER,
            r#"
            precision mediump float;

            varying vec2 texCoords;

            uniform sampler2D texture;

            void main() {
                vec4 texColor = texture2D( texture, texCoords.xy );
                gl_FragColor = texColor;
                // gl_FragColor = vec4(1, 1, 1, 0.5);
            }
        "#,
        )?;
        let program = link_program(&context, &vert_shader, &frag_shader)?;
        context.use_program(Some(&program));
        self.assets.textured_shader = Some(ShaderBundle::new(&context, program));

        context.active_texture(GL::TEXTURE0);
        context.uniform1i(
            self.assets
                .textured_shader
                .as_ref()
                .and_then(|s| s.texture_loc.as_ref()),
            0,
        );

        self.assets.rect_buffer = Some(context.create_buffer().ok_or("failed to create buffer")?);
        context.bind_buffer(GL::ARRAY_BUFFER, self.assets.rect_buffer.as_ref());
        let rect_vertices: [f32; 8] = [1., 1., -1., 1., -1., -1., 1., -1.];
        vertex_buffer_data(&context, &rect_vertices);

        context.clear_color(0.0, 0.0, 0.5, 0.5);

        Ok(())
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
            (Matrix3::from_nonuniform_scale(
                self.viewport_width / self.viewport.scale,
                self.viewport_height / self.viewport.scale,
            ) * Matrix3::from_translation(Vector2::new(-self.viewport.x, -self.viewport.y))
                * Matrix3::from_nonuniform_scale(1. / TILE_SIZE, -1. / TILE_SIZE))
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
            &self.assets.rect_buffer,
            2,
            shader.vertex_position,
        );
        context.uniform_matrix4fv_with_f32_array(
            shader.transform_loc.as_ref(),
            false,
            <Matrix4<f32> as AsRef<[f32; 16]>>::as_ref(&Matrix4::from_scale(1.)),
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

        context.bind_texture(GL::TEXTURE_2D, Some(&self.assets.tex_back));
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
                if !cell.water {
                    continue;
                }

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
                context.uniform_matrix3fv_with_f32_array(
                    shader.tex_transform_loc.as_ref(),
                    false,
                    <Matrix3<f32> as AsRef<[f32; 9]>>::as_ref(
                        &(Matrix3::from_nonuniform_scale(0.25, 0.125)
                            * Matrix3::from_translation(Vector2::new(3., 3.))
                            * Matrix3::from_translation(Vector2::new(0.5, 0.5))
                            * Matrix3::from_scale(0.5)),
                    ),
                );
                context.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
                draws += 1;
                // cell_draws += 1;
            }
        }

        let mut draw_ore = |x, y, ore: u32| -> Result<(), JsValue> {
            if 0 < ore {
                let idx = (ore / 10).min(3);
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
                context.uniform_matrix3fv_with_f32_array(
                    shader.tex_transform_loc.as_ref(),
                    false,
                    <Matrix3<f32> as AsRef<[f32; 9]>>::as_ref(
                        &(Matrix3::from_nonuniform_scale(1. / 4., 1.)
                            * Matrix3::from_translation(Vector2::new(idx as f32, 0.))
                            * Matrix3::from_translation(Vector2::new(0.5, 0.5))
                            * Matrix3::from_scale(0.5)),
                    ),
                );
                context.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
                draws += 1;
            }
            Ok(())
        };

        let mut scan_ore = |ore, tex| -> Result<(), JsValue> {
            context.bind_texture(GL::TEXTURE_2D, Some(tex));
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

                    if let Some(OreValue(cell_ore, v)) = cell.ore {
                        if cell_ore == ore {
                            draw_ore(x, y, v)?;
                        }
                    }
                    // cell_draws += 1;
                }
            }
            Ok(())
        };

        scan_ore(Ore::Iron, &self.assets.tex_iron)?;
        scan_ore(Ore::Coal, &self.assets.tex_coal)?;
        scan_ore(Ore::Copper, &self.assets.tex_copper)?;
        scan_ore(Ore::Stone, &self.assets.tex_stone)?;

        console_log!("drawn: {}, bounds: {:?}", draws, (left, top, right, bottom));

        self.perf_render.add(performance().now() - start_render);

        Ok(())
    }
}

pub fn compile_shader(context: &GL, shader_type: u32, source: &str) -> Result<WebGlShader, String> {
    let shader = context
        .create_shader(shader_type)
        .ok_or_else(|| String::from("Unable to create shader object"))?;
    context.shader_source(&shader, source);
    context.compile_shader(&shader);

    if context
        .get_shader_parameter(&shader, GL::COMPILE_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(shader)
    } else {
        Err(context
            .get_shader_info_log(&shader)
            .unwrap_or_else(|| String::from("Unknown error creating shader")))
    }
}

pub fn link_program(
    context: &GL,
    vert_shader: &WebGlShader,
    frag_shader: &WebGlShader,
) -> Result<WebGlProgram, String> {
    let program = context
        .create_program()
        .ok_or_else(|| String::from("Unable to create shader object"))?;

    context.attach_shader(&program, vert_shader);
    context.attach_shader(&program, frag_shader);
    context.link_program(&program);

    if context
        .get_program_parameter(&program, GL::LINK_STATUS)
        .as_bool()
        .unwrap_or(false)
    {
        Ok(program)
    } else {
        Err(context
            .get_program_info_log(&program)
            .unwrap_or_else(|| String::from("Unknown error creating program object")))
    }
}
