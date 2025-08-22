
use crate::block::math::ChunkCoord;
use crate::block::main::Chunk;
use crate::utils::vec3;
use glam::{Vec3, IVec3};

/// Default gravity constant (Earth gravity: 9.8 m/s²)
pub const GRAVITY: Vec3 = Vec3::new(0.0, -9.8, 0.0);
pub const DRAG_COEFFICIENT: f32 = 0.98;

/// Axis-Aligned Bounding Box for collision detection and spatial queries
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AABB {
	pub min: Vec3,
	pub max: Vec3,
}

impl AABB {
	/// Creates a new AABB from minimum and maximum coordinates
	#[inline] pub const fn new(min: Vec3, max: Vec3) -> Self {
		Self { min, max }
	}

	/// Creates a new AABB from position and size coordinates
	#[inline] pub const fn from_pos(pos: Vec3, size: Vec3) -> Self {
		let min = Vec3::new(pos.x-size.x,pos.y,pos.z-size.z);
		let max = Vec3::new(pos.x+size.x,pos.y+size.y,pos.z+size.z);
		Self { min, max }
	}
	
	/// Creates an AABB from center position and half-extents
	#[inline] pub const fn from_center(center: Vec3, half_extents: Vec3) -> Self {
		Self {
			min: vec3::const_sub(center, half_extents),
			max: vec3::const_add(center, half_extents),
		}
	}
	
	/// Creates an AABB from integer coordinates (useful for voxel/grid systems)
	#[inline] pub const fn from_ivec(min: IVec3, max: IVec3) -> Self {
		Self {
			min: Vec3::new(min.x as f32, min.y as f32, min.z as f32),
			max: Vec3::new(max.x as f32, max.y as f32, max.z as f32),
		}
	}

	// Create AABB for a chunk at given coordinates with chunk size
	#[inline] pub const fn from_chunk_coord(chunk_coord: &ChunkCoord) -> Self {
		let chunk_size = Chunk::SIZE_F;
		let min = Vec3::new(
			chunk_coord.x() as f32 * chunk_size,
			chunk_coord.y() as f32 * chunk_size,
			chunk_coord.z() as f32 * chunk_size,
		);
		let max = Vec3::new(min.x + chunk_size, min.y + chunk_size, min.z + chunk_size);
		Self { min, max }
	}
	
	/// Creates an AABB with the given size centered at the origin
	#[inline] pub const fn from_size(size: Vec3) -> Self {
		let half_size = vec3::const_mul_s(size, 0.5);
		Self::from_center(Vec3::ZERO, half_size)
	}


	/// Returns the dimensions (width, height, depth) of the AABB
	#[inline] pub const fn dimensions(&self) -> Vec3 {
		Vec3::new(self.max.x - self.min.x , self.max.y - self.min.y ,self.max.z - self.min.z)
	}
	
	/// Returns the center position of the AABB
	#[inline] pub const fn center(&self) -> Vec3 {
		Vec3::new((self.max.x + self.min.x)*0.5, (self.max.y + self.min.y)*0.5, (self.max.z + self.min.z)*0.5)
	}
	
	/// Returns the half-extents of the AABB
	#[inline] pub const fn extents(&self) -> Vec3 {
		let dim = self.dimensions();
		Vec3::new(dim.x*0.5,dim.y*0.5,dim.z*0.5)
	}
	
	/// Returns the surface area of the AABB
	#[inline] pub const fn surface_area(&self) -> f32 {
		let dims = self.dimensions();
		2.0 * (dims.x * dims.y + dims.y * dims.z + dims.z * dims.x)
	}
	
	/// Returns the volume of the AABB
	#[inline] pub const fn volume(&self) -> f32 {
		let dims = self.dimensions();
		dims.x * dims.y * dims.z
	}
	
	/// Checks if the AABB is valid (min <= max for all axes)
	#[inline] pub const fn is_valid(&self) -> bool {
		self.min.x <= self.max.x && 
		self.min.y <= self.max.y && 
		self.min.z <= self.max.z
	}


	/// Checks if this AABB overlaps with another AABB
	#[inline] pub const fn intersects(&self, other: &Self) -> bool {
		self.min.x <= other.max.x &&
		self.max.x >= other.min.x &&
		self.min.y <= other.max.y &&
		self.max.y >= other.min.y &&
		self.min.z <= other.max.z &&
		self.max.z >= other.min.z
	}
	
	/// Checks if a point is inside the AABB (inclusive)
	#[inline] pub const fn contains_point(&self, point: Vec3) -> bool {
		point.x >= self.min.x && point.x <= self.max.x &&
		point.y >= self.min.y && point.y <= self.max.y &&
		point.z >= self.min.z && point.z <= self.max.z
	}
	
	/// Checks if this AABB completely contains another AABB
	#[inline] pub const fn contains(&self, other: &Self) -> bool {
		self.min.x <= other.min.x && self.max.x >= other.max.x &&
		self.min.y <= other.min.y && self.max.y >= other.max.y &&
		self.min.z <= other.min.z && self.max.z >= other.max.z
	}
	
	/// Returns the distance squared from a point to this AABB (0 if point is inside)
	#[inline] pub const fn distance_squared_to_point(&self, point: Vec3) -> f32 {
		let dx = (point.x - self.max.x).max(0.0).max(self.min.x - point.x);
		let dy = (point.y - self.max.y).max(0.0).max(self.min.y - point.y);
		let dz = (point.z - self.max.z).max(0.0).max(self.min.z - point.z);
		dx * dx + dy * dy + dz * dz
	}


	/// Returns a new AABB translated by the given vector
	#[inline] pub const fn translate(&self, translation: Vec3) -> Self {
		Self {
			min: vec3::const_add(self.min, translation),
			max: vec3::const_add(self.max, translation),
		}
	}
	
	/// Returns the smallest AABB that contains both this and another AABB
	#[inline] pub const fn union(&self, other: &Self) -> Self {
		Self {
			min: vec3::const_min(self.min, other.min),
			max: vec3::const_max(self.max, other.max),
		}
	}
	
	/// Returns the intersection of this AABB with another, or None if they don't intersect
	#[inline] pub const fn intersection(&self, other: &Self) -> Option<Self> {
		if !self.intersects(other) { return None; }
		
		Some(Self {
			min: vec3::const_max(self.min, other.min),
			max: vec3::const_min(self.max, other.max),
		})
	}
	
	/// Returns an AABB expanded by the given amount in all directions
	#[inline] pub const fn expanded(&self, amount: Vec3) -> Self {
		Self {
			min: vec3::const_sub(self.min, amount),
			max: vec3::const_add(self.max, amount),
		}
	}
	
	/// Returns an AABB expanded by the given scalar in all directions
	#[inline] pub const fn expanded_uniform(&self, amount: f32) -> Self {
		self.expanded(Vec3::splat(amount))
	}
	
	/// Returns an AABB scaled from its center
	#[inline] pub const fn scaled(&self, scale: f32) -> Self {
		let center = self.center();
		let half_extents = vec3::const_mul_s(self.extents(), scale);
		Self::from_center(center, half_extents)
	}
}

