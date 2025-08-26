
use crate::utils::math;
use crate::physic::aabb::AABB;
use crate::utils::input::{Keyboard, InputMapping};
use crate::utils::vec3;
use crate::ext::config::CameraConfig;
use crate::item::inventory;
use crate::physic::aabb;
use glam::{Vec3, Mat4, Quat};
use winit::dpi::PhysicalSize;
use wgpu::util::DeviceExt;

/// Movement mode enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum MovementMode {
	/// Movement is relative to camera orientation (default)
	CameraRelative,
	/// Movement is relative to world axes (ignores camera rotation)
	WorldRelative,
	/// Movement is relative to camera orientation (just not vertically)
	Flat,
}

/// Camera rotation mode enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraMode {
	/// Camera rotation is smoothly interpolated (default)
	Smooth,
	/// Camera rotation is set instantly to target
	Instant,
}

/// Represents a player with integrated camera system and movement capabilities
#[allow(dead_code)]
pub struct Player {
	pos: Vec3,
	config: CameraConfig,
	controller: PlayerController,
	movement_mode: MovementMode,
	camera_mode: CameraMode,
	inventory: inventory::Inventory,
	body: aabb::PhysicsBody,
	camera_system: CameraSystem,
}

const MOUSE_TO_SCREEN: f32 = 0.0056789;
const SAFE_FRAC_PI_2: f32 = std::f32::consts::FRAC_PI_2 - 0.0001;
const PLAYER_SIZE: Vec3 = Vec3::new(0.8,1.8,0.8);

#[allow(dead_code)]
impl Player {
	/// Creates a new player with default position and given camera configuration
	pub fn new(
		config: CameraConfig, 
		pos: Vec3, 
		device: &wgpu::Device, 
		size: PhysicalSize<u32>,
		bind_group_layout: &wgpu::BindGroupLayout,
	) -> Self {
		let aabb = aabb::AABB::from_pos(pos, PLAYER_SIZE);
		
		Self {
			pos,
			config,
			controller: PlayerController::new(config),
			movement_mode: MovementMode::Flat,
			camera_mode: CameraMode::Instant,
			inventory: inventory::Inventory::default(),
			body: aabb::PhysicsBody::new(aabb),
			camera_system: CameraSystem::new(device, size, config, bind_group_layout),
		}
	}

	#[cfg(test)]
	pub fn dummy(pos: Vec3, config: CameraConfig) -> Self {
		let aabb = aabb::AABB::from_pos(pos, Vec3::new(0.8,1.8,0.8));
		Self {
			pos,
			config,
			controller: PlayerController::new(config),
			movement_mode: MovementMode::Flat,
			camera_mode: CameraMode::Instant,
			inventory: inventory::Inventory::default(),
			body: aabb::PhysicsBody::new(aabb),
			camera_system: CameraSystem::dummy(),
		}
	}

	/// Updates player state and returns movement delta
	#[inline] pub fn update(&mut self, delta_time: f32, queue: &wgpu::Queue) -> Vec3 {
		// Clamp delta time to prevent physics issues with large frame times
		let dt = delta_time.min(0.01);

		self.update_rotation(dt);
		let movement = self.calculate_movement(dt);
		
		// Update the camera system's GPU resources
		self.camera_system.update(queue, self.cam_pos());
		
