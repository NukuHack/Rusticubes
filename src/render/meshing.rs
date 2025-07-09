
use crate::block::lut::{EDGE_TABLE, TRI_TABLE};
use crate::block::main::Chunk;
use crate::block::math::BlockPosition;
use glam::Vec3;
use std::mem;
use wgpu::util::DeviceExt;

// =============================================
// Vertex Definition
// =============================================

/// A vertex with position, normal, and UV coordinates
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Vertex {
    pub position: [f32; 3], //96 let it stay cus's why not ...
    pub normal: [f32; 3], //96 -> not used ...
    pub uv: [f32; 2], //64 -> 4 options : 2 bits
}

impl Vertex {
    /// Describes the vertex buffer layout for wgpu
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                // Position
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // Normal
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                // UV coordinates
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x2,
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

    /// Generates a marching cubes mesh for the given block
    pub fn add_marching_cube(&mut self, position: Vec3, points: u32) {
        if points == 0 || points == 0xFF_FF_FF_FF {
            return; // Early exit for empty or full cubes
        }

        let base_pos = position;
        let mut edge_vertex_cache = [None; 12];

        // Process each of the 8 sub-cubes
        for i in 0..8 {
            let case_index = Self::calculate_case_index(points, i);

            // Skip empty or full sub-cubes
            if case_index == 0 || case_index == 255 {
                continue;
            }

            let edges = EDGE_TABLE[case_index as usize];
            if edges == 0 {
                continue;
            }

            // Calculate sub-cube offset
            let sub_offset = Vec3::new(
                (i & 1) as f32 * 0.5,
                ((i >> 1) & 1) as f32 * 0.5,
                ((i >> 2) & 1) as f32 * 0.5,
            );

            // Cache edge vertices
            for edge in 0..12 {
                if (edges & (1 << edge)) != 0 && edge_vertex_cache[edge].is_none() {
                    let [a, b] = EDGE_VERTICES[edge];
                    edge_vertex_cache[edge] = Some(a.lerp(b, 0.5));
                }
            }

            // Generate triangles
            self.generate_triangles(case_index, &edge_vertex_cache, base_pos + sub_offset);

            // Clear cache for next sub-cube
            edge_vertex_cache = [None; 12];
        }
    }

    #[inline]
    fn calculate_case_index(points: u32, sub_cube_idx: usize) -> u8 {
        let mut case_index = 0u8;
        for bit in 0..8 {
            let (x, y, z) = match bit {
                0 => (0, 0, 0),
                1 => (1, 0, 0),
                2 => (1, 0, 1),
                3 => (0, 0, 1),
                4 => (0, 1, 0),
                5 => (1, 1, 0),
                6 => (1, 1, 1),
                7 => (0, 1, 1),
                _ => unreachable!(),
            };

            let x = x + ((sub_cube_idx & 1) as u8);
            let y = y + (((sub_cube_idx >> 1) & 1) as u8);
            let z = z + (((sub_cube_idx >> 2) & 1) as u8);

            let bit_pos = x as u32 + (y as u32) * 3 + (z as u32) * 9;
            if (points & (1u32 << bit_pos)) != 0 {
                case_index |= 1 << bit;
            }
        }
        case_index
    }

    #[inline]
    fn generate_triangles(
        &mut self,
        case_index: u8,
        edge_vertex_cache: &[Option<Vec3>; 12],
        position: Vec3,
    ) {
        let triangles = &TRI_TABLE[case_index as usize];
        let mut i = 0;

        while i < 16 && triangles[i] != -1 {
            let v0 = edge_vertex_cache[triangles[i] as usize].unwrap();
            let v1 = edge_vertex_cache[triangles[i + 1] as usize].unwrap();
            let v2 = edge_vertex_cache[triangles[i + 2] as usize].unwrap();

            self.add_triangle(&[position + v0, position + v1, position + v2]);
            i += 3;
        }
    }

