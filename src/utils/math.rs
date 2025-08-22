

#[inline]
pub const fn const_sqrt(x: f32) -> f32 {
	// Handle special cases
	if x.is_nan() || x < 0.0 {
		return f32::NAN; // Or use f32::from_bits(0x7fc00000) for quiet NaN
	} else if x == 0.0 {
		return 0.0;
	} else if x.is_infinite() {
		return f32::INFINITY;
	}
	
	// Fast inverse square root approximation
	let i = x.to_bits();
	let i = 0x5f3759df - (i >> 1);
	let y = f32::from_bits(i);
	
	// Convert 1/sqrt(x) to sqrt(x)
	let result = x * y;
	
	// Newton-Raphson iterations
	let improved = 0.5 * (result + x / result);
	let better = 0.5 * (improved + x / improved);
	let final_result = 0.5 * (better + x / better);
	0.5 * (final_result + x / final_result)
}



/// Helper function to interpolate between two angles, handling wraparound
#[inline] 
pub const fn lerp_angle(from: f32, to: f32, t: f32) -> f32 {
	let diff = ((to - from + std::f32::consts::PI) % (2.0 * std::f32::consts::PI)) - std::f32::consts::PI;
	from + diff * t
}

/// Helper function to linearly interpolate between two f32 values
#[inline] 
pub const fn lerp_f32(from: f32, to: f32, t: f32) -> f32 {
	from + (to - from) * t
}
