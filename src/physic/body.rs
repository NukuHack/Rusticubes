use crate::physic::aabb::{AABB};
use glam::Vec3;

/// Physics system functions
pub struct PhysicsBody {
	pub aabb: AABB,
	pub velocity: Vec3,
	pub acceleration: Vec3,
	pub restitution: f32, // Bounciness (0.0 = no bounce, 1.0 = perfect bounce)
	pub friction: f32,    // Friction coefficient (0.0 = no friction, 1.0 = full friction)
	pub collision_softness: f32, // How much penetration is allowed (1.0 = solid, 0.5 = half penetration, etc.)
}

impl PhysicsBody {
	#[inline]
	pub const fn new(aabb: AABB) -> Self {
		Self {
			aabb,
			velocity: Vec3::ZERO,
			acceleration: Vec3::ZERO,
			restitution: 0.5,
			friction: 0.5,
			collision_softness: 1.0, // Default to solid collisions
		}
	}

	#[inline]
	pub fn update(&mut self, dt: f32, gravity: Vec3) {
		// Apply gravity to acceleration
		self.acceleration += gravity;
		
		// Update velocity
		self.velocity += self.acceleration * dt;
		
		// Update position
		self.aabb = self.aabb.translate(self.velocity * dt);
		
		// Reset acceleration for next frame
		self.acceleration = Vec3::ZERO;
	}
	
	#[inline]
	pub fn apply_impulse(&mut self, impulse: Vec3) {
		self.velocity += impulse;
	}
	
	#[inline]
	pub fn resolve_collision(&mut self, other: &AABB) -> Option<Vec3> {
		if let Some(mut penetration) = self.aabb.penetration_vector(other) {
			// Apply softness factor - reduce the penetration vector
			if self.collision_softness < 1.0 {
				penetration *= self.collision_softness;
			}
			
			// Move out of collision (by the potentially reduced amount)
			self.aabb = self.aabb.translate(penetration);
			
			// Calculate reflection if we have velocity
			if self.velocity.length_squared() > 0.0 {
				let normal = penetration.normalize_or_zero();
				let velocity_along_normal = self.velocity.dot(normal);
				
				// Don't resolve if objects are moving apart
				if velocity_along_normal > 0.0 {
					return None;
				}
				
				// Apply bounce (restitution)
				let mut new_velocity = self.velocity - (1.0 + self.restitution) * velocity_along_normal * normal;
				
				// Apply friction
				let tangent_velocity = self.velocity - velocity_along_normal * normal;
				new_velocity -= tangent_velocity * self.friction;
				
				self.velocity = new_velocity;
			}
			
			return Some(penetration);
		}
		None
	}
}