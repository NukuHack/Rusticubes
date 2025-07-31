#[cfg(test)]
mod tests {
	use super::*;
	use glam::{vec3, Vec3};
	use crate::physic::aabb::{AABB, PhysicsBody, GRAVITY};

	// AABB Creation Tests
	#[test]
	fn test_aabb_creation() {
		let aabb = AABB::new(vec3(0.0, 0.0, 0.0), vec3(10.0, 10.0, 10.0));
		assert_eq!(aabb.min, vec3(0.0, 0.0, 0.0));
		assert_eq!(aabb.max, vec3(10.0, 10.0, 10.0));
		
		let aabb_from_center = AABB::from_center(vec3(5.0, 5.0, 5.0), vec3(2.5, 2.5, 2.5));
		assert_eq!(aabb_from_center.min, vec3(2.5, 2.5, 2.5));
		assert_eq!(aabb_from_center.max, vec3(7.5, 7.5, 7.5));
		
		let aabb_from_size = AABB::from_size(vec3(4.0, 6.0, 8.0));
		assert_eq!(aabb_from_size.min, vec3(-2.0, -3.0, -4.0));
		assert_eq!(aabb_from_size.max, vec3(2.0, 3.0, 4.0));
	}

	// AABB Property Tests
	#[test]
	fn test_aabb_properties() {
		let aabb = AABB::new(vec3(1.0, 2.0, 3.0), vec3(5.0, 8.0, 9.0));
		
		assert_eq!(aabb.dimensions(), vec3(4.0, 6.0, 6.0));
		assert_eq!(aabb.center(), vec3(3.0, 5.0, 6.0));
		assert_eq!(aabb.half_extents(), vec3(2.0, 3.0, 3.0));
		
		let expected_surface_area = 2.0 * (4.0 * 6.0 + 6.0 * 6.0 + 6.0 * 4.0);
		assert!((aabb.surface_area() - expected_surface_area).abs() < f32::EPSILON);
		
		let expected_volume = 4.0 * 6.0 * 6.0;
		assert!((aabb.volume() - expected_volume).abs() < f32::EPSILON);
		
		assert!(aabb.is_valid());
		
		let invalid_aabb = AABB::new(vec3(5.0, 5.0, 5.0), vec3(1.0, 1.0, 1.0));
		assert!(!invalid_aabb.is_valid());
	}

	// AABB Intersection Tests
	#[test]
	fn test_aabb_intersects() {
		let a = AABB::new(vec3(0.0, 0.0, 0.0), vec3(10.0, 10.0, 10.0));
		let b = AABB::new(vec3(5.0, 5.0, 5.0), vec3(15.0, 15.0, 15.0));
		assert!(a.intersects(&b));
		
		let c = AABB::new(vec3(11.0, 11.0, 11.0), vec3(20.0, 20.0, 20.0));
		assert!(!a.intersects(&c));
		
		// Edge case: touching AABBs should intersect
		let d = AABB::new(vec3(10.0, 0.0, 0.0), vec3(20.0, 10.0, 10.0));
		assert!(a.intersects(&d));
		
		// Test self-intersection
		assert!(a.intersects(&a));
	}

	#[test]
	fn test_aabb_contains() {
		let outer = AABB::new(vec3(0.0, 0.0, 0.0), vec3(10.0, 10.0, 10.0));
		let inner = AABB::new(vec3(2.0, 2.0, 2.0), vec3(8.0, 8.0, 8.0));
		let overlapping = AABB::new(vec3(5.0, 5.0, 5.0), vec3(15.0, 15.0, 15.0));
		
		assert!(outer.contains(&inner));
		assert!(!outer.contains(&overlapping));
		assert!(outer.contains(&outer)); // Self-containment
		
		// Point containment tests
		assert!(outer.contains_point(vec3(5.0, 5.0, 5.0)));
		assert!(outer.contains_point(vec3(0.0, 0.0, 0.0))); // Edge point
		assert!(!outer.contains_point(vec3(11.0, 5.0, 5.0)));
	}

