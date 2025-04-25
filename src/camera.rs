
use winit::event::*;
use wgpu::util::DeviceExt;
use glam::{Vec3, Mat4, Quat};
use winit::keyboard::{KeyCode as Key};

pub struct CameraSystem {
    pub camera: Camera,
    pub projection: Projection,
    pub controller: CameraController,
    pub uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl CameraSystem {
    pub fn new(
        device: &wgpu::Device,
        size: &winit::dpi::PhysicalSize<u32>,
        position: Vec3,
    ) -> Self {
        let yaw = -90.0f32.to_radians(); // left-right (- is left; + is right)
        let pitch = 0.0f32.to_radians(); // up-down (- is down; + is up)
        let fovy = 90.0f32.to_radians(); // field of view
        let (znear, zfar) = (0.01, 100.0);
        let (camera_speed, sensitivity) = (4.0, 0.5);

        let camera = Camera::new(position, yaw, pitch);
        let projection = Projection::new(*size, fovy, znear, zfar);
        let controller = CameraController::new(camera_speed, sensitivity);
        let mut uniform = CameraUniform::default();
        uniform.update_view_proj(&camera, &projection);
        
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
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
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        Self {
            camera,
            projection,
            controller,
            uniform,
            camera_buffer,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue, delta_time: f32) {
        self.controller.update_camera(&mut self.camera, &mut self.projection, delta_time);
        self.uniform.update_view_proj(&self.camera, &self.projection);
        queue.write_buffer(
            &self.camera_buffer,
            0,
            bytemuck::cast_slice(std::slice::from_ref(&self.uniform)),
        );
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl Default for CameraUniform {
    #[inline]
    fn default() -> Self {
        Self { view_proj: Mat4::IDENTITY.to_cols_array_2d() }
    }
}

impl CameraUniform {
    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        let view = camera.calc_matrix();
        let proj = projection.calc_matrix();
        self.view_proj = (proj * view).to_cols_array_2d();
    }
}

pub const SAFE_FRAC_PI_2: f32 = std::f32::consts::FRAC_PI_2 - 0.0001;

pub struct Camera {
    pub position: Vec3,
    pub yaw: f32,    // Stored in radians
    pub pitch: f32,   // Stored in radians
}

impl Camera {
    #[inline]
    pub fn new(position: Vec3, yaw: f32, pitch: f32) -> Self {
        Self { position, yaw, pitch }
    }

    pub fn calc_matrix(&self) -> Mat4 {
        // Create rotation quaternion from yaw (around Y) and pitch (around X)
        let yaw_rot = Quat::from_rotation_y(self.yaw);
        let pitch_rot = Quat::from_rotation_x(self.pitch);
        let rotation = yaw_rot * pitch_rot;
        
        Mat4::look_to_rh(
            self.position,
            rotation * Vec3::NEG_Z,  // Forward direction
            Vec3::Y                  // Up direction
        )
    }

    #[inline]
    pub fn forward(&self) -> Vec3 {
        let yaw_rot = Quat::from_rotation_y(self.yaw);
        let pitch_rot = Quat::from_rotation_x(self.pitch);
        yaw_rot * pitch_rot * Vec3::NEG_Z
    }

    #[inline]
    pub fn right(&self) -> Vec3 {
        let yaw_rot = Quat::from_rotation_y(self.yaw);
        yaw_rot * Vec3::X
    }

    #[inline]
    pub fn up(&self) -> Vec3 {
        let yaw_rot = Quat::from_rotation_y(self.yaw);
        let pitch_rot = Quat::from_rotation_x(self.pitch);
        (yaw_rot * pitch_rot) * Vec3::Y
    }
}

pub struct Projection {
    aspect: f32,
    fovy: f32,  // Stored in radians
    znear: f32,
    zfar: f32,
}

impl Projection {
    pub fn new(size: winit::dpi::PhysicalSize<u32>, fovy: f32, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: size.width as f32 / size.height as f32,
            fovy,
            znear,
            zfar,
        }
    }

    #[inline]
    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }

    #[inline]
    pub fn calc_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar)
    }
}

// Rest of the code (CameraController and related structs) remains largely the same,
// just replace cgmath::Vector3 with glam::Vec3 where needed

#[derive(Debug)]
pub struct CameraController {
    movement: MovementInputs,
    rotation: RotationInputs,
    run_multi: f32,
    scroll: f32,
    speed: f32,
    sensitivity: f32,
}

#[derive(Debug, Default)]
struct MovementInputs {
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    run: bool,
}

#[derive(Debug, Default)]
struct RotationInputs {
    horizontal: f32,
    vertical: f32,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            movement: MovementInputs::default(),
            rotation: RotationInputs::default(),
            scroll: 0.0,
            run_multi: 2.5,
            speed,
            sensitivity,
        }
    }

    pub fn process_keyboard(&mut self, key: &Key, state: &ElementState) -> bool {
        let is_pressed = *state == ElementState::Pressed;
        match key {
            Key::KeyW | Key::ArrowUp => {
                self.movement.forward = is_pressed;
                true
            }
            Key::KeyS | Key::ArrowDown => {
                self.movement.backward = is_pressed;
                true
            }
            Key::KeyA | Key::ArrowLeft => {
                self.movement.left = is_pressed;
                true
            }
            Key::KeyD | Key::ArrowRight => {
                self.movement.right = is_pressed;
                true
            }
            Key::Space => {
                self.movement.up = is_pressed;
                true
            }
            Key::ShiftLeft => {
                self.movement.run = is_pressed;
                true
            }
            Key::ControlLeft => {
                self.movement.down = is_pressed;
                true
            }
            _ => false,
        }
    }

    #[inline]
    pub fn reset_keyboard(&mut self) {
        self.movement = MovementInputs::default();
    }

    #[inline]
    pub fn process_mouse(&mut self, delta_x: f32, delta_y: f32) {
        self.rotation.horizontal = -delta_x;
        self.rotation.vertical = -delta_y;
    }

    #[inline]
    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = match delta {
            MouseScrollDelta::LineDelta(_, y) => y * 0.5,
            MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
        };
    }

    #[inline]
    pub fn update_camera(&mut self, camera: &mut Camera, projection: &mut Projection, delta_time: f32) {
        // Movement
        let run_multiplier = if self.movement.run { self.run_multi } else { 1.0 };
        
        // Get the camera's actual forward and right vectors (already normalized)
        let mut forward_dir = camera.forward();
        forward_dir.y = 0.0;
        let mut right_dir = camera.right();
        right_dir.y = 0.0;
        
        let forward_amount = (self.movement.forward as i8 - self.movement.backward as i8) as f32 * run_multiplier;
        let right_amount = (self.movement.right as i8 - self.movement.left as i8) as f32 * run_multiplier;
        let up_amount = (self.movement.up as i8 - self.movement.down as i8) as f32 * run_multiplier;

        // Movement should be relative to camera orientation
        camera.position += forward_dir * forward_amount * self.speed * delta_time;
        camera.position += right_dir * right_amount * self.speed * delta_time;
        camera.position.y += up_amount * self.speed * delta_time;

        // Zoom
        let delta = self.scroll * self.sensitivity;
        projection.fovy = (projection.fovy - delta).clamp(0.001, std::f32::consts::PI);
        self.scroll = 0.0;

        // Rotation
        let delta = self.sensitivity * 0.05;
        camera.yaw += self.rotation.horizontal * delta;
        camera.pitch = (camera.pitch + self.rotation.vertical * delta)
            .clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);
        
        self.rotation.horizontal = 0.0;
        self.rotation.vertical = 0.0;
    }
}