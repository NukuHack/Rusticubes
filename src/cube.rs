use crate::geometry::{GeometryBuffer, Vertex, EDGE_TABLE, TRI_TABLE};
use crate::traits::VectorTypeConversion;
use ahash::AHasher;
use cgmath::{Deg, InnerSpace, Matrix4, Quaternion, Rotation3, Vector3, VectorSpace};
use std::{
    collections::{HashMap, HashSet},
    hash::BuildHasherDefault,
};
use wgpu::util::DeviceExt;

// Type aliases for better readability
type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;

/// Represents a block in the world with optimized storage
#[derive(Clone, Copy, Debug)]
pub enum Block {
    /// Simple block with material and packed rotation
    Simple {
        material: u16,
        rotation: u8, // Packed as [X:2, Y:2, Z:2, _:2]
    },
    /// Marching cubes block with material and density field
    Marching {
        material: u16,
        points: u32, // 3x3x3 density field (27 bits)
    },
}

impl Block {
    // Rotation bit masks and shifts
    const ROT_MASK_X: u8 = 0b0000_0011;
    const ROT_MASK_Y: u8 = 0b0000_1100;
    const ROT_MASK_Z: u8 = 0b0011_0000;
    const ROT_SHIFT_X: u8 = 0;
    const ROT_SHIFT_Y: u8 = 2;
    const ROT_SHIFT_Z: u8 = 4;

    /// Creates a default empty block
    #[inline]
    pub const fn default() -> Self {
        Self::Simple {
            material: 0,
            rotation: 0,
        }
    }

    /// Creates a new simple block with default material
    #[inline]
    pub const fn new() -> Self {
        Self::Simple {
            material: 1,
            rotation: 0,
        }
    }

    /// Creates a new marching cubes block with center point set
    #[inline]
    pub const fn new_dot() -> Self {
        Self::Marching {
            material: 1,
            points: 0x20_00, // Center point set
        }
    }

    /// Creates a new block with specified rotation
    #[inline]
    pub const fn new_rot(rotation: u8) -> Self {
        Self::Simple {
            material: 1,
            rotation,
        }
    }

    /// Creates a block from a quaternion rotation
    #[inline]
    pub fn from_quaternion(rotation: Quaternion<f32>) -> Self {
        Self::Simple {
            material: 1,
            rotation: Self::quaternion_to_rotation(rotation),
        }
    }

    /// Extracts rotation components (0-3)
    #[inline]
    pub fn get_rotation(&self) -> Option<(u8, u8, u8)> {
        match self {
            Block::Simple { rotation, .. } => Some((
                (rotation & Self::ROT_MASK_X) >> Self::ROT_SHIFT_X,
                (rotation & Self::ROT_MASK_Y) >> Self::ROT_SHIFT_Y,
                (rotation & Self::ROT_MASK_Z) >> Self::ROT_SHIFT_Z,
            )),
            _ => None,
        }
    }

    /// Converts packed rotation to quaternion
    #[inline]
    pub fn rotation_to_quaternion(&self) -> Quaternion<f32> {
        self.get_rotation()
            .map(|(x, y, z)| {
                // Convert to degrees (0, 90, 180, 270)
                let angles = [
                    Deg(x as f32 * 90.0),
                    Deg(y as f32 * 90.0),
                    Deg(z as f32 * 90.0),
                ];

                // Apply rotations in ZYX order
                Quaternion::from_angle_z(angles[2])
                    * Quaternion::from_angle_y(angles[1])
                    * Quaternion::from_angle_x(angles[0])
            })
            .unwrap_or_else(|| Quaternion::new(1.0, 0.0, 0.0, 0.0)) // Identity quaternion
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        match self {
            Block::Simple { material, .. } | Block::Marching { material, .. } => *material == 0,
        }
    }

    #[inline]
    pub fn is_marching(&self) -> bool {
        matches!(self, Block::Marching { .. })
    }

    #[inline]
    pub fn material(&self) -> u16 {
        match self {
            Block::Simple { material, .. } | Block::Marching { material, .. } => *material,
        }
    }

    #[inline]
    pub fn set_material(&mut self, material: u16) {
        match self {
            Block::Simple { material: m, .. } | Block::Marching { material: m, .. } => {
                *m = material
            }
        }
    }

