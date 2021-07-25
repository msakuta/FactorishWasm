use super::{
    perlin_noise::{gen_terms, perlin_noise_pixel, Xor128},
    Cell, Ore, OreValue, Position,
};
use serde::Deserialize;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Deserialize)]
pub(crate) struct TerrainParameters {
    pub width: u32,
    pub height: u32,
    pub unlimited: bool,
    pub terrain_seed: u32,
    pub water_noise_threshold: f64,
    pub resource_amount: f64,
    pub noise_scale: f64,
    pub noise_threshold: f64,
}

pub(crate) const CHUNK_SIZE: usize = 16;
pub(crate) const CHUNK_SIZE_I: i32 = CHUNK_SIZE as i32;
pub(crate) const CHUNK_SIZE_F: f64 = CHUNK_SIZE as f64;
pub(crate) const CHUNK_SIZE2: usize = CHUNK_SIZE * CHUNK_SIZE;

pub(crate) struct Chunk {
    pub cells: Vec<Cell>,
    /// Maintain a buffer for rendering minimap for performance
    pub minimap_buffer: Vec<u8>,
}

impl Chunk {
    pub(crate) fn new(cells: Vec<Cell>) -> Self {
        Self {
            cells,
            minimap_buffer: vec![0u8; CHUNK_SIZE2 * 4],
        }
    }
}

pub(crate) type Chunks = HashMap<Position, Chunk>;

pub(crate) trait ChunksExt {
    fn get_tile(&self, position: Position) -> Option<&Cell>;
    fn get_tile_mut(&mut self, position: Position) -> Option<&mut Cell>;
}

impl ChunksExt for Chunks {
    fn get_tile(&self, position: Position) -> Option<&Cell> {
        let chunk = self.get(&Position::new(
            position.x.div_euclid(CHUNK_SIZE_I),
            position.y.div_euclid(CHUNK_SIZE_I),
        ));
        if let Some(chunk) = chunk {
            chunk.cells.get(
                (position.x.rem_euclid(CHUNK_SIZE_I)
                    + position.y.rem_euclid(CHUNK_SIZE_I) * CHUNK_SIZE_I) as usize,
            )
        } else {
            None
        }
    }

    fn get_tile_mut(&mut self, position: Position) -> Option<&mut Cell> {
        let chunk = self.get_mut(&Position::new(
            position.x.div_euclid(CHUNK_SIZE_I),
            position.y.div_euclid(CHUNK_SIZE_I),
        ));
        if let Some(chunk) = chunk {
            chunk.cells.get_mut(
                (position.x.rem_euclid(CHUNK_SIZE_I)
                    + position.y.rem_euclid(CHUNK_SIZE_I) * CHUNK_SIZE_I) as usize,
            )
        } else {
            None
        }
    }
}

/// Generate a chunk at given position. It does not update background image, because it requires
/// knowledge on connecting chunks. The caller needs to call [`calculate_back_image`] or
/// [`calculate_back_image_all`] at some point.
pub(crate) fn gen_chunk(position: Position, terrain_params: &TerrainParameters) -> Chunk {
    let TerrainParameters {
        terrain_seed,
        water_noise_threshold,
        resource_amount,
        noise_scale,
        noise_threshold,
        ..
    } = *terrain_params;

    let mut ret = vec![Cell::default(); CHUNK_SIZE2];
    let bits = 1;
    let mut rng = Xor128::new(terrain_seed);
    let ocean_terms = gen_terms(&mut rng, bits);
    let iron_terms = gen_terms(&mut rng, bits);
    let copper_terms = gen_terms(&mut rng, bits);
    let coal_terms = gen_terms(&mut rng, bits);
    let stone_terms = gen_terms(&mut rng, bits);
    for y in 0..CHUNK_SIZE {
        for x in 0..CHUNK_SIZE {
            let [fx, fy] = [
                (x as f64 + position.x as f64 * CHUNK_SIZE as f64) / noise_scale,
                (y as f64 + position.y as f64 * CHUNK_SIZE as f64) / noise_scale,
            ];
            let cell = &mut ret[(x + y * CHUNK_SIZE) as usize];
            cell.water = water_noise_threshold < perlin_noise_pixel(fx, fy, bits, &ocean_terms);
            if cell.water {
                continue; // No ores in water
            }
            let iron = (perlin_noise_pixel(fx, fy, bits, &iron_terms) - noise_threshold)
                * 4.
                * resource_amount;
            let copper = (perlin_noise_pixel(fx, fy, bits, &copper_terms) - noise_threshold)
                * 4.
                * resource_amount;
            let coal = (perlin_noise_pixel(fx, fy, bits, &coal_terms) - noise_threshold)
                * 4.
                * resource_amount;
            let stone = (perlin_noise_pixel(fx, fy, bits, &stone_terms) - noise_threshold)
                * 4.
                * resource_amount;

            match [
                (Ore::Iron, iron),
                (Ore::Copper, copper),
                (Ore::Coal, coal),
                (Ore::Stone, stone),
            ]
            .iter()
            .map(|(ore, v)| (ore, v.max(0.) as u32))
            .max_by_key(|v| v.1)
            {
                Some((ore, v)) if 0 < v => cell.ore = Some(OreValue(*ore, v)),
                _ => (),
            }
        }
    }
    Chunk::new(ret)
}