/// Collision detection and response
impl AABB {
	/// Calculates the penetration vector when this AABB is colliding with another
	/// Returns the minimum translation vector to separate the AABBs
	pub fn penetration_vector(&self, other: &Self) -> Option<Vec3> {
		if !self.intersects(other) {
			return None;
		}
		
		// Calculate overlaps on each axis
		let overlap_x = (self.max.x.min(other.max.x) - self.min.x.max(other.min.x)).abs();
		let overlap_y = (self.max.y.min(other.max.y) - self.min.y.max(other.min.y)).abs();
		let overlap_z = (self.max.z.min(other.max.z) - self.min.z.max(other.min.z)).abs();
		
		// Find the axis with minimum penetration (shortest separation distance)
		let (min_overlap, axis) = if overlap_x <= overlap_y && overlap_x <= overlap_z {
			(overlap_x, 0)
		} else if overlap_y <= overlap_z {
			(overlap_y, 1)
		} else {
			(overlap_z, 2)
		};
		
		// Determine separation direction based on center positions
		let center_diff = other.center() - self.center();
		let mut separation = Vec3::ZERO;
		
		match axis {
			0 => separation.x = if center_diff.x > 0.0 { -min_overlap } else { min_overlap },
			1 => separation.y = if center_diff.y > 0.0 { -min_overlap } else { min_overlap },
			2 => separation.z = if center_diff.z > 0.0 { -min_overlap } else { min_overlap },
			_ => unreachable!(),
		}
		
		Some(separation)
	}
	
	/// Simple collision resolution that moves this AABB out of another
	#[inline]
	pub fn resolve_collision(&mut self, other: &Self) -> bool {
		if let Some(penetration) = self.penetration_vector(other) {
			*self = self.translate(penetration);
			true
		} else {
			false
		}
	}
}

/// A rigid body with physics properties for collision and movement simulation
#[derive(Debug, Clone)]
pub struct PhysicsBody {
	pub aabb: AABB,
	pub velocity: Vec3,
	pub acceleration: Vec3,
	pub is_grounded: bool,
	pub mass: f32,
	pub restitution: f32,        // Bounciness (0.0 = no bounce, 1.0 = perfect bounce)
	pub friction: f32,           // Friction coefficient (0.0 = no friction, 1.0 = full friction)
	pub collision_softness: f32, // Collision penetration allowance (1.0 = solid)
	pub is_kinematic: bool,      // If true, body is not affected by physics forces
}

impl PhysicsBody {
	/// Creates a new physics body with default properties
	#[inline]
	pub const fn new(aabb: AABB) -> Self {
		Self {
			aabb,
			velocity: Vec3::ZERO,
			acceleration: Vec3::ZERO,
			is_grounded: false,
			mass: 1.0,
			restitution: 0.5,
			friction: 0.5,
			collision_softness: 1.0,
			is_kinematic: false,
		}
	}
	