    /// Rotates the block around an axis by N 90° steps
    #[inline]
    pub fn rotate(&mut self, axis: Axis, steps: u8) {
        if let Block::Simple { rotation, .. } = self {
            let (mask, shift) = match axis {
                Axis::X => (Self::ROT_MASK_X, Self::ROT_SHIFT_X),
                Axis::Y => (Self::ROT_MASK_Y, Self::ROT_SHIFT_Y),
                Axis::Z => (Self::ROT_MASK_Z, Self::ROT_SHIFT_Z),
            };

            let current = (*rotation & mask) >> shift;
            let new_rot = (current + steps) % 4;
            *rotation = (*rotation & !mask) | (new_rot << shift);
        }
    }

    /// Converts quaternion to packed rotation format
    #[inline]
    pub fn quaternion_to_rotation(rotation: Quaternion<f32>) -> u8 {
        // Extract Euler angles using efficient quaternion conversion
        let angles = [
            (2.0 * (rotation.s * rotation.v.x + rotation.v.y * rotation.v.z))
                .atan2(1.0 - 2.0 * (rotation.v.x.powi(2) + rotation.v.y.powi(2))),
            (2.0 * (rotation.s * rotation.v.y - rotation.v.z * rotation.v.x)).asin(),
            (2.0 * (rotation.s * rotation.v.z + rotation.v.x * rotation.v.y))
                .atan2(1.0 - 2.0 * (rotation.v.y.powi(2) + rotation.v.z.powi(2))),
        ];

        // Convert to 2-bit values (0-3) representing 90-degree increments
        let x = ((angles[0].rem_euclid(std::f32::consts::TAU) / std::f32::consts::FRAC_PI_2).round()
            as u8)
            & 0x3;
        let y = ((angles[1].rem_euclid(std::f32::consts::TAU) / std::f32::consts::FRAC_PI_2).round()
            as u8)
            & 0x3;
        let z = ((angles[2].rem_euclid(std::f32::consts::TAU) / std::f32::consts::FRAC_PI_2).round()
            as u8)
            & 0x3;

        // Pack into a single byte
        x | (y << 2) | (z << 4)
    }

    /// Sets all rotation axes at once
    #[inline]
    pub fn set_rotation(&mut self, x: u8, y: u8, z: u8) {
        if let Block::Simple { rotation, .. } = self {
            *rotation = (x & 0x3) | ((y & 0x3) << 2) | ((z & 0x3) << 4);
        }
    }

    /// Sets a point in the 3x3x3 density field
    #[inline]
    pub fn set_point(&mut self, (x, y, z, value): (u8, u8, u8, bool)) {
        if let Block::Marching { points, .. } = self {
            debug_assert!(x < 3 && y < 3 && z < 3, "Coordinates must be 0-2");
            let bit_pos = x as u32 + (y as u32) * 3 + (z as u32) * 9;
            let mask = 1u32 << bit_pos;

            // Use bitwise operations instead of branches
            *points = (*points & !mask) | (if value { mask } else { 0 });
        }
    }

    /// Gets a point from the 3x3x3 density field
    #[inline]
    pub fn get_point(&self, x: u8, y: u8, z: u8) -> Option<bool> {
        match self {
            Block::Marching { points, .. } => {
                debug_assert!(x < 3 && y < 3 && z < 3, "Coordinates must be 0-2");
                let bit_pos = x as u32 + (y as u32) * 3 + (z as u32) * 9;
                Some((*points & (1u32 << bit_pos)) != 0)
            }
            _ => None,
        }
    }

