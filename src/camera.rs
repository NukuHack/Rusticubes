
use winit::event::*;
use cgmath::{InnerSpace, SquareMatrix};
use wgpu::util::DeviceExt;
use winit::keyboard::{KeyCode as Key};


pub struct CameraSystem {
    pub camera: Camera,
    pub projection: Projection,
    pub controller: CameraController,
    pub uniform: CameraUniform,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}
impl CameraSystem {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        position: cgmath::Point3<f32>,
    ) -> Self {
        let yaw: cgmath::Rad<f32>= cgmath::Rad(-std::f32::consts::FRAC_PI_2);
        let pitch: cgmath::Rad<f32>= cgmath::Rad(-0.3);
        let fovy: f32 = 70.0;
        let znear: f32 = 0.01;
        let zfar: f32 = 100.0;
        let camera_speed: f32 = 4.0;
        let sensitivity: f32 = 0.5;

        let camera: Camera = Camera::new(position, yaw, pitch);
        let projection: Projection = Projection::new(config.width, config.height, cgmath::Rad::from(cgmath::Deg(fovy)), znear, zfar);
        let controller: CameraController = CameraController::new(camera_speed, sensitivity);
        let mut uniform: CameraUniform = CameraUniform::new();
        uniform.update_view_proj(&camera, &projection);
        let buffer: wgpu::Buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Camera Buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_group_layout: wgpu::BindGroupLayout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
        let bind_group: wgpu::BindGroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
            controller,
            uniform,
            buffer,
            bind_group,
            bind_group_layout,
        }
    }
    pub fn update(&mut self, queue: &wgpu::Queue, delta_time: std::time::Duration) {
        self.controller.update_camera(&mut self.camera, &mut self.projection, delta_time);
        self.uniform.update_view_proj(&self.camera, &self.projection);
        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(std::slice::from_ref(&self.uniform)),
        );
    }
}