		movement
	}

	/// Updates camera rotation based on controller input
	fn update_rotation(&mut self, dt: f32) {
		// Apply mouse input to target rotation
		// mouse_x controls yaw (horizontal rotation)
		// mouse_y controls pitch (vertical rotation)
		self.controller.target_yaw -= self.controller.mouse_delta.x * self.config.sensitivity * MOUSE_TO_SCREEN;
		self.controller.target_pitch -= self.controller.mouse_delta.y * self.config.sensitivity * MOUSE_TO_SCREEN;
		
		// Clamp pitch to prevent over-rotation
		self.controller.target_pitch = self.controller.target_pitch.clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);

		match self.camera_mode {
			CameraMode::Smooth => {
				// Smooth rotation interpolation
				let smooth_factor = 1.0 - (-self.config.smoothness * dt).exp();
				self.controller.current_yaw = math::lerp_angle(
					self.controller.current_yaw,
					self.controller.target_yaw,
					smooth_factor
				);
				self.controller.current_pitch = math::lerp_f32(
					self.controller.current_pitch,
					self.controller.target_pitch,
					smooth_factor
				);
			}
			CameraMode::Instant => {
				// Set rotation instantly
				self.controller.current_yaw = self.controller.target_yaw;
				self.controller.current_pitch = self.controller.target_pitch;
			}
		}

		// Apply rotation to camera
		self.camera_system.camera_mut().set_rotation(Vec3::new(self.controller.current_pitch, self.controller.current_yaw, 0.));
		
		// Reset mouse delta for next frame
		self.controller.mouse_delta = Vec3::ZERO;
	}

	/// Calculates movement vector based on current inputs
	fn calculate_movement(&mut self, dt: f32) -> Vec3 {
		let run_multiplier = if self.controller.is_running() {
			self.config.run_multiplier
		} else {
			1.0
		};
		let speed = self.config.speed * run_multiplier;

		// Get movement direction from packed input
		let movement_dir = self.controller.get_direction();

		// Calculate target velocity based on movement mode
		let target_velocity = match self.movement_mode {
			MovementMode::CameraRelative => {
				// Relative to camera orientation
				self.camera_system.camera().right() * movement_dir.x 
					+ self.camera_system.camera().up() * movement_dir.y 
					+ self.camera_system.camera().forward() * movement_dir.z 
			}
			MovementMode::Flat => {
				// Relative to camera yaw only (ignores camera pitch) 
				self.camera_system.camera().right() * movement_dir.x 
					+ Vec3::Y * movement_dir.y 
					+ self.camera_system.camera().flat_forward() * movement_dir.z
			}
			MovementMode::WorldRelative => {
				// Relative to world axes 
				Vec3::X * movement_dir.x 
					+ Vec3::Y * movement_dir.y 
					+ Vec3::NEG_Z * movement_dir.z  // -Z is forward in right-handed system
			}
		} * speed;

		// Apply acceleration based on whether we're moving or stopping
		let acceleration = if target_velocity.length_squared() > 0.0 { 
			15.
		} else { 
			10.
		};
		
		self.controller.velocity = self.controller.velocity.lerp(
			target_velocity, 
			acceleration * dt
		);

		self.controller.velocity * dt
	}

	/// Gets the player's current position
	#[inline] pub const fn pos(&self) -> Vec3 { self.pos }
	#[inline] pub const fn cam_pos(&self) -> Vec3 {
		let pos = self.pos(); let off = self.config.offset;
		Vec3::new(pos.x + off.x, pos.y + off.y, pos.z + off.z)
	}

	/// Resizes the camera projection
	#[inline] pub fn append_position(&mut self, offset: Vec3) { self.pos += offset; }
	#[inline] pub fn resize(&mut self, new_size: PhysicalSize<u32>) { self.camera_system.resize(new_size); }

	/// Appends position to both player and camera
	#[inline] pub const fn controller(&self) -> &PlayerController { &self.controller }
	#[inline] pub const fn controller_mut(&mut self) -> &mut PlayerController { &mut self.controller }

	/// Sets the movement mode
	#[inline] pub const fn set_movement_mode(&mut self, mode: MovementMode) { self.movement_mode = mode; }
	#[inline] pub const fn set_camera_mode(&mut self, mode: CameraMode) { self.camera_mode = mode; }

	/// Handles zooming via mouse scroll
	#[inline] pub const fn camera(&self) -> &Camera { &self.camera_system.camera }
	#[inline] pub const fn camera_system(&self) -> &CameraSystem { &self.camera_system }
	#[inline] pub const fn camera_system_mut(&mut self) -> &mut CameraSystem { &mut self.camera_system }
	#[inline] pub const fn inventory(&self) -> &inventory::Inventory { &self.inventory }
	#[inline] pub const fn inventory_mut(&mut self) -> &mut inventory::Inventory { &mut self.inventory }
}

