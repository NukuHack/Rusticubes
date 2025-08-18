
use wgpu::util::DeviceExt;
use glam::IVec3;
use std::mem;

// =============================================
// Vertex Definition
// =============================================

/// A vertex with position, normal
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Vertex {
	pub position: u32,  // 4 bits per axis (x,y,z)
}
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
	pub packed_data: u32,  // 5 bits per axis (x,y,z) + normal index in 3 bits
}

impl Vertex {
	/// Describes the vertex buffer layout for wgpu
	pub const fn desc() -> wgpu::VertexBufferLayout<'static> {
		wgpu::VertexBufferLayout {
			array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Vertex,
			attributes: &[
				// Packed all
				wgpu::VertexAttribute {
					offset: 0,
					shader_location: 0,
					format: wgpu::VertexFormat::Uint32,
				},
			],
		}
	}
	pub const fn new(position: u32) -> Self {
		Self{ position }
	}
}
impl InstanceRaw {
	pub const fn desc() -> wgpu::VertexBufferLayout<'static> {
		wgpu::VertexBufferLayout {
			array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Instance, // This marks it as instance data
			attributes: &[
				wgpu::VertexAttribute {
					offset: 0,
					shader_location: 1,
					format: wgpu::VertexFormat::Uint32,
				},
			],
		}
	}
}

// =============================================
// Chunk Mesh Builder
// =============================================

/// Builder for constructing chunk meshes efficiently
pub struct ChunkMeshBuilder {
	pub instances: Vec<InstanceRaw>,
}
impl ChunkMeshBuilder {
	/// Creates a new mesh builder with optimized initial capacity
	#[inline] pub fn new() -> Self {
		Self { // set the starting capacity smaller because now with all the culling there is chance for a chunk to be invisible
			instances: Vec::new(),
		}
	}
	#[inline] pub fn build(self, device: &wgpu::Device) -> GeometryBuffer {
		GeometryBuffer::new(device, &self.instances)
	}
}

/// Make sure your CUBE_FACES constant matches the neighbor array order:
/// Normal vectors for face lookup
pub const CUBE_FACES: [IVec3; 6] = [
	IVec3::NEG_X, // [0] Left face
	IVec3::X,     // [1] Right face  
	IVec3::NEG_Z, // [2] Front face
	IVec3::Z,     // [3] Back face
	IVec3::Y,     // [4] Top face
	IVec3::NEG_Y, // [5] Bottom face
];
pub const VERTICES: [Vertex; 6] = {
	let p0 = Vertex::new((0 as u16 | (0 as u16) << 4 | (0 as u16) << 8) as u32);
	let p1 = Vertex::new((0 as u16 | (0 as u16) << 4 | (1 as u16) << 8) as u32);
	let p2 = Vertex::new((1 as u16 | (0 as u16) << 4 | (1 as u16) << 8) as u32);
	let p3 = Vertex::new((1 as u16 | (0 as u16) << 4 | (1 as u16) << 8) as u32);
	let p4 = Vertex::new((1 as u16 | (0 as u16) << 4 | (0 as u16) << 8) as u32);
	let p5 = Vertex::new((0 as u16 | (0 as u16) << 4 | (0 as u16) << 8) as u32);
	[p0, p1, p2, p3, p4, p5]
};


// =============================================
// Geometry Buffer
// =============================================

/// GPU buffer storage for geometry data
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryBuffer {
	pub instance_buffer: wgpu::Buffer,
	pub num_instances: u32,
}

impl GeometryBuffer {
	/// Creates a new geometry buffer with the given data
	#[inline] pub fn new(device: &wgpu::Device, instances: &[InstanceRaw]) -> Self {
		if instances.is_empty() {
			return Self::empty(device);
		}

		Self {
			instance_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("Vertex Buffer"),
				contents: bytemuck::cast_slice(instances),
				usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
			}),
			num_instances: instances.len() as u32,
		}
	}

	/// Creates an empty geometry buffer
	#[inline] pub fn empty(device: &wgpu::Device) -> Self {
		Self {
			instance_buffer : device.create_buffer(&wgpu::BufferDescriptor {
				label: Some("Empty Vertex Buffer"),
				size: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
				usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
				mapped_at_creation: false,
			}),
			num_instances: 0,
		}
	}
}
