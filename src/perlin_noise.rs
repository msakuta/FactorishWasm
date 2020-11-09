pub fn perlin_noise_pixel(x: f64, y: f64, bit: u32) -> f64 {
    let mut sum = 0.;
    let [mut maxv, mut f] = [0., 1.];
    let persistence = 0.5;
    for i in (0..2).rev() {
        let cell = 1 << i;
        let fcell = cell as f64;
        let (a00, a01, a10, a11, fx, fy);
        a00 = noise_pixel(x / fcell, y / fcell, bit);
        a01 = noise_pixel(x / fcell, y / fcell + 1., bit);
        a10 = noise_pixel(x / fcell + 1., y / fcell, bit);
        a11 = noise_pixel(x / fcell + 1., y / fcell + 1., bit);
        fx = (x % fcell) / fcell;
        fy = (y % fcell) / fcell;
        sum += ((a00 * (1. - fx) + a10 * fx) * (1. - fy) + (a01 * (1. - fx) + a11 * fx) * fy) * f;
        maxv += f;
        f *= persistence;
    }
    return sum / maxv;
}

fn noise_pixel(x: f64, y: f64, bit: u32) -> f64 {
    // Use CRC32 to distribute influence of input vector [x, y] uniformly
    // to the RNG state vector.
    let seed = crc32_gp(&[x.floor() as u32, y.floor() as u32, bit]);
    let mut rs = Xor128::new(seed);
    return rs.next();
}

const fn make_crc_table() -> [u8; 256] {
    let mut crc_table = [0u8; 256];
    let mut n = 0;
    while n < 256 {
        let mut c = n;
        let mut k = 0;
        while k < 8 {
            c = if c & 1 != 0 {
                0xEDB88320 ^ (c >> 1)
            } else {
                c >> 1
            };
            k += 1;
        }
        crc_table[n] = c as u8;
        n += 1;
    }
    crc_table
}

fn crc32_gp(str: &[u32]) -> u32 {
    const table: [u8; 256] = make_crc_table();
    let mut crc = 0xffffffff; //0 ^ (-1);

    for i in 0..str.len() {
        let val = str[i];
        for j in 0..4 {
            crc = (crc >> 8) ^ table[((crc ^ (val >> j)) & 0xFF) as usize] as u32;
        }
    }

    crc ^ 0xffffffff
}
struct Xor128 {
    x: u32,
}

impl Xor128 {
    fn new(seed: u32) -> Self {
        let mut ret = Xor128 { x: 2463534242 };
        if 0 < seed {
            ret.x ^= seed;
            ret.nexti();
        }
        ret.nexti();
        ret
    }

    fn nexti(&mut self) -> u32 {
        // T = (I + L^a)(I + R^b)(I + L^c)
        // a = 13, b = 17, c = 5
        let x1 = self.x ^ (self.x << 13);
        let x2 = x1 ^ (x1 >> 17);
        self.x = x2 ^ (x2 << 5);
        self.x
    }

    fn next(&mut self) -> f64 {
        self.nexti() as f64 / 0xffffffffu32 as f64
    }
}

mod test {
    #[test]
    fn test_perlin_noise() {
        assert!(super::perlin_noise_pixel(3., 5., 3).is_finite());
        assert_eq!(super::perlin_noise_pixel(3., 5., 3), 0.32818058878063394);
    }

    #[test]
    fn test_noise_pixel() {
        assert_eq!(super::noise_pixel(3., 5., 3), 0.5978840514081261);
    }
}
