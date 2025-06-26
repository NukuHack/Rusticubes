
use super::cube_tables::{EDGE_TABLE, TRI_TABLE};
use super::cube::Chunk;
use glam::{Mat4, Quat, Vec3};
use std::mem;
use wgpu::util::DeviceExt;

// =============================================
// Vertex Definition
// =============================================

/// A vertex with position, normal, and UV coordinates
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
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
        Self {
            vertices: Vec::with_capacity(Chunk::VOLUME * 4), // Average 4 vertices per cube
            indices: Vec::with_capacity(Chunk::VOLUME * 6),  // Average 6 indices per cube
            current_vertex: 0,
        }
    }

    /// Generates a marching cubes mesh for the given block
    pub fn add_marching_cube(&mut self, points: u32, position: Vec3) {
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
                uv: [0.0, 0.0],
            },
            Vertex {
                position: [vertices[1].x, vertices[1].y, vertices[1].z],
                normal: normal_arr,
                uv: [1.0, 0.0],
            },
            Vertex {
                position: [vertices[2].x, vertices[2].y, vertices[2].z],
                normal: normal_arr,
                uv: [0.5, 1.0],
            },
        ]);

        self.indices.extend([base, base + 1, base + 2]);
        self.current_vertex += 3;
    }

    /// Adds a rotated cube to the mesh with face culling
    pub fn add_cube(
        &mut self,
        position: Vec3,
        rotation: Quat,
        texture_map: [f32; 2],
        chunk: &Chunk,
    ) {
        let [fs, fe] = texture_map;
        let transform = Mat4::from_rotation_translation(rotation, position);

        // Check face visibility (culling)
        let face_visibility = [
            chunk.is_block_cull(position + Vec3::Z), // Front
            chunk.is_block_cull(position - Vec3::Z), // Back
            chunk.is_block_cull(position + Vec3::Y), // Top
            chunk.is_block_cull(position - Vec3::Y), // Bottom
            chunk.is_block_cull(position + Vec3::X), // Right
            chunk.is_block_cull(position - Vec3::X), // Left
        ];

        // Early exit if no faces are visible
        if !face_visibility.iter().any(|&v| v) {
            return;
        }

        // Transform all vertices upfront
        let mut transformed_vertices = [[0.0f32; 3]; 8];
        for (i, vertex) in CUBE_VERTICES.iter().enumerate() {
            let pos = transform.transform_point3(Vec3::from(*vertex));
            transformed_vertices[i] = [pos.x, pos.y, pos.z];
        }

        // Add visible faces
        for (face_idx, &visible) in face_visibility.iter().enumerate() {
            if !visible {
                continue;
            }

            let face = &CUBE_FACES[face_idx];
            let uv = match face_idx {
                0 => [[fs, fs], [fe, fs], [fe, fe], [fs, fe]], // Front
                1 => [[fe, fs], [fs, fs], [fs, fe], [fe, fe]], // Back
                2 => [[fs, fe], [fe, fe], [fe, fs], [fs, fs]], // Top
                3 => [[fs, fs], [fe, fs], [fe, fe], [fs, fe]], // Bottom
                4 => [[fs, fs], [fe, fs], [fe, fe], [fs, fe]], // Right
                5 => [[fe, fs], [fs, fs], [fs, fe], [fe, fe]], // Left
                _ => unreachable!(),
            };

            let base = self.current_vertex as u16;
            for (i, &vertex_idx) in face.iter().enumerate() {
                self.vertices.push(Vertex {
                    position: transformed_vertices[vertex_idx as usize],
                    normal: CUBE_FACE_NORMALS[face_idx],
                    uv: uv[i],
                });
            }

            // Add two triangles (quad)
            self.indices
                .extend(&[base, base + 1, base + 2, base + 2, base + 3, base]);
            self.current_vertex += 4;
        }
    }
}

// =============================================
// Marching Cubes Constants
// =============================================

const HALF: f32 = 0.5;

/// Edge vertices for the marching cubes algorithm
const EDGE_VERTICES: [[Vec3; 2]; 12] = [
    [Vec3::ZERO, Vec3::new(HALF, 0.0, 0.0)], // Edge 0
    [Vec3::new(HALF, 0.0, 0.0), Vec3::new(HALF, 0.0, HALF)], // Edge 1
    [Vec3::new(HALF, 0.0, HALF), Vec3::new(0.0, 0.0, HALF)], // Edge 2
    [Vec3::new(0.0, 0.0, HALF), Vec3::ZERO], // Edge 3
    [Vec3::new(0.0, HALF, 0.0), Vec3::new(HALF, HALF, 0.0)], // Edge 4
    [Vec3::new(HALF, HALF, 0.0), Vec3::new(HALF, HALF, HALF)], // Edge 5
    [Vec3::new(HALF, HALF, HALF), Vec3::new(0.0, HALF, HALF)], // Edge 6
    [Vec3::new(0.0, HALF, HALF), Vec3::new(0.0, HALF, 0.0)], // Edge 7
    [Vec3::ZERO, Vec3::new(0.0, HALF, 0.0)], // Edge 8
    [Vec3::new(HALF, 0.0, 0.0), Vec3::new(HALF, HALF, 0.0)], // Edge 9
    [Vec3::new(HALF, 0.0, HALF), Vec3::new(HALF, HALF, HALF)], // Edge 10
    [Vec3::new(0.0, 0.0, HALF), Vec3::new(0.0, HALF, HALF)], // Edge 11
];

// =============================================
// Cube Geometry Constants
// =============================================

const CUBE_SIZE: f32 = 1.0; // Unit-sized cube

/// Cube vertices (8 corners of a unit cube)
const CUBE_VERTICES: [[f32; 3]; 8] = [
    [0.0, 0.0, CUBE_SIZE],             // front-bottom-left
    [CUBE_SIZE, 0.0, CUBE_SIZE],       // front-bottom-right
    [CUBE_SIZE, CUBE_SIZE, CUBE_SIZE], // front-top-right
    [0.0, CUBE_SIZE, CUBE_SIZE],       // front-top-left
    [0.0, 0.0, 0.0],                   // back-bottom-left
    [CUBE_SIZE, 0.0, 0.0],             // back-bottom-right
    [CUBE_SIZE, CUBE_SIZE, 0.0],       // back-top-right
    [0.0, CUBE_SIZE, 0.0],             // back-top-left
];

/// Face normals for each cube face
const CUBE_FACE_NORMALS: [[f32; 3]; 6] = [
    [0.0, 0.0, 1.0],  // Front
    [0.0, 0.0, -1.0], // Back
    [0.0, 1.0, 0.0],  // Top
    [0.0, -1.0, 0.0], // Bottom
    [1.0, 0.0, 0.0],  // Right
    [-1.0, 0.0, 0.0], // Left
];

/// Cube faces defined as quads (will be converted to triangles)
const CUBE_FACES: [[u16; 4]; 6] = [
    [0, 1, 2, 3], // Front face
    [5, 4, 7, 6], // Back face
    [3, 2, 6, 7], // Top face
    [4, 5, 1, 0], // Bottom face
    [1, 5, 6, 2], // Right face
    [4, 0, 3, 7], // Left face
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
