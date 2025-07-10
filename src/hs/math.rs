// might need more of these small files for math and other extra shit 
use std::time::{SystemTime, UNIX_EPOCH};

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
    // Fast random number generator (xorshift)
    pub fn next_u32(&mut self) -> u32 {
        self.state ^= self.state << 13;
        self.state ^= self.state >> 17;
        self.state ^= self.state << 5;
        self.state
    }
    #[inline]
    // Hashing number generator (pcg_hash)
    fn next_u32__(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(747796405).wrapping_add(2891336453);
        let state = self.state;
        let word = ((state >> ((state >> 28) + 4)) ^ state).wrapping_mul(277803737);
        (word >> 22) ^ word
    }

    #[inline]
    fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    fn range_f32(&mut self, min: f32, max: f32) -> f32 {
        self.next_f32() * (max-min) + min
    }
}

// Convenience functions
#[inline]
pub fn random_float(min: f32, max: f32) -> f32 {
    Rand::from_time().range_f32(min, max)
}
#[inline]
pub fn next_float() -> f32 {
    Rand::from_time().next_f32()
}
#[inline]
pub fn next_int() -> u32 {
    Rand::from_time().next_u32()
}
#[inline]
pub fn random_float_seeded(seed: u32, min: f32, max: f32) -> f32 {
    Rand::new(seed).range_f32(min, max)
}

#[test]
pub fn test_gen() {
    // Image dimensions
    let width = 800;
    let height = 600;
    
    // Create a buffer to store the image
    let mut img = vec![0u8; width * height * 3];
    let perl = Noise::new(next_int());
    
    for y in 0..height {
        for x in 0..width {
            // Seed the RNG with pixel coordinates
            
            // Generate RGB values
            //let color = perl.noise_2d(y as i32, x as i32);
            let color = perl.noise_2d(y as i32, x as i32) * 255.0;
            
            // Convert to 8-bit values and store in buffer
            let idx = (y * width + x) as usize * 3;
            img[idx] = color as u8;
            img[idx + 1] = color as u8;
            img[idx + 2] = color as u8;
        }
    }
    
    // In a real application, you would save the image to a file here
    // For example, using the image crate: 
    match image::save_buffer("output.png", &img, width as u32, height as u32, image::ColorType::Rgb8) {
        Ok(_) => {},
        Err(e) => { println!("image save error: {:?}", e); },
    }
    println!("Image generated ({}x{})", width, height);
}



// Inspiration from https://github.com/Auburn/FastNoiseLite/blob/master/Rust/src/lib.rs

pub struct Noise {
    seed: u32,
}

impl Noise {
    pub fn new(seed: u32) -> Self {
        Self { seed }
    }
    
    pub fn from_time() -> Self {
        Self { seed: next_int() }
    }

    #[inline(always)]
    fn grad(seed: i32, x_primed: i32, y_primed: i32, xd: f32, yd: f32) -> f32 {
        let hash = Self::hash(seed, x_primed, y_primed);
        // Improved hash distribution with better mixing
        let hash = hash ^ (hash >> 16);
        let hash = hash.wrapping_mul(0x85ebca6bu32 as i32);
        let hash = hash ^ (hash >> 13);
        let hash = hash.wrapping_mul(0xc2b2ae35u32 as i32);
        let hash = hash ^ (hash >> 16);
        
        let hash = ((hash & 0x7FFFFFFF) >> 24) & 127; // Use more bits, ensure positive
        
        unsafe {
            // SAFETY: hash is always within bounds due to masking
            let xg = *GRADIENTS_2D.get_unchecked((hash & 0xFE) as usize);
            let yg = *GRADIENTS_2D.get_unchecked((hash | 1) as usize);
            xd * xg + yd * yg
        }
    }

    #[inline(always)]
    fn hash(seed: i32, x_primed: i32, y_primed: i32) -> i32 {
        let mut hash = seed ^ x_primed ^ y_primed;
        // Better hash mixing to avoid clustering
        hash = hash.wrapping_mul(0x27d4eb2d);
        hash = hash ^ (hash >> 15);
        hash = hash.wrapping_mul(0x2ba8b153);
        hash
    }

    #[inline(always)]
    fn floor(f: f32) -> i32 {
        f as i32 - (f < 0.0) as i32
    }

