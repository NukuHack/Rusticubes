
use glam::Vec3;
use winit::event::*;
use winit::keyboard::KeyCode as Key;

/// Movement mode enum
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MovementMode {
    /// Movement is relative to camera orientation (default)
    CameraRelative,
    /// Movement is relative to world axes (ignores camera rotation)
    WorldRelative,
    /// Movement is relative to camera orientation (just not vertically)
    Flat,
}

/// Camera rotation mode enum
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraMode {
    /// Camera rotation is smoothly interpolated (default)
    Smooth,
    /// Camera rotation is set instantly to target
    Instant,
}

/// Represents a player with camera controls and movement capabilities
pub struct Player {
    position: Vec3,
    config: CameraConfig,
    controller: PlayerController,
    movement_mode: MovementMode,
    camera_mode: CameraMode,
}

#[allow(dead_code)]
impl Player {
    /// Creates a new player with default position and given camera configuration
    pub fn new(config: CameraConfig) -> Self {
        Self {
            position: Vec3::ZERO,
            config,
            controller: PlayerController::new(config),
            movement_mode: MovementMode::Flat,
            camera_mode: CameraMode::Smooth,
        }
    }

    /// Gets the player's current position
    pub fn position(&self) -> Vec3 {
        self.position
    }

    /// Updates player state and returns movement delta
    pub fn update(
        &mut self,
        camera_system: &mut CameraSystem,
        delta_time: f32,
    ) -> Vec3 {
        // Clamp delta time to prevent physics issues with large frame times
        let dt = delta_time.min(0.1);

        // Split mutable borrows to avoid holding multiple mutable references
        let (camera, projection) = camera_system.split_mut();
        
        self.update_rotation(camera, dt);
        let movement = self.calculate_movement(camera, dt);
        
        self.apply_movement(movement, camera);
        self.handle_zoom(projection);
        
        movement
    }

    /// Appends position to both player and camera
    pub fn append_position(&mut self, offset: Vec3, camera: &mut Camera) {
        self.position += offset;
        camera.set_position(self.position);
    }

    pub fn controller(&mut self) -> &mut PlayerController {
        &mut self.controller
    }

    /// Sets the movement mode
    pub fn set_movement_mode(&mut self, mode: MovementMode) {
        self.movement_mode = mode;
    }

    /// Sets the camera mode
    pub fn set_camera_mode(&mut self, mode: CameraMode) {
        self.camera_mode = mode;
    }

    /// Updates camera rotation based on controller input
    fn update_rotation(&mut self, camera: &mut Camera, dt: f32) {
        // Apply rotation with sensitivity (fixed yaw/pitch mapping)
        self.controller.target_rotation += Vec3::new(
            -self.controller.rotation.y, // Pitch (vertical) from mouse Y
            -self.controller.rotation.x,  // Yaw (horizontal) from mouse X
            0.0
        ) * self.config.sensitivity * 0.05;
        
        // Clamp pitch to prevent over-rotation
        self.controller.target_rotation.x = self.controller.target_rotation.x
            .clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);

        match self.camera_mode {
            CameraMode::Smooth => {
                // Smooth rotation interpolation
                let smooth_factor = 1.0 - (-self.config.smoothness * dt).exp();
                self.controller.current_rotation = self.controller.current_rotation
                    .lerp(self.controller.target_rotation, smooth_factor);
            }
            CameraMode::Instant => {
                // Set rotation instantly
                self.controller.current_rotation = self.controller.target_rotation;
            }
        }

