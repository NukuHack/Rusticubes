use std::time::{SystemTime, UNIX_EPOCH};

/// A simple random number generator with multiple algorithms
pub struct Rand {
	seed: u32,
}

impl Rand {
	/// Creates a new RNG with the given seed
	#[inline(always)]
	pub fn new(seed: u32) -> Self {
		Self { seed }
	}
	
	/// Creates a new RNG seeded with the current time
	#[inline(always)]
	pub fn from_time() -> Self {
		let seed = SystemTime::now()
			.duration_since(UNIX_EPOCH)
			.unwrap()
			.subsec_nanos();
		Self::new(seed)
	}

	// ========== Basic RNG Methods ==========
	
	/// Returns a random f32 in [0, 1)
	#[inline(always)]
	pub fn next_f32(&mut self) -> f32 {
		self.next_u32() as f32 / u32::MAX as f32
	}
	
	/// Returns a random u32
	#[inline(always)]
	pub fn next_u32(&mut self) -> u32 {
		let rand = Self::xorshift(self.seed);
		self.seed = rand;
		rand
	}
	
	/// Returns a random f32 in [min, max)
	#[inline(always)]
	pub fn range_f32(&mut self, min: f32, max: f32) -> f32 {
		self.next_f32() * (max - min) + min
	}
	
	// ========== RNG Algorithms ==========
	
	/// PCG hash function variant
	#[inline(always)]
	pub fn pcg_hash(input: u32) -> u32 {
		let mut seed = input.wrapping_mul(747796405).wrapping_add(2891336453);
		seed = ((seed >> ((seed >> 28) + 4)) ^ seed).wrapping_mul(277803737);
		(seed >> 22) ^ seed
	}
	
	/// Xorshift algorithm (fast)
	#[inline(always)]
	pub fn xorshift(mut seed: u32) -> u32 {
		seed ^= seed << 13;
		seed ^= seed >> 17;
		seed ^= seed << 5;
		seed
	}
	
	/// PCG32 variant (fast)
	#[inline(always)]
	pub fn pcg32_fast(seed: u32) -> u32 {
		let x = 0x15ea5e5;
		let count = x >> 29;
		let y = seed >> 15;
		let x = y ^ (x >> 22);
		x >> (22 + count)
	}
	
	/// PCG with better mixing
	#[inline(always)]
	pub fn pcg_improved(input: u32) -> u32 {
		let state = input.wrapping_add(0x9e3779b9);
		state
			.wrapping_mul(0x85ebca6b)
			.rotate_left(13)
			.wrapping_mul(0xc2b2ae35)
	}
}

// ========== Convenience Functions ==========

/// Returns a random f32 in [min, max) using a time-based seed
	#[inline(always)]
pub fn random_float(min: f32, max: f32) -> f32 {
	Rand::from_time().range_f32(min, max)
}

/// Returns a random f32 in [0, 1) using a time-based seed
	#[inline(always)]
pub fn next_float() -> f32 {
	Rand::from_time().next_f32()
}

/// Returns a random u32 in [min, max) using a time-based seed
	#[inline(always)]
pub fn random_int(min: u32, max: u32) -> u32 {
	min + (Rand::from_time().next_u32() % (max - min))
}

/// Returns a random u32 using a time-based seed
	#[inline(always)]
pub fn next_int() -> u32 {
	Rand::from_time().next_u32()
}

/// Returns a random bool using a time-based seed
	#[inline(always)]
pub fn random_bool() -> bool {
	Rand::from_time().next_u32() % 2 == 0
}

// Sigmoid-like interpolation (recommended)
pub fn smooth_interpolate(noise: f32) -> f32 {
	// Uses a sigmoid-like curve that doesn't flatten at 0
	let scaled = noise * 2.0; // increase sensitivity
	let sigmoid = scaled / (1.0 + scaled.abs()); // smooth sigmoid
	if sigmoid > 0.0 {
		sigmoid * 0.85
	} else {
		sigmoid * 0.25
	}
}