    /// Generates marching cubes mesh for this block
    pub fn generate_marching_cubes_mesh(
        &self,
        position: Vector3<u32>,
        builder: &mut ChunkMeshBuilder,
    ) {
        let Block::Marching { points, .. } = self else {
            return;
        };

        let base_pos = position.to_vec3_f32() - Vector3::unit_z(); // Z-offset adjustment
        let mut edge_vertex_cache = [None; 12]; // Cache calculated edge vertices

        // Process each sub-cube
        for sub_z in 0..2 {
            for sub_y in 0..2 {
                for sub_x in 0..2 {
                    // Calculate case index using bit manipulation
                    let idx = [
                        (sub_x, sub_y, sub_z),
                        (sub_x + 1, sub_y, sub_z),
                        (sub_x + 1, sub_y, sub_z + 1),
                        (sub_x, sub_y, sub_z + 1),
                        (sub_x, sub_y + 1, sub_z),
                        (sub_x + 1, sub_y + 1, sub_z),
                        (sub_x + 1, sub_y + 1, sub_z + 1),
                        (sub_x, sub_y + 1, sub_z + 1),
                    ];

                    let mut case_index = 0u8;
                    for (i, &(x, y, z)) in idx.iter().enumerate() {
                        let bit_pos = x as u32 + (y as u32) * 3 + (z as u32) * 9;
                        if (*points & (1u32 << bit_pos)) != 0 {
                            case_index |= 1 << i;
                        }
                    }

                    // Skip empty/full sub-cubes
                    if case_index == 0 || case_index == 255 {
                        continue;
                    }

                    let edges = EDGE_TABLE[case_index as usize];
                    if edges == 0 {
                        continue;
                    }

                    // Calculate and cache edge vertices only when needed
                    for edge in 0..12 {
                        if (edges & (1 << edge)) != 0 && edge_vertex_cache[edge].is_none() {
                            let [a, b] = EDGE_VERTICES[edge];
                            edge_vertex_cache[edge] = Some(a.lerp(b, 0.5));
                        }
                    }

                    // Generate triangles
                    let triangles = &TRI_TABLE[case_index as usize];
                    let sub_offset =
                        Vector3::new(sub_x as f32 * 0.5, sub_y as f32 * 0.5, sub_z as f32 * 0.5);

                    // Process triangles in batches
                    let mut i = 0;
                    while i < 16 && triangles[i] != -1 {
                        let tri_vertices = [
                            edge_vertex_cache[triangles[i] as usize].unwrap(),
                            edge_vertex_cache[triangles[i + 1] as usize].unwrap(),
                            edge_vertex_cache[triangles[i + 2] as usize].unwrap(),
                        ];

                        let world_vertices = [
                            base_pos + sub_offset + tri_vertices[0],
                            base_pos + sub_offset + tri_vertices[1],
                            base_pos + sub_offset + tri_vertices[2],
                        ];

                        builder.add_triangle(&world_vertices);
                        i += 3;
                    }

                    // Clear the cache for the next sub-cube
                    edge_vertex_cache = [None; 12];
                }
            }
        }
    }
}

/// Edge vertices for marching cubes algorithm
const HALF: f32 = 0.5;
const EDGE_VERTICES: [[Vector3<f32>; 2]; 12] = [
    [Vector3::new(0.0, 0.0, 0.0), Vector3::new(HALF, 0.0, 0.0)], // Edge 0
    [Vector3::new(HALF, 0.0, 0.0), Vector3::new(HALF, 0.0, HALF)], // Edge 1
    [Vector3::new(HALF, 0.0, HALF), Vector3::new(0.0, 0.0, HALF)], // Edge 2
    [Vector3::new(0.0, 0.0, HALF), Vector3::new(0.0, 0.0, 0.0)], // Edge 3
    [Vector3::new(0.0, HALF, 0.0), Vector3::new(HALF, HALF, 0.0)], // Edge 4
    [
        Vector3::new(HALF, HALF, 0.0),
        Vector3::new(HALF, HALF, HALF),
    ], // Edge 5
    [
        Vector3::new(HALF, HALF, HALF),
        Vector3::new(0.0, HALF, HALF),
    ], // Edge 6
    [Vector3::new(0.0, HALF, HALF), Vector3::new(0.0, HALF, 0.0)], // Edge 7
    [Vector3::new(0.0, 0.0, 0.0), Vector3::new(0.0, HALF, 0.0)], // Edge 8
    [Vector3::new(HALF, 0.0, 0.0), Vector3::new(HALF, HALF, 0.0)], // Edge 9
    [
        Vector3::new(HALF, 0.0, HALF),
        Vector3::new(HALF, HALF, HALF),
    ], // Edge 10
    [Vector3::new(0.0, 0.0, HALF), Vector3::new(0.0, HALF, HALF)], // Edge 11
];

/// Axis enumeration for rotation
#[derive(Debug, Clone, Copy)]
pub enum Axis {
    X,
    Y,
    Z,
}

