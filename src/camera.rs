use glam::{Vec3, Mat4, Quat};
use wgpu::util::DeviceExt;
use winit::dpi::PhysicalSize;

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
        self.view_proj = (projection.matrix * camera.view_matrix()).to_cols_array_2d();
        self.position = camera.position.extend(0.0).into();
    }
}

pub struct CameraSystem {
    pub camera: Camera,
    pub projection: Projection,
    pub uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
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
        
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
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
                resource: camera_buffer.as_entire_binding(),
            }],
            label: Some("camera_bind_group"),
        });

        Self {
            camera,
            projection,
            uniform,
            camera_buffer,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        self.uniform.update_view_proj(&self.camera, &self.projection);
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.uniform]));
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.projection.resize(new_size);
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

#[derive(Debug)]
pub struct Camera {
    pub position: Vec3,
    pub rotation: Vec3,
}

impl Camera {
    pub fn new(position: Vec3, rotation: Vec3) -> Self {
        Self { position, rotation }
    }

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_rh(self.position, self.forward(), Vec3::Y)
    }

    pub fn forward(&self) -> Vec3 {
        Quat::from_rotation_y(self.rotation.x) * Quat::from_rotation_x(self.rotation.y) * Vec3::NEG_Z
    }

    pub fn right(&self) -> Vec3 {
        Quat::from_rotation_y(self.rotation.x) * Vec3::X
    }

    pub fn up(&self) -> Vec3 {
        Quat::from_rotation_y(self.rotation.x) * Quat::from_rotation_x(self.rotation.y) * Vec3::Y
    }
}

pub struct Projection {
    pub matrix: Mat4,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Projection {
    pub fn new(size: PhysicalSize<u32>, fovy: f32, znear: f32, zfar: f32) -> Self {
        let aspect = size.width as f32 / size.height as f32;
        Self {
            matrix: Mat4::perspective_rh(fovy, aspect, znear, zfar),
            aspect,
            fovy,
            znear,
            zfar,
        }
    }

    pub fn resize(&mut self, size: PhysicalSize<u32>) {
        self.aspect = size.width as f32 / size.height as f32;
        self.matrix = Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar);
    }

    pub fn set_fovy(&mut self, fovy: f32) {
        self.fovy = fovy;
        self.matrix = Mat4::perspective_rh(self.fovy, self.aspect, self.znear, self.zfar);
    }
}

pub const SAFE_FRAC_PI_2: f32 = std::f32::consts::FRAC_PI_2 - 0.0001;

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
}

impl CameraConfig {
    pub fn new(position: Vec3) -> Self {
        Self {
            position,
            rotation: Vec3::new(-90.0f32.to_radians(),0.0,0.0),
            fovy: 90.0f32.to_radians(),
            znear: 0.01,
            zfar: 100.0,
            speed: 4.0,
            sensitivity: 0.5,
            run_multiplier: 2.5,
            smoothness: 5.0,
        }
    }
}

impl Default for CameraConfig {
    fn default() -> Self {
        Self::new(Vec3::ZERO)
    }
}


