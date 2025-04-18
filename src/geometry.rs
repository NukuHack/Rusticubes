use cgmath::SquareMatrix;
use cgmath::{InnerSpace, Rotation3};
use image::GenericImageView;
use std::mem;
use wgpu::util::DeviceExt;

// --- Vertex & Buffer Layouts ---
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
                },
            ],
        }
    }
}

// --- Geometry Buffer ---
#[derive(Debug, Clone)]
pub struct GeometryBuffer {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
}

impl GeometryBuffer {
    pub fn new(device: &wgpu::Device, indices: &[u32], vertices: &[Vertex]) -> Self {
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        Self {
            vertex_buffer,
            index_buffer,
            num_indices: indices.len() as u32,
        }
    }
}

// --- Instance Manager ---
pub struct InstanceManager {
    pub instances: Vec<Instance>,
    pub instance_buffer: wgpu::Buffer,
    pub capacity: usize,
}
impl InstanceManager {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let instances = vec![super::cube::Block::default().to_instance()];

        // Rest remains the same - buffer creation and initialization
        let capacity = instances.len() * 2;
        let buffer_size = (capacity * mem::size_of::<InstanceRaw>()) as u64;
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer_size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        queue.write_buffer(
            &instance_buffer,
            0,
            bytemuck::cast_slice(&instances.iter().map(|i| i.to_raw()).collect::<Vec<_>>()),
        );

        Self {
            instances,
            instance_buffer,
            capacity,
        }
    }
}
impl InstanceManager {
    pub fn add_instance(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, instance: Instance) {
        if self.instances.len() >= self.capacity {
            self.capacity *= 2;
            let new_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Instance Buffer"),
                size: (self.capacity * mem::size_of::<InstanceRaw>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            queue.write_buffer(
                &new_buffer,
                0,
                bytemuck::cast_slice(
                    &self
                        .instances
                        .iter()
                        .map(|i| i.to_raw())
                        .collect::<Vec<_>>(),
                ),
            );

            self.instance_buffer = new_buffer;
        }

        self.instances.push(instance.clone());
        let offset = self.instances.len() - 1;
        queue.write_buffer(
            &self.instance_buffer,
            (offset * mem::size_of::<InstanceRaw>()) as u64,
            bytemuck::cast_slice(&[instance.to_raw()]),
        );
    }

    pub fn remove_instance(&mut self, index: usize, queue: &wgpu::Queue) {
        if index >= self.instances.len() {
            return; // or handle error as needed
        }

        // Remove the instance from the vector
        self.instances.remove(index);

        // Rebuild the instance buffer with the updated data
        let instance_data: Vec<InstanceRaw> = self.instances.iter().map(|i| i.to_raw()).collect();
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&instance_data),
        );
    }
}
impl InstanceManager {
    // Add multiple instances at once
    pub fn add_instances(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        instances: &[Instance],
    ) {
        let required_capacity = self.instances.len() + instances.len();
        if required_capacity > self.capacity {
            self.capacity = required_capacity * 2;
            let new_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Instance Buffer"),
                size: (self.capacity * mem::size_of::<InstanceRaw>()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            // Upload all existing + new instances
            let all_instances = self
                .instances
                .iter()
                .chain(instances.iter())
                .map(|i| i.to_raw())
                .collect::<Vec<_>>();

            queue.write_buffer(&new_buffer, 0, bytemuck::cast_slice(&all_instances));
            self.instance_buffer = new_buffer;
        } else {
            // Only upload new instances
            let offset = self.instances.len();
            let new_raw: Vec<_> = instances.iter().map(|i| i.to_raw()).collect();
            queue.write_buffer(
                &self.instance_buffer,
                (offset * mem::size_of::<InstanceRaw>()) as u64,
                bytemuck::cast_slice(&new_raw),
            );
        }

        self.instances.extend_from_slice(instances);
    }

    // Remove a range of instances
    pub fn remove_instances(&mut self, start: usize, end: usize, queue: &wgpu::Queue) {
        if start >= end || end > self.instances.len() {
            return;
        }
        self.instances.drain(start..end);
        let instance_data: Vec<_> = self.instances.iter().map(|i| i.to_raw()).collect();
        queue.write_buffer(
            &self.instance_buffer,
            0,
            bytemuck::cast_slice(&instance_data),
        );
    }
}
impl InstanceManager {
    /// Add cubes from a chunk to the instance manager
    pub fn add_chunk(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        chunk: &super::cube::Chunk,
    ) {
        let new_instances: Vec<Instance> = chunk
            .blocks
            .iter()
            .filter(|cube| !cube.is_empty())
            .map(|cube| cube.to_world_instance(chunk.position))
            .collect();

        // Batch upload all instances at once
        self.add_instances(device, queue, &new_instances);
    }

