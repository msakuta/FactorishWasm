use super::{
    assets::{HARVESTING_SEGMENTS, HARVESTING_THICKNESS, MAX_SPRITES, SPRITE_COMPONENTS},
    shader_bundle::ShaderBundle,
    utils::{enable_buffer, vertex_buffer_sub_data, Flatten},
};
use crate::{
    apply_bounds, elect_pole::draw_wire_gl, items::render_drop_item_gl, performance,
    structure::Structure, Cell, FactorishState, FluidType, Ore, OreValue, Position, PowerWire,
    Rotation, Vector2f, CHUNK_SIZE, CHUNK_SIZE_I, DROP_ITEM_SIZE, INDEX_CHUNK_SIZE,
    ORE_HARVEST_TIME, TILE_SIZE, TILE_SIZE_F,
};
use cgmath::{Matrix3, Matrix4, Rad, Vector2, Vector3};
use slice_of_array::SliceFlatExt;
use wasm_bindgen::prelude::*;
use web_sys::{WebGlRenderingContext as GL, WebGlTexture};

pub(crate) fn draw_direction_arrow_gl(
    pos: impl Into<Vector2f>,
    rotation: &Rotation,
    state: &FactorishState,
    gl: &GL,
) -> Result<(), JsValue> {
    let drawer = DirectionDrawer::with_tex(&state.assets.tex_direction);
    drawer.draw(pos, rotation, state, gl)
}

pub(crate) struct DirectionDrawer<'a> {
    tex: &'a WebGlTexture,
}

impl<'a> DirectionDrawer<'a> {
    pub fn with_tex(tex: &'a WebGlTexture) -> Self {
        Self { tex }
    }

    pub fn draw(
        &self,
        pos: impl Into<Vector2f>,
        rotation: &Rotation,
        state: &FactorishState,
        gl: &GL,
    ) -> Result<(), JsValue> {
        let Vector2f { x, y } = pos.into();
        let shader = state
            .assets
            .textured_shader
            .as_ref()
            .ok_or_else(|| js_str!("Shader not found"))?;
        gl.use_program(Some(&shader.program));
        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(self.tex));

        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::from_nonuniform_scale(1., 1.).flatten(),
        );

        gl.uniform_matrix4fv_with_f32_array(
            shader.transform_loc.as_ref(),
            false,
            (state.get_world_transform()?
                * Matrix4::from_scale(2.)
                * Matrix4::from_translation(Vector3::new(x + 0.5, y + 0.5, 0.))
                * Matrix4::from_angle_z(Rad(rotation.angle_rad() as f32 + std::f32::consts::PI))
                * Matrix4::from_nonuniform_scale(0.25, 0.5, 1.)
                * Matrix4::from_translation(Vector3::new(-0.5, -0.5, 0.)))
            .flatten(),
        );

        enable_buffer(&gl, &state.assets.screen_buffer, 2, shader.vertex_position);

        gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

        Ok(())
    }
}

#[macro_export]
macro_rules! draw_fuel_alarm_gl_impl {
    ($self_:expr, $state:expr, $gl:expr) => {
        if $self_.recipe.is_some() && $self_.power == 0. {
            crate::gl::draw_fuel_alarm_gl($self_, $state, $gl)?;
        }
    };
}