    pub fn noise_2d(&self, x: i32, y: i32) -> f32 {
        // Precompute consts
        const F2: f32 = 0.5 * (1.7320508075688772 - 1.);
        const G2: f32 = (3. - 1.7320508075688772) / 6.;
        const G2_2: f32 = 2. * G2;
        const G2_2_MINUS_1: f32 = G2_2 - 1.;
        const SCALE: f32 = 17.;
        const FREQE: f32 = 1. / 137.;
        const ONE_MINUS_2G2: f32 = 1. - 2. * G2;
        const INVERSE_G2_MINUS_2: f32 = (1. / G2) - 2.;
        // Hashing consts
        const PRIME_X: i32 = 0x5205402B;
        const PRIME_Y: i32 = 0x5AC0E4F1;
        // Convert input coordinates with better frequency
        let x = x as f32 * FREQE;
        let y = y as f32 * FREQE;
        // Skew the input space
        let t = (x + y) * F2;
        let x = x + t;
        let y = y + t;
        // Get base coordinates
        let i = Self::floor(x);
        let j = Self::floor(y);
        let xi = x - i as f32;
        let yi = y - j as f32;
        // Get relative coordinates
        let t = (xi + yi) * G2;
        let x0 = xi - t;
        let y0 = yi - t;

        // Prime the coordinates
        let i = i.wrapping_mul(PRIME_X);
        let j = j.wrapping_mul(PRIME_Y);

        // Calculate n0 contribution with adjusted falloff
        let a = 0.5 - x0 * x0 - y0 * y0;
        let n0 = if a > 0.0 {
            a*a*a* Self::grad(self.seed as i32, i, j, x0, y0)
        } else {
            0.0
        };

        // Calculate n2 contribution with adjusted falloff
        let c = (2.0 * ONE_MINUS_2G2 * INVERSE_G2_MINUS_2) * t 
            + ((-2.0 * ONE_MINUS_2G2 * ONE_MINUS_2G2) + a);
        let n2 = if c > 0.0 {
            c*c*c * Self::grad(
                self.seed as i32,
                i.wrapping_add(PRIME_X),
                j.wrapping_add(PRIME_Y),
                x0 + G2_2_MINUS_1,
                y0 + G2_2_MINUS_1,
            )
        } else {
            0.0
        };

        // Calculate n1 contribution with adjusted falloff
        let n1 = if y0 > x0 {
            let x1 = x0 + G2;
            let y1 = y0 + (G2 - 1.0);
            let b = 0.5 - x1 * x1 - y1 * y1;
        
            if b > 0.0 {
                b*b*b * Self::grad(self.seed as i32, i, j.wrapping_add(PRIME_Y), x1, y1)
            } else {
                0.0
            }
        } else {
            let x1 = x0 + (G2 - 1.0);
            let y1 = y0 + G2;
            let b = 0.5 - x1 * x1 - y1 * y1;
        
            if b > 0.0 {
                b*b*b * Self::grad(self.seed as i32, i.wrapping_add(PRIME_X), j, x1, y1)
            } else {
                0.0
            }
        };

        let result = (n0 + n1 + n2) * SCALE;
                
        // Clamp to reasonable range
        (result + 0.53).clamp(0., 1.)
    }
}



