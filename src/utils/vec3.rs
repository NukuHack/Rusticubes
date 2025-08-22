
use crate::utils::math::const_sqrt;
use glam::Vec3;

// Basic arithmetic operations
#[inline] pub const fn const_dot(a: Vec3, b: Vec3) -> f32 {
	a.x * b.x + a.y * b.y + a.z * b.z
}

#[inline] pub const fn const_cross(a: Vec3, b: Vec3) -> Vec3 {
	Vec3::new(
		a.y * b.z - a.z * b.y,
		a.z * b.x - a.x * b.z,
		a.x * b.y - a.y * b.x
	)
}

// Updated length function - now does actual length calculation
#[inline] pub const fn const_length(a: Vec3) -> f32 {
    const_sqrt(const_dot(a, a))
}

// Add distance function - now does actual distance calculation
#[inline] pub const fn const_distance(a: Vec3, b: Vec3) -> f32 {
    const_sqrt(const_distance_squared(a, b))
}

// Updated normalization function - now does actual normalization
#[inline] pub const fn const_normalize(a: Vec3) -> Vec3 {
    let len = const_length(a);
    if len == 0.0 {
        Vec3::ZERO
    } else {
        const_div_s(a, len)
    }
}

#[inline] pub const fn const_add(a: Vec3, b: Vec3) -> Vec3 {
	Vec3::new(a.x + b.x, a.y + b.y, a.z + b.z)
}

#[inline] pub const fn const_sub(a: Vec3, b: Vec3) -> Vec3 {
	Vec3::new(a.x - b.x, a.y - b.y, a.z - b.z)
}

#[inline] pub const fn const_mul(a: Vec3, b: Vec3) -> Vec3 {
	Vec3::new(a.x * b.x, a.y * b.y, a.z * b.z)
}

#[inline] pub const fn const_div(a: Vec3, b: Vec3) -> Vec3 {
	Vec3::new(a.x / b.x, a.y / b.y, a.z / b.z)
}

#[inline] pub const fn const_mul_s(a: Vec3, scalar: f32) -> Vec3 {
	Vec3::new(a.x * scalar, a.y * scalar, a.z * scalar)
}

#[inline] pub const fn const_div_s(a: Vec3, scalar: f32) -> Vec3 {
	Vec3::new(a.x / scalar, a.y / scalar, a.z / scalar)
}

#[inline] pub const fn const_neg(a: Vec3) -> Vec3 {
	Vec3::new(-a.x, -a.y, -a.z)
}

// Length and distance operations
#[inline] pub const fn const_length_squared(a: Vec3) -> f32 {
	const_dot(a, a)
}

#[inline] pub const fn const_distance_squared(a: Vec3, b: Vec3) -> f32 {
	const_length_squared(const_sub(a, b))
}

// Comparison operations
#[inline] pub const fn const_eq(a: Vec3, b: Vec3) -> bool {
	a.x == b.x && a.y == b.y && a.z == b.z
}

#[inline] pub const fn const_approx_eq(a: Vec3, b: Vec3, epsilon: f32) -> bool {
	const_max_element(const_abs(const_sub(a, b))) <= epsilon
}

#[inline] pub const fn const_max_element(a: Vec3) -> f32 {
	if a.x > a.y && a.x > a.z { return a.x }
	if a.z > a.y { a.z } else { a.y }
}

// Component-wise operations
#[inline] pub const fn const_abs(a: Vec3) -> Vec3 {
	Vec3::new(a.x.abs(), a.y.abs(), a.z.abs())
}

#[inline] pub const fn const_min(a: Vec3, b: Vec3) -> Vec3 {
	Vec3::new(
		if a.x < b.x { a.x } else { b.x },
		if a.y < b.y { a.y } else { b.y },
		if a.z < b.z { a.z } else { b.z }
	)
}

#[inline] pub const fn const_max(a: Vec3, b: Vec3) -> Vec3 {
	Vec3::new(
		if a.x > b.x { a.x } else { b.x },
		if a.y > b.y { a.y } else { b.y },
		if a.z > b.z { a.z } else { b.z }
	)
}

#[inline] pub const fn const_clamp(a: Vec3, min: Vec3, max: Vec3) -> Vec3 {
	const_max(const_min(a, max), min)
}

#[inline] pub const fn const_lerp(a: Vec3, b: Vec3, t: f32) -> Vec3 {
	const_add(a, const_mul_s(const_sub(b, a), t))
}

// Geometric operations
#[inline] pub const fn const_reflect(i: Vec3, n: Vec3) -> Vec3 {
	const_sub(i, const_mul_s(n, 2.0 * const_dot(i, n)))
}

#[inline] pub const fn const_project(a: Vec3, b: Vec3) -> Vec3 {
	let dot = const_dot(a, b);
	let len_sq = const_length_squared(b);
	if len_sq == 0.0 {
		Vec3::ZERO
	} else {
		const_mul_s(b, dot / len_sq)
	}
}

#[inline] pub const fn const_set_x(a: Vec3, x: f32) -> Vec3 {
	Vec3::new(x, a.y, a.z)
}

#[inline] pub const fn const_set_y(a: Vec3, y: f32) -> Vec3 {
	Vec3::new(a.x, y, a.z)
}

#[inline] pub const fn const_set_z(a: Vec3, z: f32) -> Vec3 {
	Vec3::new(a.x, a.y, z)
}