    /// Remove cubes from a chunk from the instance manager
    pub fn remove_chunk(&mut self, queue: &wgpu::Queue, chunk: &super::cube::Chunk) {
        for cube in chunk.blocks.iter() {
            if !cube.is_empty() {
                let index = self
                    .instances
                    .iter()
                    .position(|i| {
                        let cube_pos = cube.get_position_f();
                        let instance_pos = i.position;
                        (cube_pos - instance_pos).magnitude() < 0.01 // Threshold for matching positions
                    })
                    .unwrap_or_else(|| panic!("Failed to find instance"));
                self.remove_instance(index, queue);
            }
        }
    }
}

pub static NULL_QUATERNION: cgmath::Quaternion<f32> = cgmath::Quaternion::new(1.0, 0.0, 0.0, 0.0);

pub fn add_def_cube() {
    unsafe {
        let state = super::get_state();
        let mut instance_manager = state.instance_manager().borrow_mut();

        let rotation = if true {
            NULL_QUATERNION
        } else {
            // Calculate adjusted yaw (including the FRAC_PI_2 offset)
            let q_yaw: cgmath::Quaternion<f32> = cgmath::Quaternion::from_angle_y(
                -state.camera_system.camera.yaw + cgmath::Rad(std::f32::consts::FRAC_PI_2),
            );
            let q_pitch: cgmath::Quaternion<f32> =
                cgmath::Quaternion::from_angle_x(state.camera_system.camera.pitch);

            // Combine rotations: first yaw (around Y-axis), then pitch (around X-axis)
            q_pitch * q_yaw
        };

        // Get camera position in world coordinates
        let camera_pos = state.camera_system.camera.position;

        // Calculate where to place the cube (in front of the camera)
        let forward = state.camera_system.camera.forward();
        let placement_distance = 6.0; // Distance in front of camera
        let placement_position =
            super::cube::vec3_f32_to_i32(camera_pos + forward * placement_distance);

        // Convert to chunk coordinates
        let chunk_pos = super::cube::ChunkCoord::from_world_position(placement_position);

        // Convert to local position within chunk
        //let local_pos = super::cube::Chunk::world_to_local_pos(placement_position);

        // Create cube at the correct position
        let cube = super::cube::Block::new_rot_raw(placement_position, rotation);

        // Add to instance manager with proper world position
        instance_manager.add_instance(
            state.device(),
            state.queue(),
            cube.to_world_instance(chunk_pos),
        );
    }
}
pub fn rem_last_cube() {
    unsafe {
        let state = super::get_state();
        let mut instance_manager = state.instance_manager().borrow_mut();
        if !instance_manager.instances.is_empty() {
            let index = instance_manager.instances.len() - 1;
            instance_manager.remove_instance(index, state.queue());
        }
    }
}
pub fn add_def_chunk() {
    unsafe {
        let state = super::get_state();
        let mut instance_manager = state.instance_manager().borrow_mut();
        let camera_position = state.camera_system.camera.position;
        let chunk_position = cgmath::Vector3::new(
            camera_position.x + 8.5,
            camera_position.y + 8.0,
            camera_position.z + 8.5,
        );
        let real_chunk_position = super::cube::vec3_f32_to_i32(chunk_position);

        if let Some(chunk) = super::cube::Chunk::load(super::cube::ChunkCoord::from_world_position(
            real_chunk_position,
        )) {
            instance_manager.add_chunk(state.device(), state.queue(), &chunk);
        } else {
            eprintln!("Failed to load chunk at {:?}", chunk_position);
        }
    }
}
pub fn rem_pos_cube(position: cgmath::Vector3<f32>, threshold: f32) {
    unsafe {
        let state = super::get_state();
        let mut instance_manager = state.instance_manager().borrow_mut();
        let index = instance_manager
            .instances
            .iter()
            .position(|i| (i.position - position).magnitude() < threshold);

        if let Some(idx) = index {
            instance_manager.remove_instance(idx, state.queue());
        }
    }
}

pub fn cast_ray_and_select_cube(
    camera: &super::camera::Camera,
    projection: &super::camera::Projection,
    instances: &[Instance],
    max_distance: f32,
) -> Option<usize> {
    // Create ray directly in center of view (NDC 0,0,-1)
    let ray_clip = cgmath::Vector4::new(0.0, 0.0, -1.0, 1.0);

    // Convert to eye (camera) space
    let inv_proj = projection.calc_matrix().invert().unwrap();
    let mut ray_eye: cgmath::Vector4<f32> = inv_proj * ray_clip;
    ray_eye = cgmath::Vector4::new(ray_eye.x, ray_eye.y, -1.0, 0.0);

    // Convert to world space
    let inv_view = camera.calc_matrix().invert().unwrap();
    let ray_world = inv_view * ray_eye;
    let ray_dir = ray_world.truncate().normalize();

    let ray_origin = camera.position;

    // Early exit if no instances
    if instances.is_empty() {
        return None;
    }

    // Optimized ray-AABB intersection with distance check
    let mut closest_index = None;
    let mut closest_distance = max_distance;

    for (index, instance) in instances.iter().enumerate() {
        let cube_center = instance.position;
        let half_extents = cgmath::Vector3::new(0.5, 0.5, 0.5);

        // Early distance check - skip if too far
        let center_dist = (cube_center - ray_origin).magnitude();
        if center_dist > max_distance + 0.87 {
            // 0.87 is approx sqrt(3)/2
            continue;
        }

        // Optimized AABB intersection
        let aabb_min = cube_center - half_extents;
        let aabb_max = cube_center + half_extents;

        if let Some(t) = ray_aabb_intersect(ray_origin, ray_dir, aabb_min, aabb_max) {
            if t > 0.0 && t < closest_distance {
                closest_distance = t;
                closest_index = Some(index);
            }
        }
    }

    closest_index
}

#[inline]
fn ray_aabb_intersect(
    ray_origin: cgmath::Vector3<f32>,
    ray_dir: cgmath::Vector3<f32>,
    aabb_min: cgmath::Vector3<f32>,
    aabb_max: cgmath::Vector3<f32>,
) -> Option<f32> {
    let mut tmin = -f32::INFINITY;
    let mut tmax = f32::INFINITY;

    // Unroll the loop for better performance
    for i in 0..3 {
        let inv_d = 1.0 / ray_dir[i];
        let mut t1 = (aabb_min[i] - ray_origin[i]) * inv_d;
        let mut t2 = (aabb_max[i] - ray_origin[i]) * inv_d;

        if inv_d < 0.0 {
            std::mem::swap(&mut t1, &mut t2);
        }

        tmin = tmin.max(t1);
        tmax = tmax.min(t2);

        if tmax < tmin {
            return None;
        }
    }

    // Return the closest positive intersection
    if tmin > 0.0 {
        Some(tmin)
    } else if tmax > 0.0 {
        Some(tmax)
    } else {
        None
    }
}

// Function to remove the raycasted cube
pub fn rem_raycasted_cube() {
    unsafe {
        let state = super::get_state();
        let instance_manager = &mut *state.instance_manager().borrow_mut();

        if let Some(index) = cast_ray_and_select_cube(
            &state.camera_system.camera,
            &state.camera_system.projection,
            &instance_manager.instances,
            10.0,
        ) {
            instance_manager.remove_instance(index, state.queue());
        }
    }
}

// --- Instance Struct ---
#[repr(C)]
#[derive(Clone)]
pub struct Instance {
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
}

impl Instance {
    #[inline] // ← Critical for performance
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position)
                * cgmath::Matrix4::from(self.rotation))
            .into(),
        }
    }
    pub fn to_cube(&self) -> super::cube::Block {
        super::cube::Block::new_rot_raw(super::cube::vec3_f32_to_i32(self.position), self.rotation)
    }
}
impl Default for Instance {
    fn default() -> Self {
        //super::cube::default().to_instance()
        Instance {
            position: cgmath::Vector3::new(0.0, 0.0, 0.0),
            rotation: cgmath::Quaternion::from_angle_y(cgmath::Deg(0.0)),
        }
    }
}

