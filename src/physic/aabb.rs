use glam::{Vec3, IVec3};

/// Default gravity constant (Earth gravity: 9.8 m/s²)
pub const GRAVITY: Vec3 = Vec3::new(0.0, -9.8, 0.0);

/// Axis-Aligned Bounding Box for collision detection and spatial queries
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AABB {
    pub min: Vec3,
    pub max: Vec3,
}

impl AABB {
    /// Creates a new AABB from minimum and maximum coordinates
    #[inline]
    pub const fn new(min: Vec3, max: Vec3) -> Self {
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
    
    /// Creates an AABB from integer coordinates (useful for voxel/grid systems)
    #[inline]
    pub fn from_ivec(min: IVec3, max: IVec3) -> Self {
        Self {
            min: min.as_vec3(),
            max: max.as_vec3(),
        }
    }
    
    /// Creates an AABB with the given size centered at the origin
    #[inline]
    pub fn from_size(size: Vec3) -> Self {
        let half_size = size * 0.5;
        Self::from_center(Vec3::ZERO, half_size)
    }
}

impl AABB {
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
    
    /// Returns the surface area of the AABB
    #[inline]
    pub fn surface_area(&self) -> f32 {
        let dims = self.dimensions();
        2.0 * (dims.x * dims.y + dims.y * dims.z + dims.z * dims.x)
    }
    
    /// Returns the volume of the AABB
    #[inline]
    pub fn volume(&self) -> f32 {
        let dims = self.dimensions();
        dims.x * dims.y * dims.z
    }
    
    /// Checks if the AABB is valid (min <= max for all axes)
    #[inline]
    pub fn is_valid(&self) -> bool {
        self.min.x <= self.max.x && 
        self.min.y <= self.max.y && 
        self.min.z <= self.max.z
    }
}

impl AABB {
    /// Checks if this AABB overlaps with another AABB
    #[inline]
    pub const fn intersects(&self, other: &Self) -> bool {
        self.min.x <= other.max.x &&
        self.max.x >= other.min.x &&
        self.min.y <= other.max.y &&
        self.max.y >= other.min.y &&
        self.min.z <= other.max.z &&
        self.max.z >= other.min.z
    }
    
    /// Checks if a point is inside the AABB (inclusive)
    #[inline]
    pub const fn contains_point(&self, point: Vec3) -> bool {
        point.x >= self.min.x && point.x <= self.max.x &&
        point.y >= self.min.y && point.y <= self.max.y &&
        point.z >= self.min.z && point.z <= self.max.z
    }
    
    /// Checks if this AABB completely contains another AABB
    #[inline]
    pub const fn contains(&self, other: &Self) -> bool {
        self.min.x <= other.min.x && self.max.x >= other.max.x &&
        self.min.y <= other.min.y && self.max.y >= other.max.y &&
        self.min.z <= other.min.z && self.max.z >= other.max.z
    }
    
    /// Returns the distance squared from a point to this AABB (0 if point is inside)
    pub fn distance_squared_to_point(&self, point: Vec3) -> f32 {
        let dx = (point.x - self.max.x).max(0.0).max(self.min.x - point.x);
        let dy = (point.y - self.max.y).max(0.0).max(self.min.y - point.y);
        let dz = (point.z - self.max.z).max(0.0).max(self.min.z - point.z);
        dx * dx + dy * dy + dz * dz
    }
}

impl AABB {
    /// Returns a new AABB translated by the given vector
    #[inline]
    pub fn translate(&self, translation: Vec3) -> Self {
        Self {
            min: self.min + translation,
            max: self.max + translation,
        }
    }
    
    /// Returns the smallest AABB that contains both this and another AABB
    #[inline]
    pub fn union(&self, other: &Self) -> Self {
        Self {
            min: self.min.min(other.min),
            max: self.max.max(other.max),
        }
    }
    
    /// Returns the intersection of this AABB with another, or None if they don't intersect
    pub fn intersection(&self, other: &Self) -> Option<Self> {
        if !self.intersects(other) {
            return None;
        }
        
        Some(Self {
            min: self.min.max(other.min),
            max: self.max.min(other.max),
        })
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
    pub fn expanded_uniform(&self, amount: f32) -> Self {
        self.expanded(Vec3::splat(amount))
    }
    
    /// Returns an AABB scaled from its center
    #[inline]
    pub fn scaled(&self, scale: f32) -> Self {
        let center = self.center();
        let half_extents = self.half_extents() * scale;
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
}

impl PhysicsBody {
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
        let drag_coefficient: f32 = 0.98;
        self.velocity *= drag_coefficient.powf(dt);
        
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
}

impl PhysicsBody {
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