// Inspiration from https://github.com/Auburn/FastNoiseLite/blob/master/Rust/src/lib.rs

pub struct Noise {
	seed: u32,
}

impl Noise {
	#[inline(always)]
	pub fn new(seed: u32) -> Self {
		Self { seed }
	}
	
	#[inline(always)]
	pub fn from_time() -> Self {
		Self { seed: next_int() }
	}

	#[inline]
	fn grad(seed: i32, x_primed: i32, y_primed: i32, xd: f32, yd: f32) -> f32 {
		let hash = Self::hash(seed, x_primed, y_primed);
		
		// Improved hash mixing
		let hash = hash.wrapping_mul(0x27d4eb2d);
		let hash = hash ^ (hash >> 15);
		let hash = hash.wrapping_mul(0x2ba8b153);
		let hash = hash ^ (hash >> 16);
		
		let hash = (hash & 0x7FFFFFFF) >> 24;
		
		unsafe {
			let idx = (hash & 0x7E) as usize; // 126 elements (0-125)
			let xg = *GRADIENTS_2D.get_unchecked(idx);
			let yg = *GRADIENTS_2D.get_unchecked(idx + 1);
			xd * xg + yd * yg
		}
	}

	#[inline(always)]
	fn hash(seed: i32, x_primed: i32, y_primed: i32) -> i32 {
		let mut hash = seed ^ x_primed ^ y_primed;
		hash = hash.wrapping_mul(0x27d4eb2d);
		hash = hash ^ (hash >> 15);
		hash = hash.wrapping_mul(0x2ba8b153);
		hash
	}

	#[inline(always)]
	fn floor(f: f32) -> i32 {
		f as i32 - (f < 0.0) as i32
	}

	pub fn noise_2d(&self, x: f32, y: f32) -> f32 {
		const F2: f32 = 0.5 * (1.7320508075688772 - 1.0);
		const G2: f32 = (3.0 - 1.7320508075688772) / 6.0;
		const G2_2: f32 = 2.0 * G2;
		const ONE_MINUS_2G2: f32 = 1.0 - 2.0 * G2;
		
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
		const PRIME_X: i32 = 0x5205402B;
		const PRIME_Y: i32 = 0x5AC0E4F1;
		let i = i.wrapping_mul(PRIME_X);
		let j = j.wrapping_mul(PRIME_Y);

		// Calculate n0 contribution with improved falloff
		let a = 0.75 - x0 * x0 - y0 * y0; // Increased radius for better coverage
		let n0 = if a > 0.0 {
			let a2 = a * a;
			a2 * a2 * Self::grad(self.seed as i32, i, j, x0, y0) // Fourth power for smoother falloff
		} else {
			0.0
		};

		// Calculate n2 contribution
		let c = (2.0 * ONE_MINUS_2G2 * ((1.0 / G2) - 2.0)) * t 
			+ ((-2.0 * ONE_MINUS_2G2 * ONE_MINUS_2G2) + a);
		let n2 = if c > 0.0 {
			let c2 = c * c;
			c2 * c2 * Self::grad(
				self.seed as i32,
				i.wrapping_add(PRIME_X),
				j.wrapping_add(PRIME_Y),
				x0 + G2_2 - 1.0,
				y0 + G2_2 - 1.0,
			)
		} else {
			0.0
		};

		// Calculate n1 contribution
		let n1 = if y0 > x0 {
			let x1 = x0 + G2;
			let y1 = y0 + (G2 - 1.0);
			let b = 0.75 - x1 * x1 - y1 * y1;
		
			if b > 0.0 {
				let b2 = b * b;
				b2 * b2 * Self::grad(self.seed as i32, i, j.wrapping_add(PRIME_Y), x1, y1)
			} else {
				0.0
			}
		} else {
			let x1 = x0 + (G2 - 1.0);
			let y1 = y0 + G2;
			let b = 0.75 - x1 * x1 - y1 * y1;
		
			if b > 0.0 {
				let b2 = b * b;
				b2 * b2 * Self::grad(self.seed as i32, i.wrapping_add(PRIME_X), j, x1, y1)
			} else {
				0.0
			}
		};

		// Scale the result to better range
		(n0 + n1 + n2) * 35.0
	}

