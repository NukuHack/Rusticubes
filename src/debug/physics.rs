#[cfg(test)]
use crate::physic::aabb::{self, AABB};
#[cfg(test)]
use crate::physic::body::{self, PhysicsBody};

#[cfg(test)]
mod tests {
	use super::*;
	use glam::vec3;
	
	#[test]
	fn test_intersects() {
		let a = AABB::new(vec3(0.0, 0.0, 0.0), vec3(10.0, 10.0, 10.0));
		let b = AABB::new(vec3(5.0, 5.0, 5.0), vec3(15.0, 15.0, 15.0));
		assert!(a.intersects(&b));
		
		let c = AABB::new(vec3(11.0, 11.0, 11.0), vec3(20.0, 20.0, 20.0));
		assert!(!a.intersects(&c));
	}
	
	#[test]
	fn test_penetration_vector() {
		let a = AABB::new(vec3(0.0, 0.0, 0.0), vec3(10.0, 10.0, 10.0));
		let b = AABB::new(vec3(8.0, 8.0, 8.0), vec3(18.0, 18.0, 18.0));
		let penetration = a.penetration_vector(&b).unwrap();
		assert!(penetration.x < 0.0 && penetration.y == 0.0 && penetration.z == 0.0);
		
		let c = AABB::new(vec3(8.0, -2.0, 8.0), vec3(18.0, 8.0, 18.0));
		let penetration = a.penetration_vector(&c).unwrap();
		assert!(penetration.x == 0.0 && penetration.y > 0.0 && penetration.z == 0.0);
	}
	
	#[test]
	fn test_physics_update() {
		let mut body = PhysicsBody {
			aabb: AABB::new(vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 1.0)),
			velocity: vec3(1.0, 0.0, 0.0),
			acceleration: vec3(0.0, -9.8, 0.0),
			restitution: 0.5,
			friction: 0.2,
		};
		
		body.update(1.0, vec3(0.0, -9.8, 0.0));
		assert!(body.aabb.min.x > 0.0);
		assert!(body.velocity.y < 0.0);
	}
}