    /// Adds a triangle to the mesh with calculated normals
    #[inline]
    pub fn add_triangle(&mut self, vertices: &[Vec3; 3]) {
        // Calculate face normal
        let edge1 = vertices[1] - vertices[0];
        let edge2 = vertices[2] - vertices[0];
        let normal = edge1.cross(edge2).normalize();
        let normal_arr = [normal.x, normal.y, normal.z];

        let base = self.current_vertex as u16;
        self.vertices.extend([
            Vertex {
                position: [vertices[0].x, vertices[0].y, vertices[0].z],
                normal: normal_arr,
                uv: [0f32, 0f32],
            },
            Vertex {
                position: [vertices[1].x, vertices[1].y, vertices[1].z],
                normal: normal_arr,
                uv: [1., 0f32],
            },
            Vertex {
                position: [vertices[2].x, vertices[2].y, vertices[2].z],
                normal: normal_arr,
                uv: [0.5, 1.],
            },
        ]);

        self.indices.extend([base, base + 1, base + 2]);
        self.current_vertex += 3;
    }
}

impl ChunkMeshBuilder {
    pub fn add_cube(&mut self, pos: Vec3, texture_map: [f32; 2], chunk: &Chunk, neighbors: &[Option<&Chunk>; 6]) {
        for (idx, normal) in CUBE_FACES.iter().enumerate() {
            let neighbor_pos:Vec3 = pos + *normal;
            
            if !self.should_cull_face(neighbor_pos, chunk, neighbors) {
                self.add_quad(pos, *normal, idx, texture_map);
            }
        }
    }

    fn should_cull_face(&self, pos: Vec3, chunk: &Chunk, neighbors: &[Option<&Chunk>; 6]) -> bool {    
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
            }, None => true, // No neighbor chunk means not loaded cull for now and reload mesh if it loads in
        }
    }

    fn get_neighbor_chunk_and_local_pos<'a>(&self, neighbor_pos: Vec3, neighbors: &[Option<&'a Chunk>; 6]) -> Option<&'a Chunk> {
        // Calculate which neighbor we need to check
        // The neighbor indices should match the order in CUBE_FACES_F:
        // [0] = Left (-X), [1] = Right (+X), [2] = Front (-Z), [3] = Back (+Z), [4] = Top (+Y), [5] = Bottom (-Y)
        // Precompute the bit patterns for comparison (avoids FP math)
        const LEFT_EDGE_BITS: u32 = (-1f32).to_bits();
        const RIGHT_EDGE_BITS: u32 = (16f32).to_bits();

        let pos_bits = neighbor_pos.to_array().map(|v| v.to_bits());

        // Check X axis first (most common)
        if pos_bits[0] == LEFT_EDGE_BITS { return neighbors[0]; }
        if pos_bits[0] == RIGHT_EDGE_BITS { return neighbors[1]; }
        
        // Then Z axis
        if pos_bits[2] == LEFT_EDGE_BITS { return neighbors[2]; }
        if pos_bits[2] == RIGHT_EDGE_BITS { return neighbors[3]; }
        
        // Finally Y axis
        if pos_bits[1] == RIGHT_EDGE_BITS { return neighbors[4]; }
        if pos_bits[1] == LEFT_EDGE_BITS { return neighbors[5]; }
        
        unreachable!("Position should be outside current chunk");
    }

    fn add_quad(&mut self, position: Vec3, normal: Vec3, idx:usize, texture_map: [f32; 2]) {
        let base = self.current_vertex as u16;
                
        let vertices = QUAD_VERTICES[idx];
        // Add vertices
        for i in 0..4 {
            self.vertices.push(Vertex {
                position: [
                    position.x + vertices[i][0],
                    position.y + vertices[i][1], 
                    position.z + vertices[i][2]
                ],
                normal: [normal.x, normal.y, normal.z],
                uv: QUAD_UVS[idx](texture_map)[i],
            });
        }
        // Add indices (two triangles forming a quad)
        self.indices.extend(&[base, base + 1, base + 2, base + 2, base + 3, base]);
        self.current_vertex += 4;
    }
}