#[allow(dead_code)]
const GRADIENTS_2D: [f32; 256] = [
    0.130526192220052, 0.99144486137381, 0.38268343236509, 0.923879532511287,
    0.608761429008721, 0.793353340291235, 0.793353340291235, 0.608761429008721,
    0.923879532511287, 0.38268343236509, 0.99144486137381, 0.130526192220051,
    0.99144486137381, -0.130526192220051, 0.923879532511287, -0.38268343236509,
    0.793353340291235, -0.608761429008721, 0.608761429008721, -0.793353340291235,
    0.38268343236509, -0.923879532511287, 0.130526192220052, -0.99144486137381,
    -0.130526192220052, -0.99144486137381, -0.38268343236509, -0.923879532511287,
    -0.608761429008721, -0.793353340291235, -0.793353340291235, -0.608761429008721,
    -0.923879532511287, -0.38268343236509, -0.99144486137381, -0.130526192220052,
    -0.99144486137381, 0.130526192220051, -0.923879532511287, 0.38268343236509,
    -0.793353340291235, 0.608761429008721, -0.608761429008721, 0.793353340291235,
    -0.38268343236509, 0.923879532511287, -0.130526192220052, 0.99144486137381,
    0.130526192220052, 0.99144486137381, 0.38268343236509, 0.923879532511287,
    0.608761429008721, 0.793353340291235, 0.793353340291235, 0.608761429008721,
    0.923879532511287, 0.38268343236509, 0.99144486137381, 0.130526192220051,
    0.99144486137381, -0.130526192220051, 0.923879532511287, -0.38268343236509,
    0.793353340291235, -0.608761429008721, 0.608761429008721, -0.793353340291235,
    0.38268343236509, -0.923879532511287, 0.130526192220052, -0.99144486137381,
    -0.130526192220052, -0.99144486137381, -0.38268343236509, -0.923879532511287,
    -0.608761429008721, -0.793353340291235, -0.793353340291235, -0.608761429008721,
    -0.923879532511287, -0.38268343236509, -0.99144486137381, -0.130526192220052,
    -0.99144486137381, 0.130526192220051, -0.923879532511287, 0.38268343236509,
    -0.793353340291235, 0.608761429008721, -0.608761429008721, 0.793353340291235,
    -0.38268343236509, 0.923879532511287, -0.130526192220052, 0.99144486137381,
    0.130526192220052, 0.99144486137381, 0.38268343236509, 0.923879532511287,
    0.608761429008721, 0.793353340291235, 0.793353340291235, 0.608761429008721,
    0.923879532511287, 0.38268343236509, 0.99144486137381, 0.130526192220051,
    0.99144486137381, -0.130526192220051, 0.923879532511287, -0.38268343236509,
    0.793353340291235, -0.608761429008721, 0.608761429008721, -0.793353340291235,
    0.38268343236509, -0.923879532511287, 0.130526192220052, -0.99144486137381,
    -0.130526192220052, -0.99144486137381, -0.38268343236509, -0.923879532511287,
    -0.608761429008721, -0.793353340291235, -0.793353340291235, -0.608761429008721,
    0.130526192220052, 0.99144486137381, 0.38268343236509, 0.923879532511287,
    0.608761429008721, 0.793353340291235, 0.793353340291235, 0.608761429008721,
    0.923879532511287, 0.38268343236509, 0.99144486137381, 0.130526192220051,
    0.99144486137381, -0.130526192220051, 0.923879532511287, -0.38268343236509,
    0.793353340291235, -0.608761429008721, 0.608761429008721, -0.793353340291235,
    0.38268343236509, -0.923879532511287, 0.130526192220052, -0.99144486137381,
    -0.130526192220052, -0.99144486137381, -0.38268343236509, -0.923879532511287,
    -0.608761429008721, -0.793353340291235, -0.793353340291235, -0.608761429008721,
    -0.923879532511287, -0.38268343236509, -0.99144486137381, -0.130526192220052,
    -0.99144486137381, 0.130526192220051, -0.923879532511287, 0.38268343236509,
    -0.793353340291235, 0.608761429008721, -0.608761429008721, 0.793353340291235,
    -0.38268343236509, 0.923879532511287, -0.130526192220052, 0.99144486137381,
    0.130526192220052, 0.99144486137381, 0.38268343236509, 0.923879532511287,
    0.608761429008721, 0.793353340291235, 0.793353340291235, 0.608761429008721,
    0.923879532511287, 0.38268343236509, 0.99144486137381, 0.130526192220051,
    0.99144486137381, -0.130526192220051, 0.923879532511287, -0.38268343236509,
    0.793353340291235, -0.608761429008721, 0.608761429008721, -0.793353340291235,
    0.38268343236509, -0.923879532511287, 0.130526192220052, -0.99144486137381,
    -0.130526192220052, -0.99144486137381, -0.38268343236509, -0.923879532511287,
    -0.608761429008721, -0.793353340291235, -0.793353340291235, -0.608761429008721,
    -0.923879532511287, -0.38268343236509, -0.99144486137381, -0.130526192220052,
    -0.99144486137381, 0.130526192220051, -0.923879532511287, 0.38268343236509,
    -0.793353340291235, 0.608761429008721, -0.608761429008721, 0.793353340291235,
    -0.38268343236509, 0.923879532511287, -0.130526192220052, 0.99144486137381,
    0.130526192220052, 0.99144486137381, 0.38268343236509, 0.923879532511287,
    0.608761429008721, 0.793353340291235, 0.793353340291235, 0.608761429008721,
    0.923879532511287, 0.38268343236509, 0.99144486137381, 0.130526192220051,
    0.99144486137381, -0.130526192220051, 0.923879532511287, -0.38268343236509,
    0.793353340291235, -0.608761429008721, 0.608761429008721, -0.793353340291235,
    0.38268343236509, -0.923879532511287, 0.130526192220052, -0.99144486137381,
    -0.130526192220052, -0.99144486137381, -0.38268343236509, -0.923879532511287,
    -0.608761429008721, -0.793353340291235, -0.793353340291235, -0.608761429008721,
];