
use winit::{
    event::*,
    keyboard::{KeyCode, PhysicalKey},
};
use wgpu::{util::DeviceExt, SurfaceConfiguration};

pub struct CameraSystem {
    #[allow(unused)]
    pub camera: Camera,
    pub uniform: CameraUniform,
    pub buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl CameraSystem {
    pub fn new(
        device: &wgpu::Device,
        config: &SurfaceConfiguration,
        camera: Camera,
    ) -> Self {
        let mut uniform = CameraUniform::new();
        uniform.update_view_proj(&camera);

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
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
                resource: buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        Self {
            camera,
            uniform,
            buffer,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.uniform.update_view_proj(&self.camera);
        queue.write_buffer(
            &self.buffer,
            0,
            bytemuck::cast_slice(&[self.uniform]),
        );
    }
}

pub struct Camera {
    #[allow(unused)]
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn new(
        eye: cgmath::Point3<f32>,
        target: cgmath::Point3<f32>,
        up: cgmath::Vector3<f32>,
        aspect: f32,
        fovy: f32,
        znear: f32,
        zfar: f32,
    ) -> Self {

        Self {
            eye,
            target,
            up,
            aspect,
            fovy,
            znear,
            zfar,
        }
    }
    fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        // 1.
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        // 2.
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        // 3.
        OPENGL_TO_WGPU_MATRIX * proj * view
    }
}

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);

// We need this for Rust to store our data correctly for the shaders
#[repr(C)]
// This is so we can store this in a buffer
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    #[allow(unused)]
    // We can't use cgmath with bytemuck directly, so we'll have
    // to convert the Matrix4 into a 4x4 f32 array
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}



pub struct CameraController {
    #[allow(unused)]
    speed: f32,
    rotation_speed: f32,
    is_forward_pressed: bool,
    is_backward_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    is_rotate_left: bool,
    is_rotate_right: bool,
}

impl CameraController {
    pub fn new(speed: f32, rotation_speed: f32) -> Self {
        Self {
            speed,
            rotation_speed,
            is_forward_pressed: false,
            is_backward_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_rotate_left: false,
            is_rotate_right: false,
        }
    }

    pub fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event: KeyEvent { state, physical_key: PhysicalKey::Code(keycode), .. },
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    KeyCode::KeyW | KeyCode::ArrowUp => {
                        self.is_forward_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyA | KeyCode::ArrowLeft => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyS | KeyCode::ArrowDown => {
                        self.is_backward_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyD | KeyCode::ArrowRight => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyQ => {
                        self.is_rotate_left = is_pressed;
                        true
                    }
                    KeyCode::KeyE => {
                        self.is_rotate_right = is_pressed;
                        true
                    }
                    _ => false
                }
            }
            _ => false
        }
    }


    pub fn update_camera(&self, camera: &mut Camera, delta_time: f32) {
        use cgmath::InnerSpace;
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();

        // Forward/Backward movement with delta_time
        if self.is_forward_pressed && forward_mag > self.speed * delta_time {
            camera.eye += forward_norm * self.speed * delta_time;
        }
        if self.is_backward_pressed {
            camera.eye -= forward_norm * self.speed * delta_time;
        }

        // Right/Left movement with delta_time
        let right = forward_norm.cross(camera.up);
        if self.is_right_pressed {
            let movement = right * self.speed * delta_time;
            camera.eye = camera.target - (forward + movement).normalize() * forward_mag;
        }
        if self.is_left_pressed {
            let movement = right * self.speed * delta_time;
            camera.eye = camera.target - (forward - movement).normalize() * forward_mag;
        }

        // Rotation with delta_time
        let mut total_rotation = 0.0;
        if self.is_rotate_left {
            total_rotation -= self.rotation_speed * delta_time;
        }
        if self.is_rotate_right {
            total_rotation += self.rotation_speed * delta_time;
        }

        if total_rotation != 0.0 {
            let rotation = cgmath::Matrix3::from_angle_y(cgmath::Rad(total_rotation));
            let new_forward = rotation * forward_norm;
            camera.target = camera.eye + new_forward * forward_mag;
        }
    }
}