        camera.set_rotation(self.controller.current_rotation);
        self.controller.rotation = Vec3::ZERO;
    }

    /// Calculates movement vector based on current inputs
    fn calculate_movement(&mut self, camera: &Camera, dt: f32) -> Vec3 {
        let run_multiplier = if self.controller.movement.run {
            self.config.run_multiplier
        } else {
            1.0
        };
        let speed = self.config.speed * run_multiplier;

        // Calculate movement direction based on input
        let movement = Vec3::new(
            (self.controller.movement.right as i8 - self.controller.movement.left as i8) as f32,
            (self.controller.movement.up as i8 - self.controller.movement.down as i8) as f32,
            (self.controller.movement.forward as i8 - self.controller.movement.backward as i8) as f32,
        ).normalize_or_zero();

        // Calculate target velocity based on movement mode
        let target_velocity = match self.movement_mode {
            MovementMode::CameraRelative => {
                // Relative to camera orientation
                camera.right() * movement.x 
                    + camera.up() * movement.y 
                    + camera.forward() * movement.z
            }
            MovementMode::Flat => {
                let ff = camera.right() * movement.x + camera.forward() * movement.z;
                // Relative to world axes (ignores camera rotation)
                Vec3::new(ff.x,0.0,ff.z)
                    + Vec3::Y * movement.y 
            }
            MovementMode::WorldRelative => {
                // Relative to camera orientation
                Vec3::X * movement.x 
                    + Vec3::Y * movement.y 
                    + Vec3::Z * movement.z
            }
        } * speed;

        // Apply acceleration based on whether we're moving or stopping
        let acceleration = if target_velocity.length_squared() > 0.0 { 
            self.config.acceleration 
        } else { 
            self.config.deceleration 
        };
        
        self.controller.velocity = self.controller.velocity.lerp(
            target_velocity, 
            acceleration * dt
        );

        self.controller.velocity * dt
    }

    /// Applies movement to player position and camera
    fn apply_movement(&mut self, movement: Vec3, camera: &mut Camera) {
        self.position += movement;
        camera.set_position(self.position);
    }

    /// Handles zooming via mouse scroll
    fn handle_zoom(&mut self, projection: &mut Projection) {
        if self.controller.scroll.abs() > f32::EPSILON {
            let delta = self.controller.scroll * self.config.zoom_sensitivity;
            projection.set_fovy(
                (projection.fovy() - delta)
                    .clamp(self.config.min_fov, self.config.max_fov)
            );
            self.controller.scroll = 0.0;
        }
    }
}

/// Handles player input and movement state
pub struct PlayerController {
    movement: MovementInputs,
    rotation: Vec3,        // Stores raw mouse input (x = horizontal, y = vertical)
    scroll: f32,
    velocity: Vec3,
    target_rotation: Vec3, // Stores target Euler angles (x = pitch, y = yaw)
    current_rotation: Vec3, // Stores smoothed Euler angles
}

/// Tracks movement input states
#[derive(Debug, Default)]
pub struct MovementInputs {
    pub forward: bool,
    pub backward: bool,
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub run: bool,
}
#[allow(dead_code)]
impl PlayerController {
    /// Creates a new controller with initial state from camera config
    pub fn new(config: CameraConfig) -> Self {
        Self {
            movement: MovementInputs::default(),
            rotation: Vec3::ZERO,
            scroll: 0.0,
            velocity: Vec3::ZERO,
            target_rotation: config.rotation,
            current_rotation: config.rotation,
        }
    }

    /// Processes keyboard input and returns whether the key was handled
    pub fn process_keyboard(&mut self, key: &Key, state: &ElementState) -> bool {
        let is_pressed = *state == ElementState::Pressed;
        
        match key {
            Key::KeyW | Key::ArrowUp => self.movement.forward = is_pressed,
            Key::KeyS | Key::ArrowDown => self.movement.backward = is_pressed,
            Key::KeyA | Key::ArrowLeft => self.movement.left = is_pressed,
            Key::KeyD | Key::ArrowRight => self.movement.right = is_pressed,
            Key::Space => self.movement.up = is_pressed,
            Key::ShiftLeft => self.movement.run = is_pressed,
            Key::ControlLeft => self.movement.down = is_pressed,
            _ => return false,
        }
        true
    }

    /// Resets all keyboard inputs
    pub fn reset_keyboard(&mut self) {
        self.movement = MovementInputs::default();
    }

    /// Processes mouse movement input
    pub fn process_mouse(&mut self, delta_x: f32, delta_y: f32) {
        // Store raw mouse movement - we'll convert to rotation in update_rotation
        self.rotation = Vec3::new(delta_x, delta_y, 0.0);
    }

    /// Processes mouse scroll input
    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = match delta {
            MouseScrollDelta::LineDelta(_, y) => y * 0.5,
            MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.01,
        };
    }

    /// Resets controller state to match camera
    pub fn reset(&mut self, camera: &Camera) {
        self.movement = MovementInputs::default();
        self.velocity = Vec3::ZERO;
        self.target_rotation = camera.rotation();
        self.current_rotation = self.target_rotation;
    }
}

use glam::{Mat4, Quat};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

// Constants
pub const SAFE_FRAC_PI_2: f32 = std::f32::consts::FRAC_PI_2 - 0.0001;

// Uniform buffer data
#[repr(C, align(16))]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    position: [f32; 4],
}

impl Default for CameraUniform {
    fn default() -> Self {
        Self { 
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
            position: [0.0; 4],
        }
    }
}

impl CameraUniform {
    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        self.view_proj = (projection.matrix() * camera.view_matrix()).to_cols_array_2d();
        self.position = camera.position.extend(0.0).into();
    }
}