// --- InstanceRaw ---
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub model: [[f32; 4]; 4],
}

impl InstanceRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct Texture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub sampler: wgpu::Sampler,
}

impl Texture {
    pub fn from_bytes(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        bytes: &[u8],
        label: &str,
    ) -> std::result::Result<Self, image::ImageError> {
        let img: image::DynamicImage = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, Some(label))
    }

    pub fn from_image(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> std::result::Result<Self, image::ImageError> {
        let rgba: image::RgbaImage = img.to_rgba8();
        let dimensions: (u32, u32) = img.dimensions();

        let size: wgpu::Extent3d = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };

        let texture: wgpu::Texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                aspect: wgpu::TextureAspect::All,
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            &rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * dimensions.0),
                rows_per_image: Some(dimensions.1),
            },
            size,
        );

        let view: wgpu::TextureView = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler: wgpu::Sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            sampler,
        })
    }

    pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    pub fn create_depth_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> Self {
        let size: wgpu::Extent3d = wgpu::Extent3d {
            width: config.width.max(1),
            height: config.height.max(1),
            depth_or_array_layers: 1,
        };

        let texture: wgpu::Texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: Self::DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view: wgpu::TextureView = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            view,
            sampler: device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                compare: Some(wgpu::CompareFunction::LessEqual),
                ..Default::default()
            }),
        }
    }
}

// --- Texture Manager ---
pub struct TextureManager {
    pub texture: Texture,
    pub depth_texture: Texture,
    pub bind_group: wgpu::BindGroup,
    pub bind_group_layout: wgpu::BindGroupLayout,
}

impl TextureManager {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        config: &wgpu::SurfaceConfiguration,
    ) -> Self {
        let texture_path = "resources/cube-diffuse.jpg";
        let bytes = std::fs::read(texture_path).expect("Texture not found");
        let texture = Texture::from_bytes(device, queue, &bytes, texture_path).unwrap();

        let depth_texture = Texture::create_depth_texture(device, config, "Depth Texture");

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
            label: Some("Texture Bind Group Layout"),
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
            label: Some("Texture Bind Group"),
        });
        Self {
            texture,
            depth_texture,
            bind_group,
            bind_group_layout,
        }
    }
}