/// Handles player input and movement state
pub struct PlayerController {
	keyboard: Keyboard,
	input_mapping: InputMapping,
	mouse_delta: Vec3,     // Raw mouse input for this frame
	velocity: Vec3,
	target_yaw: f32,       // Target yaw angle
	target_pitch: f32,     // Target pitch angle
	current_yaw: f32,      // Current smoothed yaw angle
	current_pitch: f32,    // Current smoothed pitch angle
}

impl PlayerController {
	/// Creates a new controller with initial state from camera config
	#[inline] pub const fn new(config: CameraConfig) -> Self {
		Self {
			keyboard: Keyboard::default(),
			input_mapping: InputMapping::default(),
			mouse_delta: Vec3::ZERO,
			velocity: Vec3::ZERO,
			target_yaw: config.rotation.y,
			target_pitch: config.rotation.x,
			current_yaw: config.rotation.y,
			current_pitch: config.rotation.x,
		}
	}

	#[inline] pub fn is_running(&self) -> bool {
		let mapping = &self.input_mapping;
		let keyboard = &self.keyboard;
		(mapping.run)(keyboard)
	}

	pub fn get_direction(&self) -> Vec3 {
		let mapping = &self.input_mapping;
		let keyboard = &self.keyboard;
		
		let x = (mapping.right)(keyboard) as i8 - (mapping.left)(keyboard) as i8;
		let y = (mapping.up)(keyboard) as i8 - (mapping.down)(keyboard) as i8;
		let z = (mapping.forward)(keyboard) as i8 - (mapping.backward)(keyboard) as i8;
		
		Vec3::new(x as f32, y as f32, z as f32).normalize_or_zero()
	}

	/// Creates a controller with custom input mapping
	pub const fn with_configs(config: CameraConfig, input_mapping: InputMapping) -> Self {
		Self {
			keyboard: Keyboard::default(),
			input_mapping,
			mouse_delta: Vec3::ZERO,
			velocity: Vec3::ZERO,
			target_yaw: config.rotation.y,
			target_pitch: config.rotation.x,
			current_yaw: config.rotation.y,
			current_pitch: config.rotation.x,
		}
	}

	/// Processes keyboard input using the current input mapping
	#[inline] pub const fn process_keyboard(&mut self, keyboard: &Keyboard) {
		self.keyboard = *keyboard;
	}

	/// Updates the input mapping (useful for key remapping)
	#[inline] pub const fn set_input_mapping(&mut self, mapping: InputMapping) {
		self.input_mapping = mapping;
	}

	/// Gets a reference to the current input mapping
	#[inline] pub const fn input_mapping(&self) -> &InputMapping {
		&self.input_mapping
	}

	/// Processes mouse movement input
	#[inline] pub const fn process_mouse(&mut self, delta_x: f32, delta_y: f32) {
		self.mouse_delta = Vec3::new(delta_x, delta_y, 0.);
	}
}

// Uniform buffer data
#[repr(C, align(16))]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
	view_proj: [[f32; 4]; 4],
	position: [f32; 4],
}

impl CameraUniform {
	#[inline] pub fn default() -> Self {
		Self { 
			view_proj: Mat4::IDENTITY.to_cols_array_2d(),
			position: [0.0; 4],
		}
	}
	#[inline] pub const fn to_pos_vec3(&self) -> Vec3 {
		Vec3::new(self.position[0], self.position[1], self.position[2])
	}
}

// Camera system that manages camera, projection and GPU resources
pub struct CameraSystem {
	camera: Camera,
	projection: Projection,
	uniform: CameraUniform,
	frustum: Frustum,
	buffer: wgpu::Buffer,
	bind_group: wgpu::BindGroup,
}