/// Compact chunk coordinate representation (64 bits)
/// Format: [X:26 (signed), Y:12 (signed), Z:26 (signed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkCoord(u64);

impl ChunkCoord {
    const Z_SHIFT: u8 = 0;
    const Y_SHIFT: u8 = 26;
    const X_SHIFT: u8 = 38;
    const Z_MASK: u64 = 0x03FF_FFFF; // 26 bits
    const Y_MASK: u64 = 0x0FFF; // 12 bits
    const X_MASK: u64 = 0x03FF_FFFF; // 26 bits

    /// Creates a new chunk coordinate
    #[inline]
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self(Self::pack(x, y, z))
    }

    /// Packs coordinates into a u64
    #[inline]
    pub const fn pack(x: i32, y: i32, z: i32) -> u64 {
        ((x as u64 & Self::X_MASK) << Self::X_SHIFT)
            | ((y as u64 & Self::Y_MASK) << Self::Y_SHIFT)
            | (z as u64 & Self::Z_MASK)
    }

    /// Extracts (x, y, z) coordinates
    #[inline]
    pub const fn unpack(self) -> (i32, i32, i32) {
        (self.x(), self.y(), self.z())
    }

    /// Extracts X coordinate with sign extension
    #[inline]
    pub const fn x(self) -> i32 {
        ((self.0 >> Self::X_SHIFT) as i32)
            .wrapping_shl(6)
            .wrapping_shr(6)
    }

    /// Extracts Y coordinate with sign extension
    #[inline]
    pub const fn y(self) -> i32 {
        ((self.0 >> Self::Y_SHIFT) as i32 & Self::Y_MASK as i32)
            .wrapping_shl(20)
            .wrapping_shr(20)
    }

    /// Extracts Z coordinate with sign extension
    #[inline]
    pub const fn z(self) -> i32 {
        (self.0 as i32 & Self::Z_MASK as i32)
            .wrapping_shl(6)
            .wrapping_shr(6)
    }

    /// Converts to world position (chunk min corner)
    #[inline]
    pub fn to_world_pos(self) -> Vector3<i32> {
        let chunk_size = Chunk::SIZE_I32;
        Vector3::new(
            self.x() * chunk_size,
            self.y() * chunk_size,
            self.z() * chunk_size,
        )
    }

    /// Creates from world position
    #[inline]
    pub fn from_world_pos(world_pos: Vector3<i32>) -> Self {
        let chunk_size = Chunk::SIZE_I32;
        Self::new(
            world_pos.x.div_euclid(chunk_size),
            world_pos.y.div_euclid(chunk_size),
            world_pos.z.div_euclid(chunk_size),
        )
    }
}

/// Represents a chunk of blocks in the world
#[derive(Clone, Debug)]
pub struct Chunk {
    pub blocks: FastMap<u16, Block>, // Packed position -> Block
    pub dirty: bool,
    pub mesh: Option<GeometryBuffer>,
    pub bind_group: Option<wgpu::BindGroup>,
}

impl Chunk {
    pub const SIZE: usize = 16;
    pub const SIZE_I32: i32 = Self::SIZE as i32;
    pub const VOLUME: usize = Self::SIZE.pow(3);

    /// Creates an empty chunk
    #[inline]
    pub fn empty() -> Self {
        Self {
            blocks: FastMap::with_capacity_and_hasher(
                Self::VOLUME,
                BuildHasherDefault::<AHasher>::default(),
            ),
            dirty: false,
            mesh: None,
            bind_group: None,
        }
    }

    /// Creates a new filled chunk
    pub fn new() -> Self {
        let mut chunk = Self::empty();
        chunk.blocks.reserve(Self::VOLUME);

        for x in 0..Self::SIZE {
            for y in 0..Self::SIZE {
                for z in 0..Self::SIZE {
                    let pos = Self::local_to_block_pos(Vector3::new(x as u32, y as u32, z as u32));
                    chunk.blocks.insert(pos, Block::new());
                }
            }
        }

        chunk.dirty = true;
        chunk
    }

    #[inline]
    pub fn load() -> Option<Self> {
        Some(Self::new())
    }

    #[inline]
    pub fn get_block(&self, local_pos: Vector3<u32>) -> Option<&Block> {
        self.blocks.get(&Self::local_to_block_pos(local_pos))
    }

