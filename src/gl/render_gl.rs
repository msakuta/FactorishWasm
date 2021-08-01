use super::{
    shader_bundle::ShaderBundle,
    utils::{enable_buffer, vertex_buffer_data},
};
use crate::{performance, FactorishState, TILE_SIZE};
use cgmath::{Matrix3, Matrix4, Vector2};
use wasm_bindgen::prelude::*;
use web_sys::{WebGlProgram, WebGlRenderingContext as GL, WebGlShader};

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

        // context.enable(GL::BLEND);
        // context.blend_equation(GL::FUNC_ADD);
        // context.blend_func(GL::SRC_ALPHA, GL::ONE_MINUS_SRC_ALPHA);

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
                vec4 texColor = texture2D( texture, vec2(texCoords.x, texCoords.y) );
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

        context.disable(GL::BLEND);
        context.disable(GL::DEPTH_TEST);

        // let world_transform = (Matrix4::from_translation(Vector3::new(-1., 1., 0.))
        //     * Matrix4::from_nonuniform_scale(2., -2., 1.)
        //     * Matrix4::from_nonuniform_scale(
        //         2. / self.viewport_width,
        //         -2. / self.viewport_height,
        //         1.,
        //     ))
        // .cast::<f32>()
        // .ok_or_else(|| js_str!("world transform cast failed"))?;

        let back_texture_transform =
            (Matrix3::from_nonuniform_scale(
                self.viewport_width / self.viewport.scale,
                self.viewport_height / self.viewport.scale,
            ) * Matrix3::from_translation(Vector2::new(-self.viewport.x, -self.viewport.y))
                * Matrix3::from_nonuniform_scale(1. / TILE_SIZE, -1. / TILE_SIZE))
            .cast::<f32>()
            .ok_or_else(|| js_str!("world transform cast failed"))?;

        let assets = &self.assets;

        if let Some(shader) = assets.textured_shader.as_ref() {
            context.use_program(Some(&shader.program));
            context.active_texture(GL::TEXTURE0);

            context.uniform1i(shader.texture_loc.as_ref(), 0);

            context.uniform_matrix3fv_with_f32_array(
                shader.tex_transform_loc.as_ref(),
                false,
                <Matrix3<f32> as AsRef<[f32; 9]>>::as_ref(&back_texture_transform),
            );
            context.bind_texture(GL::TEXTURE_2D, Some(&self.assets.back_tex));
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
        } else {
            console_log!("Shader bundle not found!");
        }

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