	// Significantly improved fractal noise with proper octave handling
	pub fn fractal_noise_2d(&self, x: i32, y: i32) -> f32 {
		const BASE_FREQ: f32 = 0.004;       // Better base frequency
		const LACUNARITY: f32 = 2.;         // Frequency multiplier per octave
		const PERSISTENCE: f32 = 0.5;       // Amplitude multiplier per octave
		const OCTAVES: u32 = 6;             // More octaves for detail
		
		let x = x as f32 * BASE_FREQ;
		let y = y as f32 * BASE_FREQ;

		let mut noise_sum = 0.;
		let mut amplitude = 1.;
		let mut frequency = 1.;
		let mut max_value = 1.; // For normalization
		
		for _ in 0..OCTAVES {
			noise_sum += self.noise_2d(x * frequency, y * frequency) * amplitude;
			max_value += amplitude;
			
			amplitude *= PERSISTENCE;
			frequency *= LACUNARITY;
		}

		// Normalize to prevent values from growing too large
		noise_sum / max_value
	}

	// Additional terrain-specific noise functions
	#[inline(always)]
	pub fn ridged_noise_2d(&self, x: i32, y: i32) -> f32 {
		let noise = self.fractal_noise_2d(x, y);
		1.0 - (noise * 2.0).abs() // Create ridges
	}

	#[inline(always)]
	pub fn turbulence_2d(&self, x: i32, y: i32) -> f32 {
		const BASE_FREQ: f32 = 0.004;       // Better base frequency
		const LACUNARITY: f32 = 2.;         // Frequency multiplier per octave
		const PERSISTENCE: f32 = 0.5;       // Amplitude multiplier per octave
		const OCTAVES: u32 = 4;             // Less octaves for smoother terrain
		
		let x = x as f32 * BASE_FREQ;
		let y = y as f32 * BASE_FREQ;

		let mut noise_sum = 0.;
		let mut amplitude = 1.;
		let mut frequency = 1.;
		
		for _ in 0..OCTAVES {
			noise_sum += self.noise_2d(x * frequency, y * frequency).abs() * amplitude;
			amplitude *= PERSISTENCE;
			frequency *= LACUNARITY;
		}

		noise_sum
	}

	// Warped domain noise for more organic terrain
	#[inline(always)]
	pub fn warped_noise_2d(&self, x: i32, y: i32) -> f32 {
		const WARP_STRENGTH: f32 = 0.1;
		
		let x_f = x as f32;
		let y_f = y as f32;
		
		// Generate domain warp offsets
		let warp_x = self.fractal_noise_2d((x as f32 * 1.3) as i32, (y as f32 * 1.7) as i32) * WARP_STRENGTH;
		let warp_y = self.fractal_noise_2d((x as f32 * 1.7) as i32, (y as f32 * 1.3) as i32) * WARP_STRENGTH;
		
		// Sample noise at warped coordinates
		let warped_x = (x_f + warp_x * 50.0) as i32;
		let warped_y = (y_f + warp_y * 50.0) as i32;
		
		self.fractal_noise_2d(warped_x, warped_y)
	}

	// Combine multiple noise types for complex terrain
	#[inline(always)]
	pub fn terrain_noise_2d(&self, x: i32, y: i32) -> f32 {
		let base = self.fractal_noise_2d(x, y);
		let ridged = self.ridged_noise_2d(x, y) * 0.3;
		let turbulence = self.turbulence_2d(x, y) * 0.2;
		let warped = self.warped_noise_2d(x, y) * 0.4;
		
		// Combine with different weights
		(base + ridged + turbulence + warped) * 0.5
	}
}


