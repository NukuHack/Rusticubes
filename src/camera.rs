
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
        size: &winit::dpi::PhysicalSize<u32>,
        position: cgmath::Point3<f32>,
    ) -> Self {
        let yaw: cgmath::Rad<f32>= cgmath::Rad::from(cgmath::Deg(-90.0)); // left-right (- is left; + is right) by default the camera faces left (90°)
        let pitch: cgmath::Rad<f32>= cgmath::Rad(0.0); // up-down (- is down; + is up) by default it is 0
        let fovy: cgmath::Rad<f32> = cgmath::Rad::from(cgmath::Deg(90.0)); // by default you can see the 1/4 of the world
        let (znear, zfar):(f32,f32) = (0.01, 100.0); // idk what these are actually
        let (camera_speed,sensitivity):(f32,f32) = (4.0,0.5); // speed (will put this into the player class) and sensitivity for mouse movement

        let camera: Camera = Camera::new(position, yaw, pitch);
        let projection: Projection = Projection::new(*size, fovy, znear, zfar);
        let controller: CameraController = CameraController::new(camera_speed, sensitivity);
        let mut uniform = CameraUniform::default();
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
    pub fn update(&mut self, queue: &wgpu::Queue, delta_time: f32) {
        self.controller.update_camera(&mut self.camera, &mut self.projection, delta_time);
        self.uniform.update_view_proj(&self.camera, &self.projection);
        queue.write_buffer(
            &self.buffer,
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
    fn default() -> Self {
        Self { view_proj: cgmath::Matrix4::identity().into() }
    }
}

impl CameraUniform {
    pub fn update_view_proj(&mut self, camera: &Camera, projection: &Projection) {
        let view = camera.calc_matrix();
        let proj = projection.calc_matrix();
        self.view_proj = (proj * view).into();
    }
}


pub const SAFE_FRAC_PI_2: f32 = std::f32::consts::FRAC_PI_2 - 0.0001;
pub struct Camera {
    pub position: cgmath::Point3<f32>,
    pub yaw: cgmath::Rad<f32>,
    pub pitch: cgmath::Rad<f32>,
}
impl Camera {
    pub fn new(position: cgmath::Point3<f32>, yaw: cgmath::Rad<f32>, pitch: cgmath::Rad<f32>) -> Self{
        Self {
            position,
            yaw,
            pitch,
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
    /// Get the forward vector
    pub fn forward(&self) -> cgmath::Vector3<f32> {
        let (sin_p, cos_p) = self.pitch.0.sin_cos(); // Convert to radians
        let (sin_y, cos_y) = self.yaw.0.sin_cos(); // Convert to radians
        
        cgmath::Vector3::new(cos_p * cos_y, sin_p, cos_p * sin_y).normalize()
    }
    /// Get the right vector (perpendicular to forward)
    pub fn right(&self) -> cgmath::Vector3<f32> {
        self.forward().cross(cgmath::Vector3::unit_y()).normalize()
    }

    /// Get the up vector (perpendicular to both forward and right)
    pub fn up(&self) -> cgmath::Vector3<f32> {
        self.right().cross(self.forward())
    }
}
pub struct Projection {
    aspect: f32,
    fovy: cgmath::Rad<f32>,
    znear: f32,
    zfar: f32,
}
impl Projection {
    pub fn new(
        size: winit::dpi::PhysicalSize<u32>,
        fovy: cgmath::Rad<f32>,
        znear: f32,
        zfar: f32,
    ) -> Self {
        Self {
            aspect: size.height as f32 / size.width as f32,
            fovy,
            znear,
            zfar,
        }
    }
    pub fn resize(&mut self, width: u32, height: u32) {
        self.aspect = width as f32 / height as f32;
    }
    pub fn calc_matrix(&self) -> cgmath::Matrix4<f32> {
        Self::OPENGL_TO_WGPU_MATRIX * cgmath::perspective(self.fovy, self.aspect, self.znear, self.zfar)
    }
    
    pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
        1.0, 0.0, 0.0, 0.0,
        0.0, 1.0, 0.0, 0.0,
        0.0, 0.0, 0.5, 0.5,
        0.0, 0.0, 0.0, 1.0,
    );
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
    forward: bool,
    backward: bool,
    left: bool,
    right: bool,
    up: bool,
    down: bool,
    run: bool,
}
impl MovementInputs {
    fn default() -> Self {
        Self {
            forward: false,
            backward: false,
            left: false,
            right: false,
            up: false,
            down: false,
            run: false,
        }
    }
}
#[derive(Debug)]
struct RotationInputs {
    horizontal: f32,
    vertical: f32,
}
impl Default for RotationInputs {
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
        let is_pressed: bool = state == &ElementState::Pressed;
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
    pub fn reset_keyboard(&mut self) {
        self.movement.forward = false;
        self.movement.backward = false;
        self.movement.left = false;
        self.movement.right = false;
        self.movement.up = false;
        self.movement.run = false;
        self.movement.down = false;
    }
    pub fn process_mouse(&mut self, delta_x: f32, delta_y: f32) {
        self.rotation.horizontal = delta_x as f32;
        self.rotation.vertical = -delta_y as f32; // because this is stupidly made in the opposite way AHH
    }
    pub fn process_scroll(&mut self, delta: &MouseScrollDelta) {
        self.scroll = match delta {
            MouseScrollDelta::LineDelta(_, y) => y * 0.5,
            MouseScrollDelta::PixelDelta(pos) => pos.y as f32,
        };
    }
    pub fn update_camera(&mut self, camera: &mut Camera, projection: &mut Projection, delta_time: f32) {
        { //this is the movement of the camera
            let run_multiplier: f32 = if self.movement.run { self.run_multi } else { 1.0 };
            // Calculating the actual angle to move towards
            let (yaw_sin, yaw_cos): (f32, f32) = camera.yaw.0.sin_cos();
            let forward_dir = cgmath::Vector3::new(yaw_cos, 0.0, yaw_sin).normalize();
            let right_dir = cgmath::Vector3::new(-yaw_sin, 0.0, yaw_cos).normalize();
            // Move with dynamic run multiplier
            let forward_amount: f32 = ((self.movement.forward as i8 - self.movement.backward as i8) as f32) * run_multiplier;
            let right_amount: f32 = ((self.movement.right as i8 - self.movement.left as i8) as f32) * run_multiplier;
            let up_amount: f32 = ((self.movement.up as i8 - self.movement.down as i8) as f32) * run_multiplier;
            // Setting the position to the desired point
            camera.position += forward_dir * forward_amount * self.speed * delta_time;
            camera.position += right_dir * right_amount * self.speed * delta_time;
            camera.position.y += up_amount * self.speed * delta_time;
        } { //this is the scrolling of the cam
            // Field of view +/- (zoom)
            let delta: f32 = self.scroll * self.sensitivity;
            projection.fovy = cgmath::Rad(
                (projection.fovy.0 - delta).clamp(0.001, 3.14) // pi = 3.1415926535 so leaving 0.001 for floating point inaccuracies is fine
            );
            self.scroll = 0.0; // null the value after using it
        } { // this is the rotating of the cam
            // Calculate rotation delta scaled by sensitivity and time
            let delta: f32 = self.sensitivity * delta_time;
            camera.yaw += cgmath::Rad(self.rotation.horizontal) * delta;
            let pitch = (camera.pitch.0 + self.rotation.vertical * delta)
                .clamp(-SAFE_FRAC_PI_2, SAFE_FRAC_PI_2);
            camera.pitch = cgmath::Rad(pitch);
            self.rotation.horizontal = 0.0; // null the value after using it
            self.rotation.vertical = 0.0;   // null the value after using it
        }
    }
}