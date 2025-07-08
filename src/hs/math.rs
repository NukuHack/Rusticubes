// might need more of these small files for math and other extra shit 
use std::time::{SystemTime, UNIX_EPOCH};

// Fast random number generator (xorshift32)
pub struct Rand {
    state: u32,
}
#[allow(dead_code)]
impl Rand {
    #[inline]
    pub fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    #[inline]
    pub fn from_time() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        Self::new(seed)
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        self.state
    }

    #[inline]
    fn next_basic_u32(&mut self) -> u32 {
        // Simple LCG random number generator
        self.state = self.state.wrapping_mul(1664525).wrapping_add(1013904223);
        self.state
    }

    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }

    #[inline]
    pub fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        min + self.next_f32() * (max - min)
    }
}

// Convenience functions
#[inline]
pub fn random_float(min: f32, max: f32) -> f32 {
    Rand::from_time().range_f32(min, max)
}

#[inline]
pub fn random_float_seeded(seed: u32, min: f32, max: f32) -> f32 {
    Rand::new(seed).range_f32(min, max)
}


const GRADIENTS_2D_F: [[i16; 2]; 8] = [
    [1, 1], [-1, 1], [1, -1], [-1, -1],
    [1, 0], [-1, 0], [0, 1], [0, -1],
];
const SCALE: i16 = 1 << 8;
const HALF_I32: i32 = i32::MAX / 2;

pub struct PerlInt {
    seed: u32,
    permutations: [u8; 512],
    offset: i32,
}

impl PerlInt {
    pub fn new(seed: u32) -> Self {
        let mut rng = Rand::new(seed);
        let mut permutations = [0u8; 512];
        
        // Initialize with sequential values
        let mut temp_perm = [0u8; 256];
        for i in 0..256 {
            temp_perm[i] = i as u8;
        }
        
        // Fisher-Yates shuffle
        for i in (1..256).rev() {
            let j = (rng.next_u32() % (i as u32 + 1)) as usize;
            temp_perm.swap(i, j);
        }
        
        // Duplicate the permutation table
        permutations[..256].copy_from_slice(&temp_perm);
        permutations[256..].copy_from_slice(&temp_perm);

        let offset = HALF_I32.wrapping_mul(seed as i32) >> 10;
        
        Self { seed, permutations, offset }
    }
    
    #[inline]
    fn lerp(a: i16, b: i16, t: i16) -> i16 {
        let a = a as i32;
        let b = b as i32;
        let t = t as i32;
        (a + ((b - a) * t) / 256) as i16
    }

    #[inline]
    fn fade(t: i16) -> i16 {
        let t = t.clamp(0, 255) as i32;
        let t6 = t * t / 256 * t / 256 * t / 256 * t / 256 * 6;
        let t15 = t * t / 256 * t / 256 * t / 256 * 15;
        let t10 = t * t / 256 * t / 256 * 10;
        ((t6 - t15 + t10) / 256) as i16
    }
    
    #[inline]
    fn grad(hash: u8, x: i16, y: i16) -> i16 {
        let grad = GRADIENTS_2D_F[(hash as usize) % GRADIENTS_2D_F.len()];
        grad[0] * x + grad[1] * y
    }
    
    #[inline]
    fn hash(&self, x: i32, y: i32) -> u8 {
        // Combine coordinates with seed using a simple hash
        let mut h = self.seed.wrapping_add(x as u32);
        h = h.wrapping_mul(0x9e3779b9);
        h ^= y as u32;
        h = h.wrapping_mul(0x9e3779b9);
        h = h ^ (h >> 16);
        self.permutations[h as usize % 512]
    }
    
    pub fn noise_2d(&self, x: i32, y: i32) -> i16 {
        let x = x.wrapping_add(self.offset);
        let y = y.wrapping_add(-self.offset);
        // Get integer and fractional parts
        let xi = x >> 8;
        let yi = y >> 8;        
        let xf = (x & (SCALE as i32 - 1)) as i16;
        let yf = (y & (SCALE as i32 - 1)) as i16;
        
        // Compute fade curves
        let u = Self::fade(xf);
        let v = Self::fade(yf);
        
        // Hash coordinates using our better hash function
        let aa = self.hash(xi, yi);
        let ab = self.hash(xi, yi + 1);
        let ba = self.hash(xi + 1, yi);
        let bb = self.hash(xi + 1, yi + 1);
        
        // Compute dot products
        let g1 = Self::grad(aa, xf, yf);
        let g2 = Self::grad(ba, xf - SCALE, yf);
        let g3 = Self::grad(ab, xf, yf - SCALE);
        let g4 = Self::grad(bb, xf - SCALE, yf - SCALE);
        
        // Interpolate
        let l1 = Self::lerp(g1, g2, u);
        let l2 = Self::lerp(g3, g4, u);
        let result = Self::lerp(l1, l2, v);
        
        // Scale to 0-255 range
        (result / 3).abs().clamp(0, 255)
    }