pub(crate) fn draw_electricity_alarm_gl(
    (x, y): (f32, f32),
    state: &FactorishState,
    gl: &GL,
) -> Result<(), JsValue> {
    if !(state.sim_time % 1. < 0.5) {
        return Ok(());
    }
    let shader = state
        .assets
        .textured_shader
        .as_ref()
        .ok_or_else(|| js_str!("Shader not found"))?;
    gl.use_program(Some(&shader.program));
    gl.uniform1f(shader.alpha_loc.as_ref(), 1.);
    gl.active_texture(GL::TEXTURE0);
    gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_electricity_alarm));
    gl.uniform_matrix3fv_with_f32_array(
        shader.tex_transform_loc.as_ref(),
        false,
        Matrix3::from_scale(1.).flatten(),
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

pub(crate) fn draw_fuel_alarm_gl(
    this: &dyn Structure,
    state: &FactorishState,
    gl: &GL,
) -> Result<(), JsValue> {
    if state.sim_time % 1. < 0.5 {
        let shader = state
            .assets
            .textured_shader
            .as_ref()
            .ok_or_else(|| js_str!("Shader not found"))?;
        gl.use_program(Some(&shader.program));
        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(&state.assets.tex_fuel_alarm));
        let position = this.bounding_box().center();
        // Subtract 0.5 to bring the sprite to center
        let (x, y) = (
            position.x - 0.5 + state.viewport.x as f32,
            position.y - 0.5 + state.viewport.y as f32,
        );
        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::from_scale(1.).flatten(),
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
    }
    Ok(())
}

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
        gl.uniform1f(shader.alpha_loc.as_ref(), 1.);

        gl.active_texture(GL::TEXTURE0);

        gl.uniform1i(shader.texture_loc.as_ref(), 0);

        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            back_texture_transform.flatten(),
        );
        gl.bind_texture(GL::TEXTURE_2D, Some(&self.assets.tex_dirt));
        enable_buffer(&gl, &self.assets.screen_buffer, 2, shader.vertex_position);
        gl.uniform_matrix4fv_with_f32_array(
            shader.transform_loc.as_ref(),
            false,
            (Matrix4::from_translation(Vector3::new(-1., -1., 0.)) * Matrix4::from_scale(2.))
                .flatten(),
        );
        gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

        if self.use_webgl_instancing && self.assets.instanced_arrays_ext.is_some() {
            self.render_sprites_gl_instancing(&gl)?;
        } else {
            self.render_sprites_gl(&gl, shader)?;
        }

        let draw_structures = |depth| -> Result<(), JsValue> {
            for structure in self.structure_iter() {
                structure.draw_gl(&self, &gl, depth, false)?;
            }
            Ok(())
        };

        draw_structures(0)?;

        for item in self.drop_items.iter() {
            render_drop_item_gl(self, &gl, &item.type_, item.x, item.y)?;
        }

        if let Some(shader) = self.assets.flat_shader.as_ref() {
            gl.use_program(Some(&shader.program));
            enable_buffer(&gl, &self.assets.wire_buffer, 2, shader.vertex_position);

            gl.uniform_matrix4fv_with_f32_array(
                shader.transform_loc.as_ref(),
                false,
                (self.get_world_transform()?
                    * Matrix4::from_scale(2.)
                    * Matrix4::from_translation(Vector3::new(
                        self.viewport.x as f32,
                        self.viewport.y as f32,
                        0.,
                    ))
                    * Matrix4::from_scale(1. / TILE_SIZE as f32))
                .flatten(),
            );

            let draw_wires = |wires: &[PowerWire], width: f32| -> Result<(), JsValue> {
                for PowerWire(first, second) in wires {
                    let first = self.get_structure(*first);
                    let second = self.get_structure(*second);
                    if let Some((first, second)) = first.zip(second) {
                        let first = *first.position();
                        let second = *second.position();
                        let min = (first.x.min(second.x), first.y.min(second.y));
                        let max = (first.x.max(second.x), first.y.max(second.y));
                        if -self.viewport.x <= max.0 as f64
                            && min.0 as f64
                                <= -self.viewport.x
                                    + self.viewport_width / self.viewport.scale / TILE_SIZE
                            && -self.viewport.y <= max.1 as f64
                            && min.1 as f64
                                <= -self.viewport.y
                                    + self.viewport_height / self.viewport.scale / TILE_SIZE
                        {
                            draw_wire_gl(&gl, first, second, width)?;
                        }
                    }
                }
                Ok(())
            };

            if self.debug_power_network {
                let colors = [[1., 0., 0., 1.], [0., 0., 1., 1.], [0., 1., 0., 1.]];
                for (i, nw) in self.power_networks.iter().enumerate() {
                    gl.uniform4fv_with_f32_array(
                        shader.color_loc.as_ref(),
                        &colors[i % colors.len()],
                    );

                    draw_wires(&nw.wires, 2.)?;
                }
            }

            gl.uniform4fv_with_f32_array(shader.color_loc.as_ref(), &[0.75, 0.5, 0., 1.]);

            draw_wires(&self.power_wires, 1.)?;
        }

        draw_structures(1)?;
        draw_structures(2)?;

        // Smoke rendering
        if let Some(shader) = self.assets.textured_alpha_shader.as_ref() {
            for ent in &self.temp_ents {
                let (x, y) = (ent.position.0, ent.position.1);
                gl.use_program(Some(&shader.program));
                gl.uniform1f(
                    shader.alpha_loc.as_ref(),
                    ((ent.max_life - ent.life).min(ent.life) * 0.15).min(0.35) as f32,
                );

                gl.uniform_matrix3fv_with_f32_array(
                    shader.tex_transform_loc.as_ref(),
                    false,
                    (Matrix3::from_scale(0.5) * Matrix3::from_translation(Vector2::new(-1., -1.)))
                        .flatten(),
                );

                let scale = ((ent.max_life - ent.life) * 0.3).min(2.) as f32;
                gl.uniform_matrix4fv_with_f32_array(
                    shader.transform_loc.as_ref(),
                    false,
                    (self.get_world_transform()?
                        * Matrix4::from_translation(Vector3::new(
                            2. * (self.viewport.x + x / TILE_SIZE) as f32,
                            2. * (self.viewport.y + y / TILE_SIZE) as f32,
                            0.,
                        ))
                        * Matrix4::from_angle_z(Rad(ent.rotation as f32))
                        * Matrix4::from_scale(scale))
                    .flatten(),
                );

                gl.bind_texture(GL::TEXTURE_2D, Some(&self.assets.tex_smoke));
                enable_buffer(&gl, &self.assets.rect_buffer, 2, shader.vertex_position);
                gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);
            }
        }

        let set_transform =
            |shader: &ShaderBundle, trans: (f32, f32), scale: (f32, f32)| -> Result<(), JsValue> {
                gl.uniform_matrix4fv_with_f32_array(
                    shader.transform_loc.as_ref(),
                    false,
                    (self.get_world_transform()?
                        * Matrix4::from_translation(Vector3::new(
                            2. * (self.viewport.x as f32 + trans.0),
                            2. * (self.viewport.y as f32 + trans.1),
                            0.,
                        ))
                        * Matrix4::from_nonuniform_scale(2. * scale.0, 2. * scale.1, 1.))
                    .flatten(),
                );
                Ok(())
            };

        if self.debug_bbox {
            if let Some(shader) = &self.assets.flat_shader {
                gl.use_program(Some(&shader.program));
                enable_buffer(&gl, &self.assets.screen_buffer, 2, shader.vertex_position);
                gl.uniform4fv_with_f32_array(shader.color_loc.as_ref(), &[1., 0., 0., 1.]);
                for structure in self.structure_iter() {
                    let bb = structure.bounding_box();
                    set_transform(
                        shader,
                        (bb.x0 as f32, bb.y0 as f32),
                        ((bb.x1 - bb.x0) as f32, (bb.y1 - bb.y0) as f32),
                    )?;
                    gl.draw_arrays(GL::LINE_LOOP, 0, 4);
                }
                gl.uniform4fv_with_f32_array(shader.color_loc.as_ref(), &[1., 0., 1., 1.]);
                for item in self.drop_items.iter() {
                    set_transform(
                        shader,
                        (
                            (item.x as f32 - DROP_ITEM_SIZE as f32 / 2.) / TILE_SIZE_F,
                            (item.y as f32 - DROP_ITEM_SIZE as f32 / 2.) / TILE_SIZE_F,
                        ),
                        (
                            DROP_ITEM_SIZE as f32 / TILE_SIZE_F,
                            DROP_ITEM_SIZE as f32 / TILE_SIZE_F,
                        ),
                    )?;
                    gl.draw_arrays(GL::LINE_LOOP, 0, 4);
                }
                gl.uniform4fv_with_f32_array(shader.color_loc.as_ref(), &[0., 0., 0., 1.]);
                for chunk in self.board.keys() {
                    set_transform(
                        shader,
                        (
                            chunk.x as f32 * INDEX_CHUNK_SIZE as f32,
                            chunk.y as f32 * INDEX_CHUNK_SIZE as f32,
                        ),
                        (INDEX_CHUNK_SIZE as f32, INDEX_CHUNK_SIZE as f32),
                    )?;
                    gl.draw_arrays(GL::LINE_LOOP, 0, 4);
                }
            }
        }

        if self.debug_fluidbox {
            if let Some(shader) = &self.assets.flat_shader {
                gl.use_program(Some(&shader.program));
                enable_buffer(&gl, &self.assets.screen_buffer, 2, shader.vertex_position);

                for structure in self.structure_iter() {
                    if let Some(fluid_boxes) = structure.fluid_box() {
                        let bb = structure.bounding_box();
                        for (i, fb) in fluid_boxes.iter().enumerate() {
                            const BAR_MARGIN: f32 = 0.15;
                            const BAR_WIDTH: f32 = 0.15;

                            let frame_trans = (
                                bb.x0 as f32 + 0.2 * i as f32 + BAR_MARGIN,
                                bb.y0 as f32 + BAR_MARGIN,
                            );
                            let frame_size = (BAR_WIDTH, (bb.y1 - bb.y0) as f32 - BAR_MARGIN * 2.);

                            set_transform(shader, frame_trans, frame_size)?;
                            gl.uniform4fv_with_f32_array(
                                shader.color_loc.as_ref(),
                                &[0., 0., 0., 1.],
                            );
                            gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

                            let bar_height = (fb.amount / fb.max_amount) as f32
                                * ((bb.y1 - bb.y0) as f32 - BAR_MARGIN * 2.);
                            set_transform(
                                shader,
                                (frame_trans.0, frame_trans.1 + frame_size.1 - bar_height),
                                (BAR_WIDTH, bar_height),
                            )?;
                            gl.uniform4fv_with_f32_array(
                                shader.color_loc.as_ref(),
                                match fb.type_ {
                                    Some(FluidType::Water) => &[0., 1., 1., 1.],
                                    Some(FluidType::Steam) => &[0.75, 0.75, 0.75, 1.],
                                    _ => &[0.5, 0.5, 0.5, 1.],
                                },
                            );
                            gl.draw_arrays(GL::TRIANGLE_FAN, 0, 4);

                            set_transform(shader, frame_trans, frame_size)?;
                            gl.uniform4fv_with_f32_array(
                                shader.color_loc.as_ref(),
                                &[1., 0., 0., 1.],
                            );
                            gl.draw_arrays(GL::LINE_LOOP, 0, 4);
                        }
                    }
                }
            }
        }

        if let Some((ref cursor, shader)) = self.cursor.zip(self.assets.flat_shader.as_ref()) {
            let (x, y) = (cursor[0] as f32, cursor[1] as f32);

            if let Some((selected_tool, _)) = self.get_selected_tool_or_item_opt() {
                if let Ok(mut tool) = self.new_structure(&selected_tool, &Position::from(cursor)) {
                    tool.set_rotation(&self.tool_rotation).ok();
                    for depth in 0..3 {
                        tool.draw_gl(self, &gl, depth, true)?;
                    }
                }
            }

            gl.use_program(Some(&shader.program));
            gl.uniform4fv_with_f32_array(shader.color_loc.as_ref(), &[0., 0., 1., 1.]);
            gl.uniform_matrix4fv_with_f32_array(
                shader.transform_loc.as_ref(),
                false,
                (self.get_world_transform()?
                    * Matrix4::from_translation(Vector3::new(
                        2. * (self.viewport.x as f32 + x) + 1.,
                        2. * (self.viewport.y as f32 + y) + 1.,
                        0.,
                    )))
                .flatten(),
            );
            enable_buffer(&gl, &self.assets.cursor_buffer, 2, shader.vertex_position);
            gl.draw_arrays(GL::TRIANGLE_STRIP, 0, 10);
        }

        if let Some((ore_harvesting, shader)) =
            &self.ore_harvesting.zip(self.assets.flat_shader.as_ref())
        {
            gl.use_program(Some(&shader.program));
            gl.uniform4fv_with_f32_array(shader.color_loc.as_ref(), &[1., 0.5, 1., 1.]);

            gl.uniform_matrix4fv_with_f32_array(
                shader.transform_loc.as_ref(),
                false,
                (self.get_world_transform()?
                    * Matrix4::from_translation(Vector3::new(
                        2. * (self.viewport.x as f32 + ore_harvesting.pos.x as f32) + 1.,
                        2. * (self.viewport.y as f32 + ore_harvesting.pos.y as f32) + 1.,
                        0.,
                    )))
                .flatten(),
            );

            let mut points = [[0.; 4]; HARVESTING_SEGMENTS];
            let max_angle =
                ore_harvesting.timer as f32 / ORE_HARVEST_TIME as f32 * 2. * std::f32::consts::PI;
            for i in 0..HARVESTING_SEGMENTS {
                let angle = i as f32 * max_angle / HARVESTING_SEGMENTS as f32;
                points[i as usize] = [
                    angle.sin() * (1. + HARVESTING_THICKNESS),
                    -angle.cos() * (1. + HARVESTING_THICKNESS),
                    angle.sin() * (1. - HARVESTING_THICKNESS),
                    -angle.cos() * (1. - HARVESTING_THICKNESS),
                ];
            }
            enable_buffer(
                &gl,
                &self.assets.harvesting_buffer,
                2,
                shader.vertex_position,
            );

            vertex_buffer_sub_data(&gl, &points.flat());

            gl.draw_arrays(GL::TRIANGLE_STRIP, 0, points.len() as i32 * 2);
        }

        self.perf_render.add(performance().now() - start_render);

        Ok(())
    }

    pub(crate) fn get_world_transform(&self) -> Result<Matrix4<f32>, JsValue> {
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
            &self.assets.screen_buffer,
            2,
            shader.vertex_position,
        );

        let apply_transform = |x, y| {
            context.uniform_matrix4fv_with_f32_array(
                shader.transform_loc.as_ref(),
                false,
                (world_transform
                    * Matrix4::from_scale(2.)
                    * Matrix4::from_translation(
                        Vector3::new(self.viewport.x + x as f64, self.viewport.y + y as f64, 0.)
                            .cast::<f32>()
                            .unwrap(),
                    ))
                .flatten(),
            );
        };

        let apply_texture_transform = |scale_x, scale_y, trans_x, trans_y| {
            context.uniform_matrix3fv_with_f32_array(
                shader.tex_transform_loc.as_ref(),
                false,
                (Matrix3::from_nonuniform_scale(scale_x, scale_y)
                    * Matrix3::from_translation(Vector2::new(trans_x, trans_y)))
                .flatten(),
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
                apply_texture_transform(1. / 8., 1., cell.grass_image as f32 + 1., 0.);
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
}

struct InstancingStats {
    wraps: i32,
    floats: usize,
}

impl FactorishState {
    /// Render particles if the device supports instancing. It is much faster with fewer calls to the API.
    /// Some devices may not support it, so we have a fallback function [`render_sprites_gl`], but I guess
    /// almost all modern devices do.
    fn render_sprites_gl_instancing(&self, gl: &GL) -> Result<(), JsValue> {
        // We reserve the buffer with possible maximum size since we almost always use them and the case we would
        // like to optimize is when there are a lot of sprites, so it is a good investment to pre-allocate buffer.
        // Also, if we reserve the same size every frame, it is more likely that the allocator will put it in the
        // same memory address and CPU cache can utilize it.
        let mut instance_buf = Vec::with_capacity(MAX_SPRITES * SPRITE_COMPONENTS);
        let mut stats = InstancingStats {
            wraps: 0,
            floats: 0,
        };
        self.render_repeat_gl_instancing(
            &gl,
            1. / 4.,
            1. / 8.,
            &self.assets.tex_back,
            |x, y, cell, instance_buf| {
                if cell.image != 0 {
                    instance_buf.push(x as f32);
                    instance_buf.push(y as f32);
                    instance_buf.push((cell.image % 4) as f32);
                    instance_buf.push((cell.image / 4) as f32);
                }
            },
            &mut instance_buf,
            &mut stats,
        )?;

        self.render_repeat_gl_instancing(
            &gl,
            1. / 8.,
            1.,
            &self.assets.tex_weeds,
            |x, y, cell, instance_buf| {
                if cell.grass_image == 0 {
                    return;
                }

                instance_buf.push(x as f32);
                instance_buf.push(y as f32);
                instance_buf.push(cell.grass_image as f32 + 1.);
                instance_buf.push(0.);
            },
            &mut instance_buf,
            &mut stats,
        )?;

        for (ore_type, tex) in [
            (Ore::Iron, &self.assets.tex_iron),
            (Ore::Copper, &self.assets.tex_copper),
            (Ore::Coal, &self.assets.tex_coal),
            (Ore::Stone, &self.assets.tex_stone),
        ]
        .iter()
        {
            self.render_repeat_gl_instancing(
                &gl,
                1. / 4.,
                1.,
                tex,
                |x, y, cell, instance_buf| {
                    if let Some(OreValue(ot, ore)) = cell.ore {
                        if ot == *ore_type {
                            let idx = (ore / 10).min(3);
                            instance_buf.push(x as f32);
                            instance_buf.push(y as f32);
                            instance_buf.push(idx as f32);
                            instance_buf.push(0.);
                        }
                    }
                },
                &mut instance_buf,
                &mut stats,
            )?;
        }

        // console_log!("drawn {} wraps {} floats", stats.wraps, stats.floats);

        Ok(())
    }

    fn render_repeat_gl_instancing(
        &self,
        gl: &GL,
        scale_x: f32,
        scale_y: f32,
        texture: &WebGlTexture,
        get_cell: impl Fn(i32, i32, &Cell, &mut Vec<f32>),
        instance_buf: &mut Vec<f32>,
        stats: &mut InstancingStats,
    ) -> Result<(), JsValue> {
        let shader = self
            .assets
            .textured_instancing_shader
            .as_ref()
            .ok_or_else(|| JsValue::from_str("Could not find textured_instancing_shader"))?;
        if shader.attrib_position_loc < 0 {
            return Err(JsValue::from_str("matrix location was not found"));
        }

        gl.use_program(Some(&shader.program));

        gl.uniform_matrix3fv_with_f32_array(
            shader.tex_transform_loc.as_ref(),
            false,
            Matrix3::from_nonuniform_scale(scale_x, scale_y).flatten(),
        );

        gl.active_texture(GL::TEXTURE0);
        gl.bind_texture(GL::TEXTURE_2D, Some(texture));

        instance_buf.clear();

        self.render_cells(
            |x, y, cell| {
                get_cell(x, y, cell, instance_buf);
                if MAX_SPRITES * SPRITE_COMPONENTS <= instance_buf.len() {
                    self.render_instances_with_buffer(&gl, &shader, instance_buf)?;
                    stats.wraps += 1;
                    stats.floats += instance_buf.len();
                    instance_buf.clear();
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
        if !instance_buf.is_empty() {
            self.render_instances_with_buffer(&gl, &shader, instance_buf)?;
            stats.floats += instance_buf.len();
            stats.wraps += 1;
        }

        Ok(())
    }

    /// Run a single pass of rendering with instance buffer.
    /// Note that there are no loops at all in this function.
    fn render_instances_with_buffer(
        &self,
        gl: &GL,
        shader: &ShaderBundle,
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

        gl.uniform_matrix4fv_with_f32_array(
            shader.transform_loc.as_ref(),
            false,
            world_transform.flatten(),
        );

        enable_buffer(gl, &self.assets.screen_buffer, 2, shader.vertex_position);

        gl.bind_buffer(GL::ARRAY_BUFFER, self.assets.sprites_buffer.as_ref());
        vertex_buffer_sub_data(
            gl,
            &sprites_buf[..sprites_buf.len().min(MAX_SPRITES * SPRITE_COMPONENTS)],
        );

        let stride = SPRITE_COMPONENTS as i32 * 4;
        gl.vertex_attrib_pointer_with_i32(
            shader.attrib_position_loc as u32,
            SPRITE_COMPONENTS as i32,
            GL::FLOAT,
            false,
            stride,
            0,
        );

        instanced_arrays_ext.vertex_attrib_divisor_angle(shader.attrib_position_loc as u32, 1);
        gl.enable_vertex_attrib_array(shader.attrib_position_loc as u32);

        instanced_arrays_ext.draw_arrays_instanced_angle(
            GL::TRIANGLE_FAN,
            0, // offset
            4, // num vertices per instance
            (sprites_buf.len().min(MAX_SPRITES * SPRITE_COMPONENTS) / SPRITE_COMPONENTS) as i32, // num instances
        )?;

        // console_log!("drawn {} instances: {:?}", sprites_buf.len(), &sprites_buf[..10]);

        Ok(())
    }
}