impl CameraSystem {
	/// Creates a "dummy" `CameraSystem` with no real functionality.
	/// All fields are placeholders.
	#[cfg(test)]
	pub fn dummy() -> Self { unsafe {
		use std::mem::MaybeUninit;
		#[allow(invalid_value)] // i know it is invalid ... that's the reason for this entire function to make invalid quick non existing data
		Self {
			camera: Camera::default(),
			projection: Projection::default(),
			uniform: CameraUniform::default(),
			frustum: Frustum::default(),
			buffer: MaybeUninit::uninit().assume_init(),
			bind_group: MaybeUninit::uninit().assume_init(),
		}
	}}

	pub fn new(device: &wgpu::Device, size: PhysicalSize<u32>, config: CameraConfig, bind_group_layout: &wgpu::BindGroupLayout) -> Self {
		let camera = Camera::new(config.rotation);
		let projection = Projection::new(size, config.fovy, config.znear, config.zfar);
		let uniform = CameraUniform::default();
		let frustum = Frustum::default();
		
		let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Camera Buffer"),
			contents: bytemuck::cast_slice(&[uniform]),
			usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
		});

		let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			layout: &bind_group_layout,
			entries: &[wgpu::BindGroupEntry {
				binding: 0,
				resource: buffer.as_entire_binding(),
			}],
			label: Some("camera_bind_group"),
		});

		Self {
			camera,
			projection,
			uniform,
			frustum,
			buffer,
			bind_group,
		}
	}

	pub fn update(&mut self, queue: &wgpu::Queue, pos: Vec3) {
		let view_proj_mat = self.projection.matrix() * self.camera.view_matrix(pos);
		self.uniform.view_proj = view_proj_mat.to_cols_array_2d();
		self.uniform.position = pos.extend(0.0).into();
		
		// More efficient frustum update - directly from matrices instead of recalculating
		self.frustum = Frustum::from_view_proj_matrix(view_proj_mat);
		
		queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
	}

	#[inline] pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
		self.projection.resize(new_size);
	}

	// Getters
	
	#[inline] pub const fn frustum(&self) -> &Frustum { &self.frustum }
	#[inline] pub const fn camera(&self) -> &Camera { &self.camera }
	#[inline] pub const fn uniform(&self) -> &CameraUniform { &self.uniform }
	#[inline] pub const fn camera_mut(&mut self) -> &mut Camera { &mut self.camera }
	#[inline] pub const fn projection(&self) -> &Projection { &self.projection }
	#[inline] pub const fn projection_mut(&mut self) -> &mut Projection { &mut self.projection }
	#[inline] pub const fn bind_group(&self) -> &wgpu::BindGroup { &self.bind_group }
}

// Camera representation with improved rotation handling
#[derive(Debug)]
pub struct Camera {
	rotation: Vec3, // x: pitch, y: yaw, z: roll (unused)
}

impl Camera {
	#[inline] pub const fn new(rotation: Vec3) -> Self {
		Self { rotation }
	}
	#[inline] pub const fn default() -> Self {
		Self { rotation: Vec3::new(0.,0.,0.) }
	}

	pub fn view_matrix(&self, pos: Vec3) -> Mat4 {
		// Create rotation quaternion from yaw then pitch
		let yaw_quat = Quat::from_rotation_y(self.rotation.y);
		let pitch_quat = Quat::from_rotation_x(self.rotation.x);
		let _rotation_quat = yaw_quat * pitch_quat;
		
		// Create view matrix
		Mat4::look_to_rh(pos, self.forward(), self.up())
	}

	// Direction vectors with proper quaternion composition
	pub fn forward(&self) -> Vec3 {
		// Apply yaw first, then pitch
		let yaw_quat = Quat::from_rotation_y(self.rotation.y);
		let pitch_quat = Quat::from_rotation_x(self.rotation.x);
		let rotation_quat = yaw_quat * pitch_quat;
		rotation_quat * Vec3::NEG_Z
	}

	pub fn flat_forward(&self) -> Vec3 {
		// Only apply yaw rotation for flat forward
		let yaw_quat = Quat::from_rotation_y(self.rotation.y);
		(yaw_quat * Vec3::NEG_Z).normalize()
	}

	pub fn right(&self) -> Vec3 {
		// Right vector only depends on yaw
		let yaw_quat = Quat::from_rotation_y(self.rotation.y);
		yaw_quat * Vec3::X
	}

	pub fn up(&self) -> Vec3 {
		// Apply yaw first, then pitch
		let yaw_quat = Quat::from_rotation_y(self.rotation.y);
		let pitch_quat = Quat::from_rotation_x(self.rotation.x);
		let rotation_quat = yaw_quat * pitch_quat;
		rotation_quat * Vec3::Y
	}

	// Getters and setters
	#[inline] pub const fn rotation(&self) -> Vec3 { self.rotation }
	#[inline] pub const fn set_rotation(&mut self, rotation: Vec3) { self.rotation = rotation; }
}

