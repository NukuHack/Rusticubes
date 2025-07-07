use cgmath::{Deg, InnerSpace, Matrix4, Quaternion, Rotation3, SquareMatrix, Vector3, Vector4};
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


/// Stores position for X, Y, Z as 4-bit fields: [X:4, Y:4, Z:4, Empty:4]
/// Stores rotations for X, Y, Z as 5-bit fields: [X:5, Y:5, Z:5, Empty:1]
/// Stores 3x3x3 points as a 32-bit "array" [Points: 27, Empty: 5]
#[derive(Clone, Copy)]
pub struct Block {
    /// in case someone needs it (i do i'm stupid) 4 bits is 0-15 ; 5 bits is 0-32; this goes forever (i think u256 is the current max)
    pub position: u16,    // [X:4, Y:4, Z:4, Empty:4]
    pub material: u16,    // Material info (unused in current implementation)
    pub points: u32,      // 3x3x3 points (27 bits used)
    pub rotation: u16,    // [X:5, Y:5, Z:5, Empty:1]
}

impl std::fmt::Debug for Block {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Block")
            .field("position", &format_args!("{:?}", self.position))
            .field("material", &format_args!("{:?}", self.material))
            .field("points", &format_args!("{:?}", self.points))
            .field("rotation", &format_args!("{:?}", self.rotation))
            .finish()
    }
}



// --- Geometry Buffer (modified for chunk meshes) ---
#[derive(Debug, Clone)]
pub struct GeometryBuffer {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub num_vertices: u32,
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
            num_vertices: vertices.len() as u32,
        }
    }

    pub fn empty(device: &wgpu::Device) -> Self {
        Self::new(device, &[], &[])
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
}

pub fn add_def_cube() {
    unsafe {
        let state = super::get_state();

        // Calculate where to place the cube (in front of the camera)
        let placement_distance = 6.0; // Distance in front of camera
        let placement_position = super::cube::vec3_f32_to_i32(
            state.camera_system.camera.position
                + state.camera_system.camera.forward() * placement_distance,
        );

        // Convert to chunk coordinates
        let chunk_pos = super::cube::ChunkCoord::from_world_pos(placement_position);

        // Convert to local position within chunk
        //let local_pos = super::cube::Chunk::world_to_local_pos(placement_position);

        // Create cube at the correct position
        let cube = super::cube::Block::new_raw(placement_position);

        state.data_system.world.set_block(placement_position, cube);

        // Add to instance manager with proper world position
        state.instance_manager().borrow_mut().add_instance(
            state.device(),
            state.queue(),
            cube.to_world_instance(chunk_pos),
        );
    }
}

// --- Instance Struct ---
#[repr(C)]
#[derive(Clone)]
pub struct Instance {
    pub position: Vector3<f32>,
    pub rotation: Quaternion<f32>,
}

impl Instance {
    #[inline] // ← Critical for performance
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (Matrix4::from_translation(self.position) * Matrix4::from(self.rotation)).into(),
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
            position: Vector3::new(0.0, 0.0, 0.0),
            rotation: Quaternion::from_angle_y(Deg(0.0)),
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