/// Make sure your CUBE_FACES constant matches the neighbor array order:
/// Normal vectors for face lookup
const CUBE_FACES: [Vec3; 6] = [
    Vec3::NEG_X, // [0] Left face
    Vec3::X,     // [1] Right face  
    Vec3::NEG_Z, // [2] Front face
    Vec3::Z,     // [3] Back face
    Vec3::Y,     // [4] Top face
    Vec3::NEG_Y, // [5] Bottom face
];

/// Quad vertices relative to position for each face
const QUAD_VERTICES: [[[f32; 3]; 4]; 6] = [
    // Left face (NEG_X)
    [[0., 0., 0.], [0., 0., 1.], [0., 1., 1.], [0., 1., 0.]],
    // Right face (X)
    [[1., 0., 1.], [1., 0., 0.], [1., 1., 0.], [1., 1., 1.]],
    // Front face (NEG_Z)
    [[1., 0., 0.], [0., 0., 0.], [0., 1., 0.], [1., 1., 0.]],
    // Back face (Z)
    [[0., 0., 1.], [1., 0., 1.], [1., 1., 1.], [0., 1., 1.]],
    // Top face (Y)
    [[0., 1., 1.], [1., 1., 1.], [1., 1., 0.], [0., 1., 0.]],
    // Bottom face (NEG_Y)
    [[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]],
];

/// UV coordinates for each face
const QUAD_UVS: [fn([f32; 2]) -> [[f32; 2]; 4]; 6] = [
    |[fs, fe]| [[fe, fs], [fs, fs], [fs, fe], [fe, fe]], // Left
    |[fs, fe]| [[fs, fs], [fe, fs], [fe, fe], [fs, fe]], // Right
    |[fs, fe]| [[fe, fs], [fs, fs], [fs, fe], [fe, fe]], // Front
    |[fs, fe]| [[fs, fs], [fe, fs], [fe, fe], [fs, fe]], // Back
    |[fs, fe]| [[fs, fe], [fe, fe], [fe, fs], [fs, fs]], // Top
    |[fs, fe]| [[fs, fs], [fe, fs], [fe, fe], [fs, fe]], // Bottom
];

const HALF: f32 = 1.;

/// Edge vertices for the marching cubes algorithm
const EDGE_VERTICES: [[Vec3; 2]; 12] = [
    [Vec3::ZERO, Vec3::new(HALF, 0f32, 0f32)], // Edge 0
    [Vec3::new(HALF, 0f32, 0f32), Vec3::new(HALF, 0f32, HALF)], // Edge 1
    [Vec3::new(HALF, 0f32, HALF), Vec3::new(0f32, 0f32, HALF)], // Edge 2
    [Vec3::new(0f32, 0f32, HALF), Vec3::ZERO], // Edge 3
    [Vec3::new(0f32, HALF, 0f32), Vec3::new(HALF, HALF, 0f32)], // Edge 4
    [Vec3::new(HALF, HALF, 0f32), Vec3::new(HALF, HALF, HALF)], // Edge 5
    [Vec3::new(HALF, HALF, HALF), Vec3::new(0f32, HALF, HALF)], // Edge 6
    [Vec3::new(0f32, HALF, HALF), Vec3::new(0f32, HALF, 0f32)], // Edge 7
    [Vec3::ZERO, Vec3::new(0f32, HALF, 0f32)], // Edge 8
    [Vec3::new(HALF, 0f32, 0f32), Vec3::new(HALF, HALF, 0f32)], // Edge 9
    [Vec3::new(HALF, 0f32, HALF), Vec3::new(HALF, HALF, HALF)], // Edge 10
    [Vec3::new(0f32, 0f32, HALF), Vec3::new(0f32, HALF, HALF)], // Edge 11
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