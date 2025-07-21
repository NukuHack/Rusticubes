
use crate::block::main::Chunk;
use crate::block::math::BlockPosition;
use glam::IVec3;
use std::mem;
use wgpu::util::DeviceExt;

// =============================================
// Vertex Definition
// =============================================

/// A vertex with position, normal
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Vertex {
    pub position: u32,  // 5 bits per axis (x,y,z)
}
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
	pub packed_data: u32,  // 5 bits per axis (x,y,z) + normal index in bits 16-18
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
	pub const fn new(pos: u32) -> Self {
		Self{
			position: pos
		}
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
	#[inline]
	pub fn new() -> Self {
		Self { // set the starting capacity smaller because now with all the culling there is chance for a chunk to be invisible
			instances: Vec::with_capacity(Chunk::SIZE),
		}
	}
	// pos is allways 0-15
	pub fn add_cube(&mut self, pos: IVec3, chunk: &Chunk, neighbors: &[Option<&Chunk>; 6]) {
		for (idx, normal) in CUBE_FACES.iter().enumerate() {
			let neighbor_pos: IVec3 = pos + *normal;
			
			if !self.should_cull_face(neighbor_pos, chunk, neighbors) {
				self.add_quad(pos, idx);
			}
		}
	}
	#[inline]fn should_cull_face(&self, pos: IVec3, chunk: &Chunk, neighbors: &[Option<&Chunk>; 6]) -> bool {    
		// Check if position is inside current chunk
		let idx = usize::from(BlockPosition::from(pos));
		if chunk.contains_position(pos) {
			return !chunk.get_block(idx).is_empty();
		}
		
		// Position is in neighboring chunk
		let neighbor_chunk = self.get_neighbor_chunk_and_local_pos(pos, neighbors);
		
		match neighbor_chunk {
			Some(chunk) => {
				!chunk.get_block(idx).is_empty()
			}, 
			None => true, // No neighbor chunk means not loaded cull for now and reload mesh if it loads in
		}
	}
	#[inline]fn get_neighbor_chunk_and_local_pos<'a>(&self, neighbor_pos: IVec3, neighbors: &[Option<&'a Chunk>; 6]) -> Option<&'a Chunk> {
		// Calculate which neighbor we need to check
		// The neighbor indices should match the order in CUBE_FACES_F:
		// [0] = Left (-X), [1] = Right (+X), [2] = Front (-Z), [3] = Back (+Z), [4] = Top (+Y), [5] = Bottom (-Y)
		// Precompute the bit patterns for comparison (avoids FP math)

	    // Chunk boundaries in i32 coordinates
	    const LEFT_EDGE: i32 = -1;
	    const RIGHT_EDGE: i32 = Chunk::SIZE_I;
	    
	    // Check X axis first (most common)
	    if neighbor_pos.x == LEFT_EDGE { return neighbors[0]; }  // -X (Left)
	    if neighbor_pos.x == RIGHT_EDGE { return neighbors[1]; }  // +X (Right)
	    
	    // Then Z axis
	    if neighbor_pos.z == LEFT_EDGE { return neighbors[2]; }  // -Z (Front)
	    if neighbor_pos.z == RIGHT_EDGE { return neighbors[3]; }  // +Z (Back)
	    
	    // Finally Y axis
	    if neighbor_pos.y == RIGHT_EDGE { return neighbors[4]; }  // +Y (Top)
	    if neighbor_pos.y == LEFT_EDGE { return neighbors[5]; }   // -Y (Bottom)
	    
		unreachable!("Position should be outside current chunk");  // Position is inside current chunk
	}
	#[inline]fn add_quad(&mut self, position: IVec3, idx: usize) {		
		// Add instances without any UV data - shader will calculate everything

		let position:u16 = BlockPosition::from(position).into();
		self.instances.push(InstanceRaw {
			packed_data: (position as u32 | (idx as u32) << 12)
		});
	}
}

/// Make sure your CUBE_FACES constant matches the neighbor array order:
/// Normal vectors for face lookup
const CUBE_FACES: [IVec3; 6] = [
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
	pub fn new(device: &wgpu::Device, instances: &[InstanceRaw]) -> Self {
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
	pub fn empty(device: &wgpu::Device) -> Self {
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