    #[inline]
    pub fn get_block_mut(&mut self, local_pos: Vector3<u32>) -> Option<&mut Block> {
        self.blocks.get_mut(&Self::local_to_block_pos(local_pos))
    }

    pub fn set_block(&mut self, local_pos: Vector3<u32>, block: Block) {
        let pos = Self::local_to_block_pos(local_pos);
        if block.is_empty() {
            self.blocks.remove(&pos);
        } else {
            self.blocks.insert(pos, block);
        }
        self.dirty = true;
    }

    /// Converts world position to local chunk coordinates
    #[inline]
    pub fn world_to_local_pos(world_pos: Vector3<i32>) -> Vector3<u32> {
        Vector3::new(
            world_pos.x.rem_euclid(Self::SIZE_I32) as u32,
            world_pos.y.rem_euclid(Self::SIZE_I32) as u32,
            world_pos.z.rem_euclid(Self::SIZE_I32) as u32,
        )
    }

    /// Packs local coordinates into u16
    #[inline]
    pub fn local_to_block_pos(local_pos: Vector3<u32>) -> u16 {
        debug_assert!(
            local_pos.x < Self::SIZE as u32
                && local_pos.y < Self::SIZE as u32
                && local_pos.z < Self::SIZE as u32,
            "Position out of bounds"
        );
        ((local_pos.x as u16) << 8) | ((local_pos.y as u16) << 4) | (local_pos.z as u16)
    }

    /// Unpacks position key to local coordinates
    #[inline]
    pub fn block_pos_to_local(pos: u16) -> Vector3<u32> {
        Vector3::new(
            ((pos >> 8) & 0xF) as u32,
            ((pos >> 4) & 0xF) as u32,
            (pos & 0xF) as u32,
        )
    }

    /// Generates the chunk mesh if dirty
    pub fn make_mesh(&mut self, device: &wgpu::Device, force: bool) {
        if !force && (!self.dirty || self.mesh.is_some()) {
            return;
        }

        let mut builder = ChunkMeshBuilder::new();

        for (&pos, block) in &self.blocks {
            if block.is_empty() {
                continue;
            }

            let local_pos = Self::block_pos_to_local(pos);
            if block.is_marching() {
                block.generate_marching_cubes_mesh(local_pos, &mut builder);
            } else {
                builder.add_cube(local_pos.to_vec3_f32(), block.rotation_to_quaternion());
            }
        }

        self.mesh = Some(builder.build(device));
        self.dirty = false;
    }
}

/// Represents the game world containing chunks
#[derive(Debug, Clone)]
pub struct World {
    pub chunks: FastMap<ChunkCoord, Chunk>,
    pub loaded_chunks: HashSet<ChunkCoord>,
}

impl World {
    /// Creates an empty world
    #[inline]
    pub fn empty() -> Self {
        Self {
            chunks: FastMap::with_capacity_and_hasher(
                10_000,
                BuildHasherDefault::<AHasher>::default(),
            ),
            loaded_chunks: HashSet::with_capacity(10_000),
        }
    }

    #[inline]
    pub fn get_chunk(&self, coord: ChunkCoord) -> Option<&Chunk> {
        self.chunks.get(&coord)
    }

    #[inline]
    pub fn get_chunk_mut(&mut self, coord: ChunkCoord) -> Option<&mut Chunk> {
        self.chunks.get_mut(&coord)
    }