// Camera system that manages camera, projection and GPU resources
pub struct CameraSystem {
    camera: Camera,
    projection: Projection,
    uniform: CameraUniform,
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl CameraSystem {
    pub fn new(
        device: &wgpu::Device,
        size: PhysicalSize<u32>,
        config: CameraConfig,
    ) -> Self {
        let camera = Camera::new(config.position, config.rotation);
        let projection = Projection::new(size, config.fovy, config.znear, config.zfar);
        let mut uniform = CameraUniform::default();
        uniform.update_view_proj(&camera, &projection);
        
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
            label: Some("camera_bind_group_layout"),
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
            buffer,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.uniform.update_view_proj(&self.camera, &self.projection);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.projection.resize(new_size);
    }

    // Getters
    pub fn camera(&self) -> &Camera { &self.camera }
    pub fn camera_mut(&mut self) -> &mut Camera { &mut self.camera }
    pub fn projection(&self) -> &Projection { &self.projection }
    pub fn projection_mut(&mut self) -> &mut Projection { &mut self.projection }
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout { &self.bind_group_layout }
    pub fn bind_group(&self) -> &wgpu::BindGroup { &self.bind_group }

    pub fn split_mut(&mut self) -> (&mut Camera, &mut Projection) {
        (&mut self.camera, &mut self.projection)
    }
}

// Camera representation
#[derive(Debug)]
pub struct Camera {
    position: Vec3,
    rotation: Vec3, // x: pitch, y: yaw, z: roll (unused)
}

impl Camera {
    pub fn new(position: Vec3, rotation: Vec3) -> Self {
        Self { position, rotation }
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_rh(self.position, self.forward(), self.up())
    }

    // Direction vectors
    pub fn forward(&self) -> Vec3 {
        Quat::from_rotation_y(self.rotation.y) * 
        Quat::from_rotation_x(self.rotation.x) * 
        Vec3::NEG_Z
    }

    pub fn right(&self) -> Vec3 {
        Quat::from_rotation_y(self.rotation.y) * Vec3::X
    }

    pub fn up(&self) -> Vec3 {
        Quat::from_rotation_y(self.rotation.y) * 
        Quat::from_rotation_x(self.rotation.x) * 
        Vec3::Y
    }

    // Getters and setters
    pub fn position(&self) -> Vec3 { self.position }
    pub fn set_position(&mut self, position: Vec3) { self.position = position; }
    
    pub fn rotation(&self) -> Vec3 { self.rotation }
    pub fn set_rotation(&mut self, rotation: Vec3) { self.rotation = rotation; }
    
    pub fn translate(&mut self, translation: Vec3) { self.position += translation; }
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

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.aspect = size.width as f32 / size.height as f32;
        self.update_matrix();
    }

    pub fn set_fovy(&mut self, fovy: f32) {
        self.fovy = fovy;
        self.update_matrix();
    }

    fn update_matrix(&mut self) {
        self.matrix = Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar);
    }

    // Getters
    pub fn matrix(&self) -> Mat4 { self.matrix }
    pub fn aspect(&self) -> f32 { self.aspect }
    pub fn fovy(&self) -> f32 { self.fovy }
    pub fn znear(&self) -> f32 { self.znear }
    pub fn zfar(&self) -> f32 { self.zfar }
}

// Camera configuration
#[derive(Debug, Clone, Copy)]
pub struct CameraConfig {
    pub position: Vec3,
    pub rotation: Vec3,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
    pub speed: f32,
    pub sensitivity: f32,
    pub run_multiplier: f32,
    pub smoothness: f32,
    pub acceleration: f32,
    pub deceleration: f32,
    pub zoom_sensitivity: f32,
    pub min_fov: f32,
    pub max_fov: f32,
}

impl CameraConfig {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            rotation: Vec3::new(0.0, -std::f32::consts::FRAC_PI_2, 0.0), // Looking along negative Z axis
            fovy: std::f32::consts::FRAC_PI_2, // 90 degrees in radians
            znear: 0.01,
            zfar: 100.0,
            speed: 4.0,
            sensitivity: 0.5,
            run_multiplier: 2.5,
            smoothness: 5.0,
            acceleration: 10.0,
            deceleration: 15.0,
            zoom_sensitivity: 0.1,
            min_fov: std::f32::consts::FRAC_PI_6, // 30 degrees
            max_fov: std::f32::consts::FRAC_PI_2, // 90 degrees
        }
    }
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self::new(Vec3::ZERO)
    }
}