    // from here down the code is copied from https://github.com/Auburn/FastNoiseLite/blob/master/Rust/src/lib.rs

    
    pub fn noise_2d_f(&self, x: i32, y: i32) -> f32 {
        let x0 = x;
        let y0 = y;

        let xd0:f32 = (x as f32)*(0.33) - x0 as f32;
        let yd0:f32 = (y as f32)*(0.33) - y0 as f32;
        let xd1 = xd0 - 1.;
        let yd1 = yd0 - 1.;

        let xs = Self::interp_quintic(xd0);
        let ys = Self::interp_quintic(yd0);

        let x0 = x0.wrapping_mul(Self::PRIME_X);
        let y0 = y0.wrapping_mul(Self::PRIME_Y);
        let x1 = x0.wrapping_add(Self::PRIME_X);
        let y1 = y0.wrapping_add(Self::PRIME_Y);

        let xf0 = Self::lerp_f(
            Self::grad_coord_f(self.seed as i32, x0, y0, xd0, yd0),
            Self::grad_coord_f(self.seed as i32, x1, y0, xd1, yd0),
            xs,
        );
        let xf1 = Self::lerp_f(
            Self::grad_coord_f(self.seed as i32, x0, y1, xd0, yd1),
            Self::grad_coord_f(self.seed as i32, x1, y1, xd1, yd1),
            xs,
        );

        Self::lerp_f(xf0, xf1, ys) * 1.4247691104677813
    }

    #[inline(always)]
    fn interp_quintic(t: f32) -> f32 {
        t * t * t * (t * (t * 6. - 15.) + 10.)
    }
    #[inline(always)]
    fn grad_coord_f(seed: i32, x_primed: i32, y_primed: i32, xd: f32, yd: f32) -> f32 {
        let hash = (seed ^ x_primed ^ y_primed).wrapping_mul(0x27d4eb2d);
        let hash = (hash ^ (hash >> 15)) & (127 << 1);
        let xg = GRADIENTS_2D[hash as usize];
        let yg = GRADIENTS_2D[(hash | 1) as usize];

        xd * xg + yd * yg
    }
    #[inline(always)]
    fn lerp_f(a: f32, b: f32, t: f32) -> f32 {
        a + t * (b - a)
    }
    const PRIME_X: i32 = 501125321;
    const PRIME_Y: i32 = 1136930381;
}


