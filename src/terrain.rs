use super::{
    perlin_noise::{gen_terms, perlin_noise_pixel, Xor128},
    Cell, Ore, OreValue,
};
use serde::Deserialize;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
#[derive(Deserialize)]
pub(crate) struct TerrainParameters {
    pub width: u32,
    pub height: u32,
    pub terrain_seed: u32,
    pub water_noise_threshold: f64,
    pub resource_amount: f64,
    pub noise_scale: f64,
    pub noise_threshold: f64,
}

pub(crate) fn gen_terrain(params: &TerrainParameters) -> Vec<Cell> {
    let TerrainParameters {
        width,
        height,
        terrain_seed,
        water_noise_threshold,
        resource_amount,
        noise_scale,
        noise_threshold,
    } = *params;
    let mut ret = vec![Cell::default(); (width * height) as usize];
    let bits = 1;
    let mut rng = Xor128::new(terrain_seed);
    let ocean_terms = gen_terms(&mut rng, bits);
    let iron_terms = gen_terms(&mut rng, bits);
    let copper_terms = gen_terms(&mut rng, bits);
    let coal_terms = gen_terms(&mut rng, bits);
    let stone_terms = gen_terms(&mut rng, bits);
    for y in 0..height {
        for x in 0..width {
            let [fx, fy] = [x as f64 / noise_scale, y as f64 / noise_scale];
            let cell = &mut ret[(x + y * width) as usize];
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
    calculate_back_image(&mut ret, width, height);
    ret
}

pub(crate) fn calculate_back_image(ret: &mut [Cell], width: u32, height: u32) {
    let mut rng = Xor128::new(23424321);
    // Some number with fractional part is desirable, but we don't care too precisely since it is just a visual aid.
    let noise_scale = 3.75213;
    let bits = 1;
    let grass_terms = gen_terms(&mut rng, bits);
    for uy in 0..height {
        let y = uy as i32;
        for ux in 0..width {
            let x = ux as i32;
            if ret[(ux + uy * width) as usize].water {
                ret[(ux + uy * width) as usize].image = 15;
                continue;
            }
            let get_at = |x: i32, y: i32| {
                if x < 0 || width as i32 <= x || y < 0 || height as i32 <= y {
                    false
                } else {
                    ret[(x as u32 + y as u32 * width) as usize].water
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
            let cell = &mut ret[(ux + uy * width) as usize];
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