// Keep these structs as provided in your code
#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);
pub const SAFE_FRAC_PI_2: f32 = std::f32::consts::FRAC_PI_2 - 0.0001;
pub struct Camera {
    pub position: cgmath::Point3<f32>,
    yaw: cgmath::Rad<f32>,
    pitch: cgmath::Rad<f32>,
}
impl Camera {
    pub fn new<V, Y, P>(position: V, yaw: Y, pitch: P) -> Self
    where
        V: Into<cgmath::Point3<f32>>,
        Y: Into<cgmath::Rad<f32>>,
        P: Into<cgmath::Rad<f32>>,
    {
        Self {
            position: position.into(),
            yaw: yaw.into(),
            pitch: pitch.into(),
        }
    }
    pub fn calc_matrix(&self) -> cgmath::Matrix4<f32> {
        let (sin_pitch, cos_pitch) = self.pitch.0.sin_cos();
        let (sin_yaw, cos_yaw) = self.yaw.0.sin_cos();
        cgmath::Matrix4::look_to_rh(
            self.position,
            cgmath::Vector3::new(
                cos_pitch * cos_yaw,
                sin_pitch,
                cos_pitch * sin_yaw
            ).normalize(),
            cgmath::Vector3::unit_y(),
        )
    }
}
pub struct Projection {
    aspect: f32,
    fovy: cgmath::Rad<f32>,
    znear: f32,
    zfar: f32,
}
impl Projection {
    pub fn new<F: Into<cgmath::Rad<f32>> + std::fmt::Debug>(
        width: u32,
        height: u32,
        fovy: F,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Self {
            aspect: width as f32 / height as f32,
            fovy: fovy.into(),
            znear,
            zfar,
        }
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }
    pub fn calc_matrix(&self) -> cgmath::Matrix4<f32> {
        OPENGL_TO_WGPU_MATRIX * cgmath::perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
}
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
    forward: f32,
    backward: f32,
    left: f32,
    right: f32,
    up: f32,
    down: f32,
    run: bool,
}
impl MovementInputs {
    fn default() -> Self {
        Self {
            forward: 0.0,
            backward: 0.0,
            left: 0.0,
            right: 0.0,
            up: 0.0,
            down: 0.0,
            run: false,
        }
    }
}
#[derive(Debug)]
struct RotationInputs {
    horizontal: f32,
    vertical: f32,
}
impl RotationInputs {
    fn default() -> Self {
        Self {
            horizontal: 0.0,
            vertical: 0.0,
        }
    }
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
        let is_pressed: bool = state == &ElementState::Pressed as &ElementState;
        let amount: f32 = if is_pressed { 1.0 } else { 0.0 };
        match key {
            Key::KeyW | Key::ArrowUp => {
                self.movement.forward = amount;
                true
            }
            Key::KeyS | Key::ArrowDown => {
                self.movement.backward = amount;
                true
            }
            Key::KeyA | Key::ArrowLeft => {
                self.movement.left = amount;
                true
            }
            Key::KeyD | Key::ArrowRight => {
                self.movement.right = amount;
                true
            }
            Key::Space => {
                self.movement.up = amount;
                true
            }
            Key::ShiftLeft => {
                self.movement.run = is_pressed;
                true
            }
            Key::ControlLeft => {
                self.movement.down = amount;
                true
            }
            _ => false,
        }
    }
    pub fn process_mouse(&mut self, mouse_dx: f64, mouse_dy: f64) {
        self.rotation.horizontal = mouse_dx as f32;
        self.rotation.vertical = -mouse_dy as f32; // because this is stupidly made in the opposite way AHH
    }
    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = match delta {
            MouseScrollDelta::LineDelta(_, scroll) => scroll * 0.5,
            MouseScrollDelta::PixelDelta(winit::dpi::PhysicalPosition { y: scroll, .. }) => *scroll as f32,
        };
    }
    pub fn update_camera(&mut self, camera: &mut Camera, projection: &mut Projection, dt: std::time::Duration) {
        let dt: f32 = dt.as_secs_f32();
        { //this is the movement of the camera
            let run_multiplier: f32 = if self.movement.run { self.run_multi } else { 1.0 };
            // Move with dynamic run multiplier
            let (yaw_sin, yaw_cos): (f32, f32) = camera.yaw.0.sin_cos();
            let forward_dir = cgmath::Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
            let right_dir = cgmath::Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
            let forward_amount: f32 = (self.movement.forward - self.movement.backward) * run_multiplier;
            let right_amount: f32 = (self.movement.right - self.movement.left) * run_multiplier;
            let up_amount: f32 = (self.movement.up - self.movement.down) * run_multiplier;
            camera.position += forward_dir * forward_amount * self.speed * dt;
            camera.position += right_dir * right_amount * self.speed * dt;
            camera.position.y += up_amount * self.speed * dt;
        } { //this is the scrolling of the cam
            // Field of view +/- (zoom)
            let delta: f32 = self.scroll * self.sensitivity;
            projection.fovy = cgmath::Rad(
                (projection.fovy.0 - delta).clamp(0.01, 3.14)
            );
            self.scroll = 0.0; // null the value after using it
        } { // this is the rotating of the cam
            // Calculate rotation delta scaled by sensitivity and time
            let delta: f32 = self.sensitivity * dt;
            // Extract the f32 value, apply clamp, and re-wrap into Rad<f32> s
            let max_pitch = cgmath::Rad(self::SAFE_FRAC_PI_2);
            camera.yaw += cgmath::Rad(self.rotation.horizontal) * delta;
            camera.pitch = {
                let raw_value: f32 = camera.pitch.0 + (self.rotation.vertical * delta);
                cgmath::Rad(raw_value.clamp(-max_pitch.0, max_pitch.0))
            };
            self.rotation.horizontal = 0.0; // null the value after using it
            self.rotation.vertical = 0.0;   // null the value after using it
        }
    }
}
#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}
impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }
    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        let view = camera.calc_matrix();
        let proj = projection.calc_matrix();
        self.view_proj = (proj * view).into();
    }
}