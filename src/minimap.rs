use super::{
    performance,
    structure::Position,
    terrain::{Chunk, Chunks, CHUNK_SIZE, CHUNK_SIZE_F, CHUNK_SIZE_I},
    FactorishState,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen::Clamped;
use web_sys::ImageData;

struct ImageBuffer<'a> {
    buf: &'a mut [u8],
    width: usize,
    height: usize,
}

fn copy_rect(dest: &mut ImageBuffer, src: &ImageBuffer, x: i32, y: i32) {
    let x0 = x.min(dest.width as i32).max(0) as usize;
    let x1 = (x + src.width as i32).min(dest.width as i32).max(0) as usize;
    let sx0 = (x0 as i32 - x).min(src.width as i32).max(0) as usize;
    if x0 == x1 {
        return;
    }
    let sx1 = (sx0 + (x1 - x0)).min(src.width).max(0) as usize;
    let y0 = y.min(dest.height as i32).max(0);
    let sy0 = (y0 - y) as usize;
    let y0 = y0 as usize;
    let y1 = (y + src.height as i32).min(dest.height as i32).max(0) as usize;
    if y0 == y1 {
        return;
    }
    // let dblen = dest.buf.len();
    // let sblen = src.buf.len();
    // console_log!("src: {} y: {} y1: {} sx0 {}, sx1 {}", src.height, y, y1, sx0, sx1);
    for y in y0..y1 {
        let sy = y - y0 + sy0;
        let d0 = (x0 + y * dest.width) * 4;
        let d1 = (x1 + y * dest.width) * 4;
        let s0 = (sx0 + sy * src.width) * 4;
        let s1 = (sx1 + sy * src.width) * 4;
        // console_log!("y {}: d [{}] {}..{}/{} s [{}] {}..{}/{}", y,
        //     d1 - d0, d0, d1, dblen,
        //     s1 - s0, s0, s1, sblen);
        let d = &mut dest.buf[d0..d1];
        let s = &src.buf[s0..s1];
        if d.len() != s.len() {
            panic!("d {} s {}", d.len(), s.len());
        }
        d.copy_from_slice(s);
    }
}

#[wasm_bindgen]
impl FactorishState {
    pub(crate) fn render_minimap_data(&mut self) -> Result<(), JsValue> {
        let mut chunks = std::mem::take(&mut self.board);
        let mut painted = 0;
        for (chunk_pos, chunk) in &mut chunks {
            painted += self.render_minimap_chunk(chunk_pos, chunk);
        }
        self.board = chunks;

        console_log!("painted {}", painted);

        Ok(())
    }

    pub(crate) fn render_minimap_chunk(&self, chunk_pos: &Position, chunk: &mut Chunk) -> usize {
        let mut painted = 0;
        let data = &mut chunk.minimap_buffer;

        for y in 0..CHUNK_SIZE_I {
            for x in 0..CHUNK_SIZE_I {
                let cell_pos = Position::new(x, y);
                if let Some(cell) = chunk.cells.get((x + y * CHUNK_SIZE_I) as usize) {
                    let start = ((cell_pos.x + cell_pos.y * CHUNK_SIZE_I) * 4) as usize;
                    data[start + 3] = 255;
                    let color = Self::color_of_cell(&cell);
                    data[start..start + 3].copy_from_slice(&color);
                    painted += 1;
                }
            }
        }

        // context.set_fill_style(&JsValue::from_str("#00ff7f"));
        let color = [0x00, 0xff, 0x7f];
        for structure in self.structure_iter() {
            let Position { x, y } = *structure.position();
            if chunk_pos.x * CHUNK_SIZE_I <= x
                && x < (chunk_pos.x + 1) * CHUNK_SIZE_I
                && chunk_pos.y * CHUNK_SIZE_I <= y
                && y < (chunk_pos.y + 1) * CHUNK_SIZE_I
            {
                let start = ((x.rem_euclid(CHUNK_SIZE_I)
                    + y.rem_euclid(CHUNK_SIZE_I) * CHUNK_SIZE_I)
                    * 4) as usize;
                data[start..start + 3].copy_from_slice(&color);
            }
        }

        painted
    }

    pub(crate) fn render_minimap_data_pixel(&self, chunks: &mut Chunks, position: &Position) {
        let color = self
            .structures
            .iter()
            .find(|structure| {
                structure
                    .dynamic
                    .as_deref()
                    .map(|s| *s.position() == *position)
                    .unwrap_or(false)
            })
            .map(|_| [0x00, 0xff, 0x7f])
            .or_else(|| {
                self.tile_at(position)
                    .map(|cell| Self::color_of_cell(&cell))
            })
            .unwrap_or([0x7f, 0x7f, 0x7f]);
        let (chunk_pos, cell_pos) = position.div_mod(CHUNK_SIZE_I);
        if let Some(chunk) = chunks.get_mut(&chunk_pos) {
            let start = ((cell_pos.x + cell_pos.y * CHUNK_SIZE_I) * 4) as usize;
            if start + 3 < chunk.minimap_buffer.len() {
                chunk.minimap_buffer[start..start + 3].copy_from_slice(&color);
            }
        }
    }

    /// Instead of rendering the minimap on the canvas context, it will return a ImageData
    /// that can be used to construct ImageBitmap on JS side, because dealing with promise in Rust
    /// code is cumbersome.
    pub fn render_minimap(
        &mut self,
        minimap_width: u32,
        minimap_height: u32,
    ) -> Result<ImageData, JsValue> {
        let start_render = performance().now();

        let vp = self.get_viewport();
        let data = &mut self.minimap_buffer;
        if data.len() != (minimap_width * minimap_height * 4) as usize {
            *data = vec![0u8; (minimap_width * minimap_height * 4) as usize];
        }

        data.fill(0);
        let mut data_buf = ImageBuffer {
            buf: data.as_mut(),
            width: minimap_width as usize,
            height: minimap_height as usize,
        };
        for (pos, chunk) in &mut self.board {
            let src = ImageBuffer {
                buf: &mut chunk.minimap_buffer,
                width: CHUNK_SIZE,
                height: CHUNK_SIZE,
            };
            copy_rect(
                &mut data_buf,
                &src,
                pos.x * CHUNK_SIZE_I + self.viewport.x as i32 + minimap_width as i32 / 2
                    - (vp.0 / CHUNK_SIZE_F) as i32 / 4,
                pos.y * CHUNK_SIZE_I + self.viewport.y as i32 + minimap_height as i32 / 2
                    - (vp.1 / CHUNK_SIZE_F) as i32 / 4,
            );
        }
        let image_data = ImageData::new_with_u8_clamped_array_and_sh(
            Clamped::<_>(&mut *data),
            minimap_width as u32,
            minimap_height as u32,
        )?;
        self.perf_minimap.add(performance().now() - start_render);
        return Ok(image_data);
    }
}