    #[inline]
    pub fn get_block(&self, world_pos: Vector3<i32>) -> Option<&Block> {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);
        self.chunks.get(&chunk_coord).and_then(|chunk| {
            let local = Chunk::world_to_local_pos(world_pos);
            chunk.get_block(local)
        })
    }

    #[inline]
    pub fn get_block_mut(&mut self, world_pos: Vector3<i32>) -> Option<&mut Block> {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);
        self.chunks.get_mut(&chunk_coord).and_then(|chunk| {
            let local = Chunk::world_to_local_pos(world_pos);
            chunk.get_block_mut(local)
        })
    }

    pub fn set_block(&mut self, world_pos: Vector3<i32>, block: Block) {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);

        if !self.chunks.contains_key(&chunk_coord) {
            self.set_chunk(chunk_coord, Chunk::empty());
        }

        if let Some(chunk) = self.chunks.get_mut(&chunk_coord) {
            let local = Chunk::world_to_local_pos(world_pos);
            chunk.set_block(local, block);
        }
    }

    /// Loads a chunk from storage
    pub fn load_chunk(&mut self, chunk_coord: ChunkCoord) -> bool {
        unsafe {
            let state = super::get_state();
            let device = &state.render_context.device;
            let chunk_bind_group_layout = &state.render_context.chunk_bind_group_layout;

            // Unload existing chunk first
            if self.get_chunk(chunk_coord).is_some() {
                self.unload_chunk(chunk_coord);
            }

            let Some(chunk) = Chunk::load() else {
                return false;
            };

            // Create position buffer
            let world_pos = chunk_coord.to_world_pos().to_vec3_f32();
            let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Chunk Position Buffer"),
                contents: bytemuck::cast_slice(&[world_pos.x, world_pos.y, world_pos.z, 0.0]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

            // Create bind group
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: chunk_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: position_buffer.as_entire_binding(),
                }],
                label: Some("chunk_bind_group"),
            });

            self.set_chunk(
                chunk_coord,
                Chunk {
                    bind_group: Some(bind_group),
                    ..chunk
                },
            );

            true
        }
    }

    /// Updates loaded chunks based on player position
    pub fn update_loaded_chunks(&mut self, center: Vector3<i32>, radius: u32) {
        let center_coord = ChunkCoord::from_world_pos(center);
        let (center_x, center_y, center_z) = center_coord.unpack();
        let radius_i32 = radius as i32;
        let radius_sq = (radius * radius) as i32;

        // Unload distant chunks
        let mut to_unload = Vec::new();
        for &coord in &self.loaded_chunks {
            let (x, y, z) = coord.unpack();
            let dx = x - center_x;
            let dy = y - center_y;
            let dz = z - center_z;

            if dx * dx + dy * dy + dz * dz > radius_sq {
                to_unload.push(coord);
            }
        }

        for coord in to_unload {
            self.unload_chunk(coord);
        }

        // Load new chunks in range
        for dx in -radius_i32..=radius_i32 {
            for dy in -radius_i32..=radius_i32 {
                for dz in -radius_i32..=radius_i32 {
                    if dx * dx + dy * dy + dz * dz > radius_sq {
                        continue;
                    }

                    let coord = ChunkCoord::new(center_x + dx, center_y + dy, center_z + dz);
                    if !self.loaded_chunks.contains(&coord) {
                        self.load_chunk(coord);
                    }
                }
            }
        }
    }

    #[inline]
    pub fn set_chunk(&mut self, chunk_coord: ChunkCoord, chunk: Chunk) {
        self.chunks.insert(chunk_coord, chunk);
        self.loaded_chunks.insert(chunk_coord);
    }

    #[inline]
    pub fn unload_chunk(&mut self, chunk_coord: ChunkCoord) {
        self.chunks.remove(&chunk_coord);
        self.loaded_chunks.remove(&chunk_coord);
    }

    /// Generates meshes for all dirty chunks
    pub fn make_chunk_meshes(&mut self, device: &wgpu::Device) {
        for chunk in self.chunks.values_mut() {
            chunk.make_mesh(device, false);
        }
    }

    /// Renders all chunks with meshes
    pub fn render_chunks<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        for chunk in self.chunks.values() {
            if let (Some(mesh), Some(bind_group)) = (&chunk.mesh, &chunk.bind_group) {
                render_pass.set_bind_group(2, bind_group, &[]);
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }
        }
    }
}

/// Builder for chunk meshes
pub struct ChunkMeshBuilder {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    current_vertex: u32,
}

impl ChunkMeshBuilder {
    #[inline]
    pub fn new() -> Self {
        Self {
            vertices: Vec::with_capacity(4096),
            indices: Vec::with_capacity(4096 * 3),
            current_vertex: 0,
        }
    }

    #[inline]
    pub fn add_triangle(&mut self, vertices: &[Vector3<f32>; 3]) {
        // Calculate face normal
        let edge1 = vertices[1] - vertices[0];
        let edge2 = vertices[2] - vertices[0];
        let normal = edge1.cross(edge2).normalize();
        let normal_arr = [normal.x, normal.y, normal.z];

        let base = self.current_vertex as u16;
        for vertex in vertices {
            self.vertices.push(Vertex {
                position: [vertex.x, vertex.y, vertex.z],
                normal: normal_arr,
                uv: [0.0, 0.0], // TODO: Proper UV mapping
            });
        }
        self.indices.extend([base, base + 1, base + 2]);
        self.current_vertex += 3;
    }