	#[test]
	fn test_aabb_distance() {
		let aabb = AABB::new(vec3(0.0, 0.0, 0.0), vec3(10.0, 10.0, 10.0));
		
		// Point inside AABB should have distance 0
		assert_eq!(aabb.distance_squared_to_point(vec3(5.0, 5.0, 5.0)), 0.0);
		
		// Point outside on one axis
		let dist_sq = aabb.distance_squared_to_point(vec3(15.0, 5.0, 5.0));
		assert!((dist_sq - 25.0).abs() < f32::EPSILON); // Distance = 5, squared = 25
		
		// Point outside on multiple axes
		let dist_sq = aabb.distance_squared_to_point(vec3(13.0, 14.0, 5.0));
		assert!((dist_sq - 25.0).abs() < f32::EPSILON); // sqrt(3^2 + 4^2) = 5, squared = 25
	}

	// AABB Manipulation Tests
	#[test]
	fn test_aabb_operations() {
		let a = AABB::new(vec3(0.0, 0.0, 0.0), vec3(10.0, 10.0, 10.0));
		let b = AABB::new(vec3(5.0, 5.0, 5.0), vec3(15.0, 15.0, 15.0));
		
		// Translation
		let translated = a.translate(vec3(2.0, 3.0, 4.0));
		assert_eq!(translated.min, vec3(2.0, 3.0, 4.0));
		assert_eq!(translated.max, vec3(12.0, 13.0, 14.0));
		
		// Union
		let union = a.union(&b);
		assert_eq!(union.min, vec3(0.0, 0.0, 0.0));
		assert_eq!(union.max, vec3(15.0, 15.0, 15.0));
		
		// Intersection
		let intersection = a.intersection(&b).unwrap();
		assert_eq!(intersection.min, vec3(5.0, 5.0, 5.0));
		assert_eq!(intersection.max, vec3(10.0, 10.0, 10.0));
		
		// Non-intersecting AABBs
		let c = AABB::new(vec3(20.0, 20.0, 20.0), vec3(30.0, 30.0, 30.0));
		assert!(a.intersection(&c).is_none());
		
		// Expansion
		let expanded = a.expanded(vec3(1.0, 2.0, 3.0));
		assert_eq!(expanded.min, vec3(-1.0, -2.0, -3.0));
		assert_eq!(expanded.max, vec3(11.0, 12.0, 13.0));
		
		let expanded_uniform = a.expanded_uniform(2.0);
		assert_eq!(expanded_uniform.min, vec3(-2.0, -2.0, -2.0));
		assert_eq!(expanded_uniform.max, vec3(12.0, 12.0, 12.0));
		
		// Scaling
		let scaled = a.scaled(2.0);
		let expected_center = a.center();
		let expected_half_extents = a.half_extents() * 2.0;
		assert_eq!(scaled.center(), expected_center);
		assert_eq!(scaled.half_extents(), expected_half_extents);
	}

	// Penetration Vector Tests
	#[test]
	fn test_penetration_vector() {
		let a = AABB::new(vec3(0.0, 0.0, 0.0), vec3(10.0, 10.0, 10.0));
		
		// Overlap on X-axis (minimal penetration)
		let b = AABB::new(vec3(8.0, 2.0, 2.0), vec3(18.0, 8.0, 8.0));
		let penetration = a.penetration_vector(&b).unwrap();
		assert!(penetration.x < 0.0); // Should separate on X-axis
		assert_eq!(penetration.y, 0.0);
		assert_eq!(penetration.z, 0.0);
		assert!((penetration.x + 2.0).abs() < f32::EPSILON); // Overlap is 2 units
		
		// Overlap on Y-axis (minimal penetration)
		let c = AABB::new(vec3(2.0, 8.0, 2.0), vec3(8.0, 18.0, 8.0));
		let penetration = a.penetration_vector(&c).unwrap();
		assert_eq!(penetration.x, 0.0);
		assert!(penetration.y < 0.0); // Should separate on Y-axis
		assert_eq!(penetration.z, 0.0);
		
		// No intersection
		let d = AABB::new(vec3(20.0, 20.0, 20.0), vec3(30.0, 30.0, 30.0));
		assert!(a.penetration_vector(&d).is_none());
		
		// Complete overlap (one inside another)
		let e = AABB::new(vec3(2.0, 2.0, 2.0), vec3(8.0, 8.0, 8.0));
		let penetration = a.penetration_vector(&e);
		assert!(penetration.is_some());
	}