#[rustfmt::skip]
const GRADIENTS_2D: [f32; 256] = [
     0.130526192220052,  0.99144486137381,   0.38268343236509,   0.923879532511287,  0.608761429008721,  0.793353340291235,  0.793353340291235,  0.608761429008721,
     0.923879532511287,  0.38268343236509,   0.99144486137381,   0.130526192220051,  0.99144486137381,   0.130526192220051,  0.923879532511287,  0.38268343236509,
     0.793353340291235,  0.60876142900872,   0.608761429008721,  0.793353340291235,  0.38268343236509,   0.923879532511287,  0.130526192220052,  0.99144486137381,
     0.130526192220052,  0.99144486137381,   0.38268343236509,   0.923879532511287,  0.608761429008721,  0.793353340291235,  0.793353340291235,  0.608761429008721,
     0.923879532511287,  0.38268343236509,   0.99144486137381,   0.130526192220052,  0.99144486137381,   0.130526192220051,  0.923879532511287,  0.38268343236509,
     0.793353340291235,  0.608761429008721,  0.608761429008721,  0.793353340291235,  0.38268343236509,   0.923879532511287,  0.130526192220052,  0.99144486137381,
     0.130526192220052,  0.99144486137381,   0.38268343236509,   0.923879532511287,  0.608761429008721,  0.793353340291235,  0.793353340291235,  0.608761429008721,
     0.923879532511287,  0.38268343236509,   0.99144486137381,   0.130526192220051,  0.99144486137381,   0.130526192220051,  0.923879532511287,  0.38268343236509,
     0.793353340291235,  0.60876142900872,   0.608761429008721,  0.793353340291235,  0.38268343236509,   0.923879532511287,  0.130526192220052,  0.99144486137381,
     0.130526192220052,  0.99144486137381,   0.38268343236509,   0.923879532511287,  0.608761429008721,  0.793353340291235,  0.793353340291235,  0.608761429008721,
     0.923879532511287,  0.38268343236509,   0.99144486137381,   0.130526192220052,  0.99144486137381,   0.130526192220051,  0.923879532511287,  0.38268343236509,
     0.793353340291235,  0.608761429008721,  0.608761429008721,  0.793353340291235,  0.38268343236509,   0.923879532511287,  0.130526192220052,  0.99144486137381,
     0.130526192220052,  0.99144486137381,   0.38268343236509,   0.923879532511287,  0.608761429008721,  0.793353340291235,  0.793353340291235,  0.608761429008721,
     0.923879532511287,  0.38268343236509,   0.99144486137381,   0.130526192220051,  0.99144486137381,   0.130526192220051,  0.923879532511287,  0.38268343236509,
     0.793353340291235,  0.60876142900872,   0.608761429008721,  0.793353340291235,  0.38268343236509,   0.923879532511287,  0.130526192220052,  0.99144486137381,
     0.130526192220052,  0.99144486137381,   0.38268343236509,   0.923879532511287,  0.608761429008721,  0.793353340291235,  0.793353340291235,  0.608761429008721,
     0.923879532511287,  0.38268343236509,   0.99144486137381,   0.130526192220052,  0.99144486137381,   0.130526192220051,  0.923879532511287,  0.38268343236509,
     0.793353340291235,  0.608761429008721,  0.608761429008721,  0.793353340291235,  0.38268343236509,   0.923879532511287,  0.130526192220052,  0.99144486137381,
     0.130526192220052,  0.99144486137381,   0.38268343236509,   0.923879532511287,  0.608761429008721,  0.793353340291235,  0.793353340291235,  0.608761429008721,
     0.923879532511287,  0.38268343236509,   0.99144486137381,   0.130526192220051,  0.99144486137381,   0.130526192220051,  0.923879532511287,  0.38268343236509,
     0.793353340291235,  0.60876142900872,   0.608761429008721,  0.793353340291235,  0.38268343236509,   0.923879532511287,  0.130526192220052,  0.99144486137381,
     0.130526192220052,  0.99144486137381,   0.38268343236509,   0.923879532511287,  0.608761429008721,  0.793353340291235,  0.793353340291235,  0.608761429008721,
     0.923879532511287,  0.38268343236509,   0.99144486137381,   0.130526192220052,  0.99144486137381,   0.130526192220051,  0.923879532511287,  0.38268343236509,
     0.793353340291235,  0.608761429008721,  0.608761429008721,  0.793353340291235,  0.38268343236509,   0.923879532511287,  0.130526192220052,  0.99144486137381,
     0.130526192220052,  0.99144486137381,   0.38268343236509,   0.923879532511287,  0.608761429008721,  0.793353340291235,  0.793353340291235,  0.608761429008721,
     0.923879532511287,  0.38268343236509,   0.99144486137381,   0.130526192220051,  0.99144486137381,   0.130526192220051,  0.923879532511287,  0.38268343236509,
     0.793353340291235,  0.60876142900872,   0.608761429008721,  0.793353340291235,  0.38268343236509,   0.923879532511287,  0.130526192220052,  0.99144486137381,
     0.130526192220052,  0.99144486137381,   0.38268343236509,   0.923879532511287,  0.608761429008721,  0.793353340291235,  0.793353340291235,  0.608761429008721,
     0.923879532511287,  0.38268343236509,   0.99144486137381,   0.130526192220052,  0.99144486137381,   0.130526192220051,  0.923879532511287,  0.38268343236509,
     0.793353340291235,  0.608761429008721,  0.608761429008721,  0.793353340291235,  0.38268343236509,   0.923879532511287,  0.130526192220052,  0.99144486137381,
     0.38268343236509,   0.923879532511287,  0.923879532511287,  0.38268343236509,   0.923879532511287,  0.38268343236509,   0.38268343236509,   0.923879532511287,
     0.38268343236509,   0.923879532511287,  0.923879532511287,  0.38268343236509,   0.923879532511287,  0.38268343236509,   0.38268343236509,   0.923879532511287,
];