    #[inline]
    pub fn build(self, device: &wgpu::Device) -> GeometryBuffer {
        GeometryBuffer::new(device, &self.indices, &self.vertices)
    }

    /// Adds a rotated cube to the mesh
    pub fn add_cube(&mut self, position: Vector3<f32>, rotation: Quaternion<f32>) {
        let transform =
            Matrix4::from_translation(position - Vector3::unit_z()) * Matrix4::from(rotation);
        let start_vertex = self.current_vertex;

        // Add transformed vertices
        for vertex in &CUBE_VERTICES {
            let pos = transform * Vector3::from(vertex.position).extend(1.0);
            let normal = rotation * Vector3::from(vertex.normal);

            self.vertices.push(Vertex {
                position: [pos.x, pos.y, pos.z],
                normal: [normal.x, normal.y, normal.z],
                uv: vertex.uv,
            });
        }

        // Add indices by creating each face
        self.add_cube_face(start_vertex, &CUBE_FACES[0]); // Front
        self.add_cube_face(start_vertex, &CUBE_FACES[1]); // Back
        self.add_cube_face(start_vertex, &CUBE_FACES[2]); // Top
        self.add_cube_face(start_vertex, &CUBE_FACES[3]); // Bottom
        self.add_cube_face(start_vertex, &CUBE_FACES[4]); // Right
        self.add_cube_face(start_vertex, &CUBE_FACES[5]); // Left

        self.current_vertex += CUBE_VERTICES.len() as u32;
    }

    /// Internal helper to add a single cube face
    fn add_cube_face(&mut self, base_vertex: u32, face_indices: &[u16; 6]) {
        let base = base_vertex as u16;
        self.indices.extend(face_indices.iter().map(|&i| base + i));
    }
}

// Cube geometry constants
const LENG: f32 = 1.0; // unit sized cube

// Cube vertices (8 corners of a unit cube with left-bottom-front at origin)
pub const CUBE_VERTICES: [Vertex; 8] = [
    Vertex {
        position: [0.0, 0.0, LENG],
        normal: [0.0, 0.0, 1.0],
        uv: [0.0, 0.0],
    }, // front-bottom-left (origin)
    Vertex {
        position: [LENG, 0.0, LENG],
        normal: [0.0, 0.0, 1.0],
        uv: [1.0, 0.0],
    }, // front-bottom-right
    Vertex {
        position: [LENG, LENG, LENG],
        normal: [0.0, 0.0, 1.0],
        uv: [1.0, 1.0],
    }, // front-top-right
    Vertex {
        position: [0.0, LENG, LENG],
        normal: [0.0, 0.0, 1.0],
        uv: [0.0, 1.0],
    }, // front-top-left
    Vertex {
        position: [0.0, 0.0, 0.0],
        normal: [0.0, 0.0, -1.0],
        uv: [1.0, 0.0],
    }, // back-bottom-left
    Vertex {
        position: [LENG, 0.0, 0.0],
        normal: [0.0, 0.0, -1.0],
        uv: [0.0, 0.0],
    }, // back-bottom-right
    Vertex {
        position: [LENG, LENG, 0.0],
        normal: [0.0, 0.0, -1.0],
        uv: [0.0, 1.0],
    }, // back-top-right
    Vertex {
        position: [0.0, LENG, 0.0],
        normal: [0.0, 0.0, -1.0],
        uv: [1.0, 1.0],
    }, // back-top-left
];

// Each face defined as 6 indices (2 triangles)
pub const CUBE_FACES: [[u16; 6]; 6] = [
    [0, 1, 2, 2, 3, 0], // Front face
    [5, 4, 7, 7, 6, 5], // Back face
    [3, 2, 6, 6, 7, 3], // Top face
    [4, 5, 1, 1, 0, 4], // Bottom face
    [1, 5, 6, 6, 2, 1], // Right face
    [4, 0, 3, 3, 7, 4], // Left face
];