	#[test]
	fn test_collision_resolution() {
		let mut a = AABB::new(vec3(0.0, 0.0, 0.0), vec3(10.0, 10.0, 10.0));
		let b = AABB::new(vec3(8.0, 2.0, 2.0), vec3(18.0, 8.0, 8.0));
		
		let original_pos = a.center();
		let resolved = a.resolve_collision(&b);
		
		assert!(resolved);
		assert!(a.center().x < original_pos.x); // Should move left
		assert!(!a.intersects(&b)); // Should no longer intersect
	}

	// PhysicsBody Creation Tests
	#[test]
	fn test_physics_body_creation() {
		let aabb = AABB::new(vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 1.0));
		
		let body = PhysicsBody::new(aabb);
		assert_eq!(body.aabb, aabb);
		assert_eq!(body.velocity, Vec3::ZERO);
		assert_eq!(body.mass, 1.0);
		assert!(!body.is_kinematic);
		assert!(!body.is_grounded);
		
		let kinematic = PhysicsBody::new_kinematic(aabb);
		assert!(kinematic.is_kinematic);
		assert_eq!(kinematic.mass, f32::INFINITY);
		
		let custom = PhysicsBody::with_properties(aabb, 2.0, 0.8, 0.3);
		assert_eq!(custom.mass, 2.0);
		assert_eq!(custom.restitution, 0.8);
		assert_eq!(custom.friction, 0.3);
	}

	// PhysicsBody Update Tests
	#[test]
	fn test_physics_update() {
		let mut body = PhysicsBody::new(AABB::new(vec3(0.0, 10.0, 0.0), vec3(1.0, 11.0, 1.0)));
		body.velocity = vec3(5.0, 0.0, 0.0);
		
		let initial_pos = body.aabb.center();
		body.update(1.0, GRAVITY);
		
		// Should move right due to initial velocity
		assert!(body.aabb.center().x > initial_pos.x);
		// Should fall due to gravity
		assert!(body.aabb.center().y < initial_pos.y);
		// Velocity should be affected by gravity
		assert!(body.velocity.y < 0.0);
		
		// Test kinematic body doesn't move
		let mut kinematic = PhysicsBody::new_kinematic(AABB::new(vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 1.0)));
		let initial_kinematic_pos = kinematic.aabb.center();
		kinematic.update(1.0, GRAVITY);
		assert_eq!(kinematic.aabb.center(), initial_kinematic_pos);
	}

	#[test]
	fn test_physics_forces() {
		let mut body = PhysicsBody::new(AABB::new(vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 1.0)));
		
		// Test impulse
		body.apply_impulse(vec3(10.0, 0.0, 0.0));
		assert_eq!(body.velocity, vec3(10.0, 0.0, 0.0)); // mass = 1.0
		
		// Test force (should affect acceleration)
		body.apply_force(vec3(0.0, 5.0, 0.0));
		assert_eq!(body.acceleration, vec3(0.0, 5.0, 0.0));
		
		// Test velocity setting
		body.set_velocity(vec3(2.0, 3.0, 4.0));
		assert_eq!(body.velocity, vec3(2.0, 3.0, 4.0));
		
		// Test kinetic energy
		let ke = body.kinetic_energy();
		let expected_ke = 0.5 * 1.0 * (2.0*2.0 + 3.0*3.0 + 4.0*4.0); // 0.5 * m * v²
		assert!((ke - expected_ke).abs() < f32::EPSILON);
		
		// Test kinematic body doesn't respond to forces
		let mut kinematic = PhysicsBody::new_kinematic(AABB::new(vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 1.0)));
		kinematic.apply_impulse(vec3(100.0, 0.0, 0.0));
		assert_eq!(kinematic.velocity, Vec3::ZERO);
	}

	// Collision Response Tests
	#[test]
	fn test_physics_collision_with_aabb() {
		let mut body = PhysicsBody::new(AABB::new(vec3(8.0, 0.0, 0.0), vec3(12.0, 4.0, 4.0)));
		body.velocity = vec3(-5.0, 0.0, 0.0); // Moving left
		body.restitution = 0.5;
		
		let obstacle = AABB::new(vec3(0.0, 0.0, 0.0), vec3(10.0, 10.0, 10.0));
		let penetration = body.resolve_collision_with_aabb(&obstacle);
		
		assert!(penetration.is_some());
		assert!(!body.aabb.intersects(&obstacle)); // Should be separated
		assert!(body.velocity.x > 0.0); // Should bounce back (right direction)
		
		// Test kinematic body doesn't respond
		let mut kinematic = PhysicsBody::new_kinematic(AABB::new(vec3(8.0, 0.0, 0.0), vec3(12.0, 4.0, 4.0)));
		let original_pos = kinematic.aabb.center();
		let result = kinematic.resolve_collision_with_aabb(&obstacle);
		assert!(result.is_none());
		assert_eq!(kinematic.aabb.center(), original_pos);
	}

	#[test]
	fn test_physics_collision_between_bodies() {
		let mut body1 = PhysicsBody::new(AABB::new(vec3(0.0, 0.0, 0.0), vec3(2.0, 2.0, 2.0)));
		let mut body2 = PhysicsBody::new(AABB::new(vec3(1.5, 0.0, 0.0), vec3(3.5, 2.0, 2.0)));
		
		body1.velocity = vec3(5.0, 0.0, 0.0);
		body2.velocity = vec3(-2.0, 0.0, 0.0);
		body1.mass = 2.0;
		body2.mass = 1.0;
		
		let result = body1.resolve_collision_with_body(&mut body2);
		
		assert!(result.is_some());
		assert!(!body1.aabb.intersects(&body2.aabb)); // Should be separated
		
		// Conservation of momentum check (approximately)
		let initial_momentum = 2.0 * 5.0 + 1.0 * (-2.0); // 8.0
		let final_momentum = body1.mass * body1.velocity.x + body2.mass * body2.velocity.x;
		assert!((initial_momentum - final_momentum).abs() < 0.1);
	}

	#[test]
	fn test_grounding_detection() {
		let mut body = PhysicsBody::new(AABB::new(vec3(0.0, 5.0, 0.0), vec3(2.0, 7.0, 2.0)));
		body.velocity = vec3(0.0, -1.0, 0.0); // Falling
		
		let ground = AABB::new(vec3(-10.0, 0.0, -10.0), vec3(10.0, 5.0, 10.0));
		
		body.resolve_collision_with_aabb(&ground);
		assert!(body.is_grounded); // Should detect ground collision
		
		// Test that steep slopes don't count as ground
		body.is_grounded = false;
		let steep_slope = AABB::new(vec3(-10.0, 0.0, -10.0), vec3(10.0, 20.0, 10.0));
		// This would require adjusting the collision normal, but the basic test structure is here
	}

	#[test]
	fn test_drag_coefficient() {
		let mut body = PhysicsBody::new(AABB::new(vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 1.0)));
		body.velocity = vec3(10.0, 0.0, 0.0);
		body.is_grounded = true; // Prevent gravity from interfering
		
		let initial_speed = body.velocity.length();
		body.update(1.0, Vec3::ZERO); // No gravity
		let final_speed = body.velocity.length();
		
		// Speed should decrease due to drag
		assert!(final_speed < initial_speed);
	}

	#[test]
	fn test_edge_cases() {
		// Test zero-size AABB
		let zero_aabb = AABB::new(vec3(5.0, 5.0, 5.0), vec3(5.0, 5.0, 5.0));
		assert!(zero_aabb.is_valid());
		assert_eq!(zero_aabb.volume(), 0.0);
		assert_eq!(zero_aabb.surface_area(), 0.0);
		
		// Test very small timestep
		let mut body = PhysicsBody::new(AABB::new(vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 1.0)));
		body.velocity = vec3(1.0, 1.0, 1.0);
		let initial_pos = body.aabb.center();
		
		body.update(0.001, Vec3::ZERO);
		let movement = body.aabb.center() - initial_pos;
		assert!(movement.length() < 0.01); // Very small movement for small timestep
		
		// Test zero mass (should not crash, but also not move)
		let mut zero_mass_body = PhysicsBody::new(AABB::new(vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 1.0)));
		zero_mass_body.mass = 0.0;
		zero_mass_body.apply_force(vec3(1000.0, 0.0, 0.0));
		zero_mass_body.update(1.0, GRAVITY);
		// Should not crash and should not move due to zero mass
	}
}