	/// Creates a kinematic body (not affected by physics forces)
	#[inline]
	pub fn new_kinematic(aabb: AABB) -> Self {
		Self {
			is_kinematic: true,
			mass: f32::INFINITY,
			..Self::new(aabb)
		}
	}
	
	/// Creates a physics body with custom properties
	pub fn with_properties(
		aabb: AABB,
		mass: f32,
		restitution: f32,
		friction: f32,
	) -> Self {
		Self {
			aabb,
			mass,
			restitution,
			friction,
			..Self::new(aabb)
		}
	}


	/// Updates the physics body for one timestep
	pub fn update(&mut self, dt: f32, gravity: Vec3) {
		if self.is_kinematic {
			return;
		}
		
		// Apply gravity
		self.acceleration += gravity;
		
		// Integrate velocity
		self.velocity += self.acceleration * dt;
		
		// Apply air resistance/drag (simple model)
		self.velocity *= DRAG_COEFFICIENT.powf(dt);
		
		// Integrate position
		self.aabb = self.aabb.translate(self.velocity * dt);
		
		// Reset acceleration for next frame
		self.acceleration = Vec3::ZERO;
	}
	
	/// Applies an instantaneous force impulse to the body
	#[inline]
	pub fn apply_impulse(&mut self, impulse: Vec3) {
		if !self.is_kinematic && self.mass > 0.0 {
			self.velocity += impulse / self.mass;
		}
	}
	
	/// Applies a continuous force to the body
	#[inline]
	pub fn apply_force(&mut self, force: Vec3) {
		if !self.is_kinematic && self.mass > 0.0 {
			self.acceleration += force / self.mass;
		}
	}
	
	/// Sets the body's velocity directly
	#[inline]
	pub fn set_velocity(&mut self, velocity: Vec3) {
		if !self.is_kinematic {
			self.velocity = velocity;
		}
	}
	
	/// Gets the kinetic energy of the body
	#[inline]
	pub fn kinetic_energy(&self) -> f32 {
		0.5 * self.mass * self.velocity.length_squared()
	}


	/// Resolves collision with another AABB and applies physics response
	pub fn resolve_collision_with_aabb(&mut self, other: &AABB) -> Option<Vec3> {
		if self.is_kinematic {
			return None;
		}
		
		let mut penetration = self.aabb.penetration_vector(other)?;
		
		// Apply collision softness
		if self.collision_softness < 1.0 {
			penetration *= self.collision_softness;
		}
		
		// Separate the objects
		self.aabb = self.aabb.translate(penetration);
		
		// Calculate collision response
		let normal = penetration.normalize_or_zero();
		let velocity_along_normal = self.velocity.dot(normal);
		
		// Don't resolve if objects are separating
		if velocity_along_normal > 0.0 {
			return Some(penetration);
		}
		
		// Apply restitution (bounciness)
		let restitution_impulse = -(1.0 + self.restitution) * velocity_along_normal;
		let restitution_velocity = normal * restitution_impulse;
		
		// Apply friction
		let tangent_velocity = self.velocity - velocity_along_normal * normal;
		let friction_velocity = tangent_velocity * self.friction;
		
		// Update velocity
		self.velocity = self.velocity + restitution_velocity - friction_velocity;
		
		// Update grounded state (check if collision was with ground)
		if normal.y > 0.7 { // Roughly 45-degree slope threshold
			self.is_grounded = true;
		}
		
		Some(penetration)
	}
	
	/// Resolves collision between two physics bodies
	pub fn resolve_collision_with_body(&mut self, other: &mut PhysicsBody) -> Option<Vec3> {
		if self.is_kinematic && other.is_kinematic {
			return None;
		}
		
		let penetration = self.aabb.penetration_vector(&other.aabb)?;
		let normal = penetration.normalize_or_zero();
		
		// Calculate relative velocity
		let relative_velocity = self.velocity - other.velocity;
		let velocity_along_normal = relative_velocity.dot(normal);
		
		// Don't resolve if objects are separating
		if velocity_along_normal > 0.0 {
			return Some(penetration);
		}
		
		// Calculate collision response
		let combined_restitution = (self.restitution + other.restitution) * 0.5;
		let impulse_magnitude = -(1.0 + combined_restitution) * velocity_along_normal;
		
		let total_mass = if self.is_kinematic {
			other.mass
		} else if other.is_kinematic {
			self.mass
		} else {
			self.mass + other.mass
		};
		
		let impulse = normal * impulse_magnitude / total_mass;
		
		// Apply impulses
		if !self.is_kinematic {
			self.velocity += impulse * other.mass;
			self.aabb = self.aabb.translate(penetration * (other.mass / total_mass));
		}
		
		if !other.is_kinematic {
			other.velocity -= impulse * self.mass;
			other.aabb = other.aabb.translate(-penetration * (self.mass / total_mass));
		}
		
		Some(penetration)
	}
}
