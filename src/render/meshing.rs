
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
	pub packed_data: u32, 
}

impl Vertex {
	/// Describes the vertex buffer layout for wgpu
	pub fn desc() -> wgpu::VertexBufferLayout<'static> {
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
}

// =============================================
// Chunk Mesh Builder
// =============================================

/// Builder for constructing chunk meshes efficiently
pub struct ChunkMeshBuilder {
	pub vertices: Vec<Vertex>,
	pub indices: Vec<u16>,
	current_vertex: u32,
}

impl Default for ChunkMeshBuilder {
	#[inline]
	fn default() -> Self {
		Self::new()
	}
}

impl ChunkMeshBuilder {
	/// Creates a new mesh builder with optimized initial capacity
	#[inline]
	pub fn new() -> Self {
		Self { // set the starting capacity smaller because now with all the culling there is chance for a chunk to be invisible
			vertices: Vec::with_capacity(Chunk::SIZE * 4), // Average 4 vertices per cube
			indices: Vec::with_capacity(Chunk::SIZE * 6),  // Average 6 indices per cube
			current_vertex: 0,
		}
	}
	pub fn add_cube(&mut self, pos: IVec3, _texture_map: [f32; 2], chunk: &Chunk, neighbors: &[Option<&Chunk>; 6]) {
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
	    const RIGHT_EDGE: i32 = 16;  // Assuming 16x16x16 chunks
	    
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
		let base = self.current_vertex as u16;
				
		let vertices = QUAD_VERTICES[idx];
		
		// Add vertices without any UV data - shader will calculate everything
		for i in 0..4 {
			let position = u16::from((position.x + vertices[i][0]) as u16 | ((position.y + vertices[i][1]) as u16) << 5 | ((position.z + vertices[i][2]) as u16) << 10) as u32;
			self.vertices.push(Vertex {
				packed_data: (position | (idx as u32) << 16)
			});
		}
		
		// Add indices (two triangles forming a quad)
		// This order matches the shader's expectation:
		// Vertex 0: (0, 0) - bottom-left
		// Vertex 1: (1, 0) - bottom-right  
		// Vertex 2: (1, 1) - top-right
		// Vertex 3: (0, 1) - top-left
		self.indices.extend(&[base, base + 1, base + 2, base + 2, base + 3, base]);
		self.current_vertex += 4;
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

/// Quad vertices relative to position for each face
/// These define the actual 3D positions of the quad corners
///
///	2-->3
/// ^
///	|
/// 1<--0
///
const QUAD_VERTICES: [[[i32; 3]; 4]; 6] = [
    // Left face (NEG_X) - vertices ordered for consistent winding
    [[0, 0, 0], [0, 0, 1], [0, 1, 1], [0, 1, 0]],
    // Right face (X)
    [[1, 0, 1], [1, 0, 0], [1, 1, 0], [1, 1, 1]],
    // Front face (NEG_Z)
    [[1, 0, 0], [0, 0, 0], [0, 1, 0], [1, 1, 0]],
    // Back face (Z)
    [[0, 0, 1], [1, 0, 1], [1, 1, 1], [0, 1, 1]],
    // Top face (Y)
    [[0, 1, 1], [1, 1, 1], [1, 1, 0], [0, 1, 0]],
    // Bottom face (NEG_Y)
    [[0, 0, 0], [1, 0, 0], [1, 0, 1], [0, 0, 1]],
];

// =============================================
// Geometry Buffer
// =============================================

/// GPU buffer storage for geometry data
#[derive(Debug, Clone, PartialEq)]
pub struct GeometryBuffer {
	pub vertex_buffer: wgpu::Buffer,
	pub index_buffer: wgpu::Buffer,
	pub num_indices: u32,
	pub num_vertices: u32,
	pub vertex_capacity: usize,
	pub index_capacity: usize,
}

impl GeometryBuffer {
	/// Creates a new geometry buffer with the given data
	pub fn new(device: &wgpu::Device, indices: &[u16], vertices: &[Vertex]) -> Self {
		if vertices.is_empty() && indices.is_empty() {
			return Self::empty(device);
		}

		Self {
			vertex_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("Vertex Buffer"),
				contents: bytemuck::cast_slice(vertices),
				usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
			}),
			index_buffer: device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("Index Buffer"),
				contents: bytemuck::cast_slice(indices),
				usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
			}),
			num_indices: indices.len() as u32,
			num_vertices: vertices.len() as u32,
			vertex_capacity: vertices.len(),
			index_capacity: indices.len(),
		}
	}

	/// Creates an empty geometry buffer
	pub fn empty(device: &wgpu::Device) -> Self {
		let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label: Some("Empty Vertex Buffer"),
			size: mem::size_of::<Vertex>() as wgpu::BufferAddress,
			usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});

		let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label: Some("Empty Index Buffer"),
			size: mem::size_of::<u16>() as wgpu::BufferAddress,
			usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});

		Self {
			vertex_buffer,
			index_buffer,
			num_indices: 0,
			num_vertices: 0,
			vertex_capacity: 0,
			index_capacity: 0,
		}
	}

	/// Updates the buffer contents, reallocating if necessary
	pub fn update(
		&mut self,
		device: &wgpu::Device,
		queue: &wgpu::Queue,
		indices: &[u16],
		vertices: &[Vertex],
	) {
		// Helper to align buffer sizes
		#[inline]
		fn align_size(size: usize, alignment: usize) -> usize {
			((size + alignment - 1) / alignment) * alignment
		}

		// Handle vertex buffer update
		if vertices.len() > self.vertex_capacity {
			// Reallocate if capacity is insufficient
			self.vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("Vertex Buffer"),
				contents: bytemuck::cast_slice(vertices),
				usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
			});
			self.vertex_capacity = vertices.len();
		} else if !vertices.is_empty() {
			// Update existing buffer with proper alignment
			let vertex_slice = bytemuck::cast_slice(vertices);
			let aligned_size = align_size(vertex_slice.len(), wgpu::COPY_BUFFER_ALIGNMENT as usize);
			let mut aligned_data = vertex_slice.to_vec();
			aligned_data.resize(aligned_size, 0);
			queue.write_buffer(&self.vertex_buffer, 0, &aligned_data);
		}

		// Handle index buffer update
		if indices.len() > self.index_capacity {
			// Reallocate if capacity is insufficient
			self.index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("Index Buffer"),
				contents: bytemuck::cast_slice(indices),
				usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
			});
			self.index_capacity = indices.len();
		} else if !indices.is_empty() {
			// Update existing buffer with proper alignment
			let index_slice = bytemuck::cast_slice(indices);
			let aligned_size = align_size(index_slice.len(), wgpu::COPY_BUFFER_ALIGNMENT as usize);
			let mut aligned_data = index_slice.to_vec();
			aligned_data.resize(aligned_size, 0);
			queue.write_buffer(&self.index_buffer, 0, &aligned_data);
		}

		self.num_indices = indices.len() as u32;
		self.num_vertices = vertices.len() as u32;
	}
}