pub(crate) fn gen_terrain(params: &TerrainParameters) -> Chunks {
    let TerrainParameters { width, height, .. } = *params;

    let mut ret = HashMap::new();
    for y in 0..height as usize / CHUNK_SIZE {
        for x in 0..width as usize / CHUNK_SIZE {
            let pos = Position::new(x as i32, y as i32);
            ret.insert(pos, gen_chunk(pos, params));
        }
    }

    calculate_back_image_all(&mut ret);
    ret
}

pub(crate) fn calculate_back_image(terrain: &Chunks, chunk_pos: &Position, ret: &mut Vec<Cell>) {
    let mut rng = Xor128::new(23424321);
    // Some number with fractional part is desirable, but we don't care too precisely since it is just a visual aid.
    let noise_scale = 3.75213;
    let bits = 1;
    let grass_terms = gen_terms(&mut rng, bits);
    for uy in 0..CHUNK_SIZE {
        let y = uy as i32;
        for ux in 0..CHUNK_SIZE {
            let x = ux as i32;
            if ret[(ux + uy * CHUNK_SIZE) as usize].water {
                ret[(ux + uy * CHUNK_SIZE) as usize].image = 15;
                continue;
            }
            let get_at = |x: i32, y: i32| {
                if x < 0 || CHUNK_SIZE as i32 <= x || y < 0 || CHUNK_SIZE as i32 <= y {
                    terrain
                        .get_tile(Position::new(
                            x + chunk_pos.x * CHUNK_SIZE_I,
                            y + chunk_pos.y * CHUNK_SIZE_I,
                        ))
                        .map(|tile| tile.water)
                        .unwrap_or(false)
                } else {
                    ret[(x + y * CHUNK_SIZE_I) as usize].water
                }
            };
            let l = get_at(x - 1, y) as u8;
            let t = get_at(x, y - 1) as u8;
            let r = get_at(x + 1, y) as u8;
            let b = get_at(x, y + 1) as u8;
            let lt = get_at(x - 1, y - 1) as u8;
            let rt = get_at(x + 1, y - 1) as u8;
            let rb = get_at(x + 1, y + 1) as u8;
            let lb = get_at(x - 1, y + 1) as u8;
            let neighbor = l | (t << 1) | (r << 2) | (b << 3);
            let diagonal = lt | (rt << 1) | (rb << 2) | (lb << 3);
            let cell = &mut ret[(ux + uy * CHUNK_SIZE) as usize];
            cell.image = if neighbor != 0 {
                neighbor
            } else if diagonal != 0 {
                diagonal | (1 << 4)
            } else {
                0
            };

            cell.grass_image = ((perlin_noise_pixel(
                x as f64 / noise_scale,
                y as f64 / noise_scale,
                bits,
                &grass_terms,
            ) - 0.)
                * 4.
                * 6.)
                .max(0.)
                .min(6.) as u8;
        }
    }
}

pub(crate) fn calculate_back_image_all(terrain: &mut Chunks) {
    for chunk_pos in &terrain.keys().copied().collect::<Vec<_>>() {
        let mut chunk = std::mem::take(&mut terrain.get_mut(chunk_pos).unwrap().cells);
        calculate_back_image(terrain, chunk_pos, &mut chunk);
        terrain.get_mut(chunk_pos).map(|c| c.cells = chunk);
    }
}