/// Frustum plane indices for better code readability
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrustumPlane {
	Left = 0,
	Right = 1,
	Bottom = 2,
	Top = 3,
	Near = 4,
	Far = 5,
}

impl FrustumPlane {
	pub const ALL: [FrustumPlane; 6] = [
		FrustumPlane::Left,
		FrustumPlane::Right,
		FrustumPlane::Bottom,
		FrustumPlane::Top,
		FrustumPlane::Near,
		FrustumPlane::Far,
	];
}

// Represents a plane in 3D space using the equation ax + by + cz + d = 0
#[derive(Debug, Clone, Copy)]
pub struct Plane {
	pub normal: Vec3,
	pub distance: f32,
}

impl Plane {
	#[inline] pub const fn default() -> Self {
		Self {
			normal: Vec3::ZERO,
			distance: 0.,
		}
	}

	// Create normalized plane from 4 coefficients (a, b, c, d) where ax + by + cz + d = 0
	#[inline] pub const fn from_coefficients(a: f32, b: f32, c: f32, d: f32) -> Self {
		Self {
			normal: Vec3::new(a, b, c),
			distance: d,
		}
	}

	// Distance from point to plane (positive = in front, negative = behind)
	#[inline] pub const fn distance_to_point(&self, point: Vec3) -> f32 {
		vec3::const_dot(self.normal, point) + self.distance
	}
}

// Enhanced view frustum with better performance and usability
pub struct Frustum {
	planes: [Plane; 6]
}

