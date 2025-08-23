
use crate::utils::math::const_sqrt;
use glam::Vec2;

// Basic arithmetic operations
#[inline] pub const fn const_dot(a: Vec2, b: Vec2) -> f32 {
	a.x * b.x + a.y * b.y
}

#[inline] pub const fn const_cross(a: Vec2, b: Vec2) -> f32 {
	a.x * b.y - a.y * b.x
}

#[inline] pub const fn const_length(a: Vec2) -> f32 {
	const_sqrt(const_dot(a, a))
}

#[inline] pub const fn const_distance(a: Vec2, b: Vec2) -> f32 {
	const_sqrt(const_distance_squared(a, b))
}

#[inline] pub const fn const_normalize(a: Vec2) -> Vec2 {
	let len = const_length(a);
	if len == 0.0 {
		Vec2::ZERO
	} else {
		const_div_s(a, len)
	}
}

#[inline] pub const fn const_add(a: Vec2, b: Vec2) -> Vec2 {
	Vec2::new(a.x + b.x, a.y + b.y)
}

#[inline] pub const fn const_sub(a: Vec2, b: Vec2) -> Vec2 {
	Vec2::new(a.x - b.x, a.y - b.y)
}

#[inline] pub const fn const_mul(a: Vec2, b: Vec2) -> Vec2 {
	Vec2::new(a.x * b.x, a.y * b.y)
}

#[inline] pub const fn const_div(a: Vec2, b: Vec2) -> Vec2 {
	Vec2::new(a.x / b.x, a.y / b.y)
}

#[inline] pub const fn const_mul_s(a: Vec2, scalar: f32) -> Vec2 {
	Vec2::new(a.x * scalar, a.y * scalar)
}

#[inline] pub const fn const_div_s(a: Vec2, scalar: f32) -> Vec2 {
	Vec2::new(a.x / scalar, a.y / scalar)
}

#[inline] pub const fn const_neg(a: Vec2) -> Vec2 {
	Vec2::new(-a.x, -a.y)
}

// Length and distance operations
#[inline] pub const fn const_length_squared(a: Vec2) -> f32 {
	const_dot(a, a)
}

#[inline] pub const fn const_distance_squared(a: Vec2, b: Vec2) -> f32 {
	const_length_squared(const_sub(a, b))
}

// Comparison operations
#[inline] pub const fn const_eq(a: Vec2, b: Vec2) -> bool {
	a.x == b.x && a.y == b.y
}

#[inline] pub const fn const_approx_eq(a: Vec2, b: Vec2, epsilon: f32) -> bool {
	const_max_element(const_abs(const_sub(a, b))) <= epsilon
}

#[inline] pub const fn const_max_element(a: Vec2) -> f32 {
	if a.x > a.y { a.x } else { a.y }
}

// Component-wise operations
#[inline] pub const fn const_abs(a: Vec2) -> Vec2 {
	Vec2::new(a.x.abs(), a.y.abs())
}

#[inline] pub const fn const_min(a: Vec2, b: Vec2) -> Vec2 {
	Vec2::new(
		if a.x < b.x { a.x } else { b.x },
		if a.y < b.y { a.y } else { b.y }
	)
}

#[inline] pub const fn const_max(a: Vec2, b: Vec2) -> Vec2 {
	Vec2::new(
		if a.x > b.x { a.x } else { b.x },
		if a.y > b.y { a.y } else { b.y }
	)
}

#[inline] pub const fn const_clamp(a: Vec2, min: Vec2, max: Vec2) -> Vec2 {
	const_max(const_min(a, max), min)
}

#[inline] pub const fn const_lerp(a: Vec2, b: Vec2, t: f32) -> Vec2 {
	const_add(a, const_mul_s(const_sub(b, a), t))
}

// Geometric operations
#[inline] pub const fn const_reflect(i: Vec2, n: Vec2) -> Vec2 {
	const_sub(i, const_mul_s(n, 2.0 * const_dot(i, n)))
}

#[inline] pub const fn const_project(a: Vec2, b: Vec2) -> Vec2 {
	let dot = const_dot(a, b);
	let len_sq = const_length_squared(b);
	if len_sq == 0.0 {
		Vec2::ZERO
	} else {
		const_mul_s(b, dot / len_sq)
	}
}

#[inline] pub const fn const_set_x(a: Vec2, x: f32) -> Vec2 {
	Vec2::new(x, a.y)
}

#[inline] pub const fn const_set_y(a: Vec2, y: f32) -> Vec2 {
	Vec2::new(a.x, y)
}