#[allow(dead_code)]
// Improved gradient table with 64 normalized 2D gradients
const GRADIENTS_2D: [f32; 128] = [
	// First quadrant (0-π/2)
	0.38268343236509, 0.923879532511287,  // 22.5°
	0.195090322016128, 0.98078528040323,  // 11.25°
	0.555570233019602, 0.831469612302545, // 33.75°
	0.0980171403295606, 0.995184726672197, // 5.625°
	// Second quadrant (π/2-π)
	-0.38268343236509, 0.923879532511287,
	-0.195090322016128, 0.98078528040323,
	-0.555570233019602, 0.831469612302545,
	-0.0980171403295606, 0.995184726672197,
	// Third quadrant (π-3π/2)
	-0.38268343236509, -0.923879532511287,
	-0.195090322016128, -0.98078528040323,
	-0.555570233019602, -0.831469612302545,
	-0.0980171403295606, -0.995184726672197,
	// Fourth quadrant (3π/2-2π)
	0.38268343236509, -0.923879532511287,
	0.195090322016128, -0.98078528040323,
	0.555570233019602, -0.831469612302545,
	0.0980171403295606, -0.995184726672197,
	// Additional gradients for better coverage
	0.707106781186548, 0.707106781186547,  // 45°
	0.831469612302545, 0.555570233019602,  // 56.25°
	0.923879532511287, 0.38268343236509,   // 67.5°
	0.98078528040323, 0.195090322016128,   // 78.75°
	-0.707106781186548, 0.707106781186547,
	-0.831469612302545, 0.555570233019602,
	-0.923879532511287, 0.38268343236509,
	-0.98078528040323, 0.195090322016128,
	-0.707106781186548, -0.707106781186547,
	-0.831469612302545, -0.555570233019602,
	-0.923879532511287, -0.38268343236509,
	-0.98078528040323, -0.195090322016128,
	0.707106781186548, -0.707106781186547,
	0.831469612302545, -0.555570233019602,
	0.923879532511287, -0.38268343236509,
	0.98078528040323, -0.195090322016128,
	// More detailed angles
	0.290284677254462, 0.956940335732209,
	0.471396736825998, 0.881921264348355,
	0.634393284163645, 0.773010453362737,
	0.773010453362737, 0.634393284163645,
	0.881921264348355, 0.471396736825998,
	0.956940335732209, 0.290284677254462,
	-0.290284677254462, 0.956940335732209,
	-0.471396736825998, 0.881921264348355,
	-0.634393284163645, 0.773010453362737,
	-0.773010453362737, 0.634393284163645,
	-0.881921264348355, 0.471396736825998,
	-0.956940335732209, 0.290284677254462,
	-0.290284677254462, -0.956940335732209,
	-0.471396736825998, -0.881921264348355,
	-0.634393284163645, -0.773010453362737,
	-0.773010453362737, -0.634393284163645,
	-0.881921264348355, -0.471396736825998,
	-0.956940335732209, -0.290284677254462,
	0.290284677254462, -0.956940335732209,
	0.471396736825998, -0.881921264348355,
	0.634393284163645, -0.773010453362737,
	0.773010453362737, -0.634393284163645,
	0.881921264348355, -0.471396736825998,
	0.956940335732209, -0.290284677254462,
	// First quadrant (0-π/2)
	0.38268343236509, 0.923879532511287,  // 22.5°
	0.195090322016128, 0.98078528040323,  // 11.25°
	0.555570233019602, 0.831469612302545, // 33.75°
	0.0980171403295606, 0.995184726672197, // 5.625°
	// Second quadrant (π/2-π)
	-0.38268343236509, 0.923879532511287,
	-0.195090322016128, 0.98078528040323,
	-0.555570233019602, 0.831469612302545,
	-0.0980171403295606, 0.995184726672197,
];