// Macro to test a single plane
macro_rules! test_plane {
	($plane:expr, $center:expr, $extents:expr, $fully_inside:ident) => {
		{
			let distance = $plane.distance_to_point($center);
			let radius = $extents.x * $plane.normal.x.abs() +
						 $extents.y * $plane.normal.y.abs() +
						 $extents.z * $plane.normal.z.abs();
			
			if distance + radius < 0.0 { return None; }
			if distance - radius < 0.0 { $fully_inside = false; }
		}
	};
}
impl Frustum {
	#[inline] pub const fn default() -> Self {
		Self {
			planes: [Plane::default(); 6]
		}
	}

	/// Extract frustum planes and corners from view-projection matrix
	pub const fn from_view_proj_matrix(view_proj: Mat4) -> Self {
		let m = view_proj.to_cols_array_2d();
		
		// Extract and normalize the 6 frustum planes
		let mut planes = [Plane::default(); 6];
		
		// Left plane: m[3] + m[0]
		planes[0] = Self::normalize_plane(Plane::from_coefficients(
			m[3][0] + m[0][0],
			m[3][1] + m[0][1],
			m[3][2] + m[0][2],
			m[3][3] + m[0][3]
		));
		
		// Right plane: m[3] - m[0]
		planes[1] = Self::normalize_plane(Plane::from_coefficients(
			m[3][0] - m[0][0],
			m[3][1] - m[0][1],
			m[3][2] - m[0][2],
			m[3][3] - m[0][3]
		));
		
		// Bottom plane: m[3] + m[1]
		planes[2] = Self::normalize_plane(Plane::from_coefficients(
			m[3][0] + m[1][0],
			m[3][1] + m[1][1],
			m[3][2] + m[1][2],
			m[3][3] + m[1][3]
		));
		
		// Top plane: m[3] - m[1]
		planes[3] = Self::normalize_plane(Plane::from_coefficients(
			m[3][0] - m[1][0],
			m[3][1] - m[1][1],
			m[3][2] - m[1][2],
			m[3][3] - m[1][3]
		));
		
		// Near plane: m[3] + m[2]
		planes[4] = Self::normalize_plane(Plane::from_coefficients(
			m[3][0] + m[2][0],
			m[3][1] + m[2][1],
			m[3][2] + m[2][2],
			m[3][3] + m[2][3]
		));
		
		// Far plane: m[3] - m[2]
		planes[5] = Self::normalize_plane(Plane::from_coefficients(
			m[3][0] - m[2][0],
			m[3][1] - m[2][1],
			m[3][2] - m[2][2],
			m[3][3] - m[2][3]
		));

		Self { planes }
	}

	/// Helper function to normalize a plane
	const fn normalize_plane(plane: Plane) -> Plane {
		let length = vec3::const_length(plane.normal);
		if length > 0.001 {
			Plane {
				normal: vec3::const_div_s(plane.normal, length),
				distance: plane.distance / length,
			}
		} else {
			plane
		}
	}

	/// Test if AABB is inside or intersecting the frustum (optimized version)
	#[inline] pub const fn contains_aabb(&self, aabb: &AABB) -> bool {
		self.intersects_aabb(aabb).is_some()
	}

	/// More detailed AABB intersection test with early exit optimization
	/// Returns Some(true) if fully inside, Some(false) if intersecting, None if outside
	#[inline] pub const fn intersects_aabb(&self, aabb: &AABB) -> Option<bool> {
		let center = aabb.center();
		let extents = aabb.extents();
		let mut fully_inside = true;

		// Test all 6 planes using the macro
		test_plane!(self.planes[0], center, extents, fully_inside);
		test_plane!(self.planes[1], center, extents, fully_inside);
		test_plane!(self.planes[2], center, extents, fully_inside);
		test_plane!(self.planes[3], center, extents, fully_inside);
		test_plane!(self.planes[4], center, extents, fully_inside);
		test_plane!(self.planes[5], center, extents, fully_inside);

		Some(fully_inside)
	}
}

// Projection representation
pub struct Projection {
	aspect: f32,
	fovy: f32,
	znear: f32,
	zfar: f32,
	matrix: Mat4,
}

impl Projection {
	pub fn new(size: PhysicalSize<u32>, fovy: f32, znear: f32, zfar: f32) -> Self {
		let aspect = size.width as f32 / size.height as f32;
		Self {
			aspect,
			fovy,
			znear,
			zfar,
			matrix: Mat4::perspective_rh(fovy, aspect, znear, zfar),
		}
	}

	pub fn default() -> Self {
		Self {
			aspect: 1.,
			fovy: 30.,
			znear: 0.5,
			zfar: 1000.,
			matrix: Mat4::perspective_rh(30., 1., 1., 100.),
		}
	}

	#[inline] pub fn resize(&mut self, size: PhysicalSize<u32>) {
		self.aspect = size.width as f32 / size.height as f32;
		self.update_matrix();
	}

	#[inline] pub fn set_fovy(&mut self, fovy: f32) {
		self.fovy = fovy;
		self.update_matrix();
	}

	#[inline] fn update_matrix(&mut self) {
		self.matrix = Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar);
	}

	// Getters
	#[inline] pub const fn matrix(&self) -> Mat4 { self.matrix }
	#[inline] pub const fn aspect(&self) -> f32 { self.aspect }
	#[inline] pub const fn fovy(&self) -> f32 { self.fovy }
	#[inline] pub const fn znear(&self) -> f32 { self.znear }
	#[inline] pub const fn zfar(&self) -> f32 { self.zfar }
}
