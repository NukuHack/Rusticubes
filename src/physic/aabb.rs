use glam::{Vec3, IVec3};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AABB {
	pub min: Vec3,
	pub max: Vec3,
}

impl AABB {
	/// Creates a new AABB from min and max coordinates
	#[inline]
	pub fn new(min: Vec3, max: Vec3) -> Self {
		Self { min, max }
	}
	
	/// Creates an AABB from center position and half-extents
	#[inline]
	pub fn from_center(center: Vec3, half_extents: Vec3) -> Self {
		Self {
			min: center - half_extents,
			max: center + half_extents,
		}
	}
	
	/// Creates an AABB from integer coordinates (useful for voxel systems)
	#[inline]
	pub fn from_ivec(min: IVec3, max: IVec3) -> Self {
		Self {
			min: min.as_vec3(),
			max: max.as_vec3(),
		}
	}
	
	/// Returns the dimensions (width, height, depth) of the AABB
	#[inline]
	pub fn dimensions(&self) -> Vec3 {
		self.max - self.min
	}
	
	/// Returns the center position of the AABB
	#[inline]
	pub fn center(&self) -> Vec3 {
		(self.min + self.max) * 0.5
	}
	
	/// Returns the half-extents of the AABB
	#[inline]
	pub fn half_extents(&self) -> Vec3 {
		self.dimensions() * 0.5
	}
	
	/// Returns the volume of the AABB
	#[inline]
	pub fn volume(&self) -> f32 {
		let dims = self.dimensions();
		dims.x * dims.y * dims.z
	}
	
	/// Checks if this AABB overlaps with another AABB
	#[inline]
	pub fn intersects(&self, other: &AABB) -> bool {
		self.min.x <= other.max.x &&
		self.max.x >= other.min.x &&
		self.min.y <= other.max.y &&
		self.max.y >= other.min.y &&
		self.min.z <= other.max.z &&
		self.max.z >= other.min.z
	}
	
	/// Checks if a point is inside the AABB
	#[inline]
	pub fn contains_point(&self, point: Vec3) -> bool {
		point.x >= self.min.x &&
		point.x <= self.max.x &&
		point.y >= self.min.y &&
		point.y <= self.max.y &&
		point.z >= self.min.z &&
		point.z <= self.max.z
	}
	
	/// Returns a new AABB that is this AABB moved by the given vector
	#[inline]
	pub fn translate(&self, translation: Vec3) -> Self {
		Self {
			min: self.min + translation,
			max: self.max + translation,
		}
	}
	
	/// Returns the smallest AABB that contains both this and another AABB
	#[inline]
	pub fn union(&self, other: &AABB) -> Self {
		Self {
			min: self.min.min(other.min),
			max: self.max.max(other.max),
		}
	}
	
	/// Calculates the penetration vector when this AABB is colliding with another
	/// Returns None if there's no collision, otherwise returns the minimum translation vector
	pub fn penetration_vector(&self, other: &AABB) -> Option<Vec3> {
		if !self.intersects(other) {
			return None;
		}
		
		let center_diff = other.center() - self.center();
		let overlap = Vec3::new(
			self.max.x.min(other.max.x) - self.min.x.max(other.min.x),
			self.max.y.min(other.max.y) - self.min.y.max(other.min.y),
			self.max.z.min(other.max.z) - self.min.z.max(other.min.z),
		);
		
		// Find the axis with minimum penetration
		let mut min_axis = 0;
		let mut min_overlap = overlap.x;
		
		if overlap.y < min_overlap {
			min_axis = 1;
			min_overlap = overlap.y;
		}
		
		if overlap.z < min_overlap {
			min_axis = 2;
			min_overlap = overlap.z;
		}
		
		let mut normal = Vec3::ZERO;
		match min_axis {
			0 => normal.x = if center_diff.x > 0.0 { -1.0 } else { 1.0 },
			1 => normal.y = if center_diff.y > 0.0 { -1.0 } else { 1.0 },
			2 => normal.z = if center_diff.z > 0.0 { -1.0 } else { 1.0 },
			_ => unreachable!(),
		}
		
		Some(normal * min_overlap.abs())
	}
	
	/// Basic collision response that moves this AABB out of another AABB
	#[inline]
	pub fn resolve_collision(&mut self, other: &AABB) {
		if let Some(penetration) = self.penetration_vector(other) {
			*self = self.translate(penetration);
		}
	}
	
	/// Checks if this AABB completely contains another AABB
	#[inline]
	pub fn contains(&self, other: &AABB) -> bool {
		self.min.x <= other.min.x &&
		self.max.x >= other.max.x &&
		self.min.y <= other.min.y &&
		self.max.y >= other.max.y &&
		self.min.z <= other.min.z &&
		self.max.z >= other.max.z
	}
	
	/// Returns an AABB expanded by the given amount in all directions
	#[inline]
	pub fn expanded(&self, amount: Vec3) -> Self {
		Self {
			min: self.min - amount,
			max: self.max + amount,
		}
	}
	
	/// Returns an AABB expanded by the given scalar in all directions
	#[inline]
	pub fn expanded_scalar(&self, amount: f32) -> Self {
		self.expanded(Vec3::splat(amount))
	}
}
