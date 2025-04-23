use crate::geometry::GeometryBuffer;
use crate::geometry::Vertex;
use crate::geometry::EDGE_TABLE;
use crate::geometry::TRI_TABLE;
use crate::traits::VectorTypeConversion;
use ahash::AHasher;
use cgmath::InnerSpace;
use cgmath::Matrix4;
use cgmath::VectorSpace;
use cgmath::Zero;
use cgmath::{Deg, Quaternion, Rotation3, Vector3};
use std::collections::{HashMap, HashSet};
use std::hash::BuildHasherDefault;
use wgpu::util::DeviceExt;

type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;

/// Stores rotations for X, Y, Z as 2-bit fields: [X:2, Y:2, Z:2, Empty:2]
/// Stores 3x3x3 points as a 32-bit "array" [Points: 27, Empty: 5]
#[derive(Clone, Copy, Debug)]
pub enum Block {
    // Simple block with just material and rotation
    Simple {
        material: u16,
        rotation: u8, // [X:2, Y:2, Z:2, Empty:2]
    },
    // Marching cubes block with material and point data
    Marching {
        material: u16,
        points: u32, // 3x3x3 points (27 bits used)
    },
}

impl Block {
    pub const ROT_MASK_X: u8 = 0b11;
    pub const ROT_SHIFT_X: u32 = 0;
    pub const ROT_MASK_Y: u8 = 0b11 << 2;
    pub const ROT_SHIFT_Y: u32 = 2;
    pub const ROT_MASK_Z: u8 = 0b11 << 4;
    pub const ROT_SHIFT_Z: u32 = 4;

    #[inline]
    pub fn default() -> Self {
        Self::Simple {
            material: 0,
            rotation: 0,
        }
    }

    #[inline]
    pub fn new() -> Self {
        Self::Simple {
            material: 1,
            rotation: 0,
        }
    }

    #[inline]
    pub fn new_dot() -> Self {
        Self::Marching {
            material: 1,
            points: 0x20_00,
        }
    }

    #[inline]
    pub fn new_rot(rotation: u8) -> Self {
        Self::Simple {
            material: 1,
            rotation,
        }
    }

    pub fn new_rot_raw(rotation: Quaternion<f32>) -> Self {
        Self::Simple {
            material: 1,
            rotation: quaternion_to_rotation(rotation),
        }
    }

    /// Extract individual rotation components (0-3)
    #[inline]
    pub fn get_x_rotation(&self) -> Option<u8> {
        match self {
            Block::Simple { rotation, .. } => {
                Some((rotation & Self::ROT_MASK_X) >> Self::ROT_SHIFT_X)
            }
            _ => None,
        }
    }

    #[inline]
    pub fn get_y_rotation(&self) -> Option<u8> {
        match self {
            Block::Simple { rotation, .. } => {
                Some((rotation & Self::ROT_MASK_Y) >> Self::ROT_SHIFT_Y)
            }
            _ => None,
        }
    }

    #[inline]
    pub fn get_z_rotation(&self) -> Option<u8> {
        match self {
            Block::Simple { rotation, .. } => {
                Some((rotation & Self::ROT_MASK_Z) >> Self::ROT_SHIFT_Z)
            }
            _ => None,
        }
    }

    /// Rotation snapping and conversion to quaternion
    pub fn rotation_to_quaternion(&self) -> Quaternion<f32> {
        let angles = [
            self.get_x_rotation().unwrap(),
            self.get_y_rotation().unwrap(),
            self.get_z_rotation().unwrap(),
        ]
        .map(|r| Deg(r as f32 * 90.0)); // 4 possible rotations (0°, 90°, 180°, 270°)

        Quaternion::from_angle_z(angles[2])
            * Quaternion::from_angle_y(angles[1])
            * Quaternion::from_angle_x(angles[0])
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

    pub fn rotate(&mut self, axis: char, steps: u8) {
        if let Block::Simple { rotation, .. } = self {
            let (current, mask, shift) = match axis {
                'x' => (
                    (*rotation & Self::ROT_MASK_X) >> Self::ROT_SHIFT_X,
                    Self::ROT_MASK_X,
                    Self::ROT_SHIFT_X,
                ),
                'y' => (
                    (*rotation & Self::ROT_MASK_Y) >> Self::ROT_SHIFT_Y,
                    Self::ROT_MASK_Y,
                    Self::ROT_SHIFT_Y,
                ),
                'z' => (
                    (*rotation & Self::ROT_MASK_Z) >> Self::ROT_SHIFT_Z,
                    Self::ROT_MASK_Z,
                    Self::ROT_SHIFT_Z,
                ),
                _ => unreachable!(),
            };

            let new_rot = (current + steps) % 4; // Only 4 possible rotations now (0-3)
            *rotation = (*rotation & !mask) | (new_rot << shift);
        }
    }

    pub fn set_rotation(&mut self, x: u8, y: u8, z: u8) {
        if let Block::Simple { rotation, .. } = self {
            *rotation = (x & 0x3) | ((y & 0x3) << 2) | ((z & 0x3) << 4);
        }
    }

    /// Set a specific point in the 3x3x3 grid (x,y,z in 0..=2)
    #[inline]
    pub fn set_point(&mut self, (x, y, z, value): (u8, u8, u8, bool)) {
        if let Block::Marching { points, .. } = self {
            assert!(x < 3 && y < 3 && z < 3, "Coordinates must be in 0..=2");
            let bit_pos = (x as u32) + (y as u32) * 3 + (z as u32) * 9;
            *points = if value {
                *points | (1 << bit_pos)
            } else {
                *points & !(1 << bit_pos)
            };
        }
    }

    /// Get a specific point in the 3x3x3 grid (x,y,z in 0..=2)
    #[inline]
    pub fn get_point(&self, x: u8, y: u8, z: u8) -> Option<bool> {
        if let Block::Marching { points, .. } = self {
            assert!(x < 3 && y < 3 && z < 3, "Coordinates must be in 0..=2");
            let bit_pos = (x as u32) + (y as u32) * 3 + (z as u32) * 9;
            Some((*points & (1 << bit_pos)) != 0)
        } else {
            None
        }
    }

    /// Precomputed corner positions for marching cubes
    const CORNER_POSITIONS: [Vector3<f32>; 8] = [
        Vector3::new(0.0, 0.0, 0.0), // 0
        Vector3::new(0.5, 0.0, 0.0), // 1
        Vector3::new(0.5, 0.0, 0.5), // 2
        Vector3::new(0.0, 0.0, 0.5), // 3
        Vector3::new(0.0, 0.5, 0.0), // 4
        Vector3::new(0.5, 0.5, 0.0), // 5
        Vector3::new(0.5, 0.5, 0.5), // 6
        Vector3::new(0.0, 0.5, 0.5), // 7
    ];

    /// Optimized marching cubes mesh generation
    pub fn generate_marching_cubes_mesh(
        &self,
        position: Vector3<u32>,
        builder: &mut ChunkMeshBuilder,
    ) {
        let Block::Marching { points, .. } = self else {
            return;
        };

        let base_pos = position.to_vec3_f32() - Vector3::new(0.0, 0.0, 1.0);
        let mut edge_vertices = [Vector3::zero(); 12];

        // Process each sub-cube in the 3x3x3 grid
        for sub_z in 0..2 {
            for sub_y in 0..2 {
                for sub_x in 0..2 {
                    // Calculate case index directly from bit positions
                    let case_index = ((points >> (sub_x + sub_y * 3 + sub_z * 9)) & 1)
                        | ((points >> (sub_x + 1 + sub_y * 3 + sub_z * 9)) & 1) << 1
                        | ((points >> (sub_x + 1 + sub_y * 3 + (sub_z + 1) * 9)) & 1) << 2
                        | ((points >> (sub_x + sub_y * 3 + (sub_z + 1) * 9)) & 1) << 3
                        | ((points >> (sub_x + (sub_y + 1) * 3 + sub_z * 9)) & 1) << 4
                        | ((points >> (sub_x + 1 + (sub_y + 1) * 3 + sub_z * 9)) & 1) << 5
                        | ((points >> (sub_x + 1 + (sub_y + 1) * 3 + (sub_z + 1) * 9)) & 1) << 6
                        | ((points >> (sub_x + (sub_y + 1) * 3 + (sub_z + 1) * 9)) & 1) << 7;

                    // Skip empty or full sub-cubes
                    if case_index == 0 || case_index == 255 {
                        continue;
                    }

                    let edges = EDGE_TABLE[case_index as usize];
                    if edges == 0 {
                        continue;
                    }

                    // Calculate edge vertices only when needed
                    for edge in 0..12 {
                        if (edges & (1 << edge)) != 0 {
                            let (a, b) = match edge {
                                0 => (0, 1),
                                1 => (1, 2),
                                2 => (2, 3),
                                3 => (3, 0),
                                4 => (4, 5),
                                5 => (5, 6),
                                6 => (6, 7),
                                7 => (7, 4),
                                8 => (0, 4),
                                9 => (1, 5),
                                10 => (2, 6),
                                11 => (3, 7),
                                _ => unreachable!(),
                            };

                            edge_vertices[edge] =
                                Self::CORNER_POSITIONS[a].lerp(Self::CORNER_POSITIONS[b], 0.5);
                        }
                    }

                    // Generate triangles
                    let triangles = &TRI_TABLE[case_index as usize];
                    let sub_offset =
                        Vector3::new(sub_x as f32 * 0.5, sub_y as f32 * 0.5, sub_z as f32 * 0.5);

                    for i in (0..16).step_by(3) {
                        if triangles[i] == -1 {
                            break;
                        }

                        let indices = [
                            triangles[i] as usize,
                            triangles[i + 1] as usize,
                            triangles[i + 2] as usize,
                        ];

                        let vertices =
                            indices.map(|idx| base_pos + sub_offset + edge_vertices[idx]);

                        builder.add_triangle(&vertices);
                    }
                }
            }
        }
    }
}

/// Convert a quaternion to the packed u8 rotation format (2 bits per axis)
pub fn quaternion_to_rotation(rotation: Quaternion<f32>) -> u8 {
    // Convert quaternion to Euler angles (simplified)
    let angles = [
        (2.0 * (rotation.s * rotation.v.x + rotation.v.y * rotation.v.z))
            .atan2(1.0 - 2.0 * (rotation.v.x.powi(2) + rotation.v.y.powi(2))),
        (2.0 * (rotation.s * rotation.v.y - rotation.v.z * rotation.v.x)).asin(),
        (2.0 * (rotation.s * rotation.v.z + rotation.v.x * rotation.v.y))
            .atan2(1.0 - 2.0 * (rotation.v.y.powi(2) + rotation.v.z.powi(2))),
    ];

    // Snap to nearest 90° increment (0-3 for each axis)
    let bits: [u8; 3] = angles.map(|a| {
        let normalized =
            (a.rem_euclid(std::f32::consts::TAU) / std::f32::consts::FRAC_PI_2).round() as u8 % 4;
        normalized & 0x3 // Ensure we only use 2 bits
    });

    bits[0] | (bits[1] << 2) | (bits[2] << 4)
}

// New u64-based chunk coordinate representation
// Format: [X:26 (signed), Y:12 (signed), Z:26 (signed)]
pub type ChunkCoord = u64;

#[allow(dead_code, unused)]
pub trait ChunkCoordHelp {
    const X_MASK: u64 = 0x03FFFFFF; // 26 bits
    const X_SHIFT: u32 = 38;
    const Y_MASK: u64 = 0x0FFF; // 12 bits
    const Y_SHIFT: u32 = 26;
    const Z_MASK: u64 = 0x03FFFFFF; // 26 bits
    const Z_SHIFT: u32 = 0;

    fn new(x: i32, y: i32, z: i32) -> Self;

    fn pack(x: i32, y: i32, z: i32) -> Self;

    fn unpack(coord: &Self) -> (i32, i32, i32);

    fn extract_x(coord: &Self) -> i32;

    fn extract_y(coord: &Self) -> i32;

    fn extract_z(coord: &Self) -> i32;

    fn to_world_pos(&self) -> Vector3<i32>;

    fn from_world_pos(world_pos: Vector3<i32>) -> Self;
}

impl ChunkCoordHelp for ChunkCoord {
    #[inline]
    fn new(x: i32, y: i32, z: i32) -> Self {
        Self::pack(x, y, z)
    }

    #[inline]
    fn pack(x: i32, y: i32, z: i32) -> Self {
        debug_assert!(
            x >= -(1 << 25) && x < (1 << 25),
            "X coordinate out of range"
        );
        debug_assert!(
            y >= -(1 << 11) && y < (1 << 11),
            "Y coordinate out of range"
        );
        debug_assert!(
            z >= -(1 << 25) && z < (1 << 25),
            "Z coordinate out of range"
        );

        ((x as u64 & Self::X_MASK) << Self::X_SHIFT)
            | ((y as u64 & Self::Y_MASK) << Self::Y_SHIFT)
            | (z as u64 & Self::Z_MASK)
    }

    #[inline]
    fn unpack(coord: &Self) -> (i32, i32, i32) {
        (
            Self::extract_x(coord),
            Self::extract_y(coord),
            Self::extract_z(coord),
        )
    }

    #[inline]
    fn extract_x(coord: &Self) -> i32 {
        // Sign extend the 26-bit value
        (((*coord >> Self::X_SHIFT) as i32) << 6) >> 6
    }

    #[inline]
    fn extract_y(coord: &Self) -> i32 {
        // Sign extend the 12-bit value
        (((*coord >> Self::Y_SHIFT) as i32 & Self::Y_MASK as i32) << 20) >> 20
    }

    #[inline]
    fn extract_z(coord: &Self) -> i32 {
        // Sign extend the 26-bit value
        ((*coord as i32 & Self::Z_MASK as i32) << 6) >> 6
    }

    #[inline]
    fn to_world_pos(&self) -> Vector3<i32> {
        let (x, y, z) = Self::unpack(self);
        Vector3::new(
            x * Chunk::CHUNK_SIZE_I,
            y * Chunk::CHUNK_SIZE_I,
            z * Chunk::CHUNK_SIZE_I,
        )
    }

    #[inline]
    fn from_world_pos(world_pos: Vector3<i32>) -> Self {
        let chunk_size = Chunk::CHUNK_SIZE_I;
        Self::new(
            world_pos.x.div_euclid(chunk_size),
            world_pos.y.div_euclid(chunk_size),
            world_pos.z.div_euclid(chunk_size),
        )
    }
}

#[derive(Clone, Debug)]
pub struct Chunk {
    pub blocks: FastMap<u16, Block>, // Key is packed position (x,y,z)
    pub dirty: bool,                 // For mesh regeneration
    pub mesh: Option<super::geometry::GeometryBuffer>,
    bind_group: Option<wgpu::BindGroup>,
}

impl Chunk {
    pub const CHUNK_SIZE: usize = 16;
    pub const CHUNK_SIZE_I: i32 = Self::CHUNK_SIZE as i32;

    /// Creates a new empty chunk
    pub fn empty() -> Self {
        Self {
            blocks: FastMap::with_capacity_and_hasher(
                Self::CHUNK_SIZE.pow(3),
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
        let capacity = Self::CHUNK_SIZE.pow(3);
        chunk.blocks.reserve(capacity);

        for x in 0..Self::CHUNK_SIZE {
            for y in 0..Self::CHUNK_SIZE {
                for z in 0..Self::CHUNK_SIZE {
                    let pos = ((x as u16) << 8) | ((y as u16) << 4) | z as u16;
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

    pub fn get_block(&self, local_pos: Vector3<u32>) -> Option<&Block> {
        let pos = Self::local_to_block_pos(local_pos);
        self.blocks.get(&pos)
    }

    pub fn get_block_mut(&mut self, local_pos: Vector3<u32>) -> Option<&mut Block> {
        let pos = Self::local_to_block_pos(local_pos);
        self.blocks.get_mut(&pos)
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

    /// Convert world position to local chunk coordinates
    #[inline]
    pub fn world_to_local_pos(world_pos: Vector3<i32>) -> Vector3<u32> {
        Vector3::new(
            world_pos.x.rem_euclid(Self::CHUNK_SIZE_I) as u32,
            world_pos.y.rem_euclid(Self::CHUNK_SIZE_I) as u32,
            world_pos.z.rem_euclid(Self::CHUNK_SIZE_I) as u32,
        )
    }

    /// Convert local coordinates to packed position key
    #[inline]
    pub fn local_to_block_pos(local_pos: Vector3<u32>) -> u16 {
        ((local_pos.x as u16) << 8) | ((local_pos.y as u16) << 4) | (local_pos.z as u16)
    }

    /// Convert packed position key to local coordinates
    #[inline]
    pub fn position_to_local(pos: u16) -> Vector3<u32> {
        Vector3::new(
            ((pos >> 8) & 0xF) as u32,
            ((pos >> 4) & 0xF) as u32,
            (pos & 0xF) as u32,
        )
    }

    pub fn make_mesh(&mut self, device: &wgpu::Device, force: bool) {
        if !force || (!self.dirty && self.mesh.is_some()) {
            return;
        }

        let mut builder = ChunkMeshBuilder::new();

        for (&pos, block) in &self.blocks {
            if block.is_empty() {
                continue;
            }

            let local_pos = Self::position_to_local(pos);
            if block.is_marching() {
                block.generate_marching_cubes_mesh(local_pos, &mut builder);
                continue;
            }

            builder.add_cube(local_pos.to_vec3_f32(), block.rotation_to_quaternion());
        }

        self.mesh = Some(builder.build(device));
        self.dirty = false;
    }
}

#[derive(Debug, Clone)]
pub struct World {
    pub chunks: FastMap<ChunkCoord, Chunk>,
    pub loaded_chunks: HashSet<ChunkCoord>,
}

impl World {
    /// Create an empty world with no chunks
    #[inline]
    pub fn empty() -> Self {
        Self {
            chunks: FastMap::with_capacity_and_hasher(
                10000,
                BuildHasherDefault::<AHasher>::default(),
            ),
            loaded_chunks: HashSet::with_capacity(10000),
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

    pub fn get_block(&self, world_pos: Vector3<i32>) -> Option<&Block> {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);
        self.chunks.get(&chunk_coord).and_then(|chunk| {
            let local = Chunk::world_to_local_pos(world_pos);
            let pos = Chunk::local_to_block_pos(local);
            chunk.blocks.get(&pos)
        })
    }

    pub fn get_block_mut(&mut self, world_pos: Vector3<i32>) -> Option<&mut Block> {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);
        self.chunks.get_mut(&chunk_coord).and_then(|chunk| {
            let local = Chunk::world_to_local_pos(world_pos);
            let pos = Chunk::local_to_block_pos(local);
            chunk.blocks.get_mut(&pos)
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

    #[inline]
    pub fn load_chunk(&mut self, chunk_coord: ChunkCoord) -> bool {
        unsafe {
            let state = super::get_state();
            let device = &state.render_context.device;
            let chunk_bind_group_layout = &state.render_context.chunk_bind_group_layout;

            // Unload existing chunk if present
            if self.get_chunk(chunk_coord).is_some() {
                super::get_state()
                    .data_system
                    .world
                    .unload_chunk(chunk_coord);
            }

            // Attempt to load chunk
            let chunk = match Chunk::load() {
                Some(c) => c,
                None => return false,
            };

            // Create position buffer (only storing Vector3<f32> now)
            let world_pos = chunk_coord.to_world_pos().to_vec3_f32();
            let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Chunk Position Buffer"),
                contents: bytemuck::cast_slice(&[world_pos.x, world_pos.y, world_pos.z, 0.0]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

            // Create bind group
            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &chunk_bind_group_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: position_buffer.as_entire_binding(),
                }],
                label: Some("chunk_bind_group"),
            });

            // Update chunk with new GPU resources
            let updated_chunk = Chunk {
                bind_group: Some(bind_group),
                ..chunk
            };

            self.set_chunk(chunk_coord, updated_chunk);
            true
        }
    }

    pub fn update_loaded_chunks(&mut self, center: Vector3<i32>, radius: u32) {
        let chunk_pos = ChunkCoord::from_world_pos(center);
        let (center_x, center_y, center_z) = ChunkCoord::unpack(&chunk_pos);
        let radius_i32 = radius as i32;
        let radius_sq = (radius * radius) as i32;

        // Track chunks we want to unload
        let mut chunks_to_unload = Vec::new();

        // Use loaded_chunks for faster iteration
        for &coord in &self.loaded_chunks {
            let (x, y, z) = ChunkCoord::unpack(&coord);
            let dx = x - center_x;
            let dy = y - center_y;
            let dz = z - center_z;

            if dx * dx + dy * dy + dz * dz > radius_sq {
                chunks_to_unload.push(coord);
            }
        }

        // Unload chunks
        for coord in chunks_to_unload {
            self.chunks.remove(&coord);
            self.loaded_chunks.remove(&coord);
        }

        // Load new chunks in range
        for dx in -radius_i32..=radius_i32 {
            for dy in -radius_i32..=radius_i32 {
                for dz in -radius_i32..=radius_i32 {
                    if dx * dx + dy * dy + dz * dz > radius_sq {
                        continue;
                    }
                    let x = center_x + dx;
                    let y = center_y + dy;
                    let z = center_z + dz;
                    let chunk_coord = ChunkCoord::new(x, y, z);
                    if !self.loaded_chunks.contains(&chunk_coord) {
                        self.load_chunk(chunk_coord);
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

    pub fn make_chunk_meshes(&mut self, device: &wgpu::Device) {
        for (&_coord, chunk) in self.chunks.iter_mut() {
            chunk.make_mesh(device, false);
        }
    }

    pub fn render_chunks<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        for (&_coord, chunk) in self.chunks.iter() {
            if let Some(mesh) = &chunk.mesh {
                if let Some(bind_group) = &chunk.bind_group {
                    render_pass.set_bind_group(2, bind_group, &[]);

                    render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    render_pass
                        .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
                }
            }
        }
    }
}

// --- Chunk Mesh Builder ---
pub struct ChunkMeshBuilder {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    current_vertex: u32,
}

impl ChunkMeshBuilder {
    #[inline]
    pub fn new() -> Self {
        Self {
            vertices: Vec::with_capacity(4096 * 8),
            indices: Vec::with_capacity(4096 * 36),
            current_vertex: 0,
        }
    }

    #[inline]
    pub fn add_triangle(&mut self, vertices: &[Vector3<f32>; 3]) {
        let normal = (vertices[1] - vertices[0])
            .cross(vertices[2] - vertices[0])
            .normalize();

        let base = self.current_vertex as u16;
        for vertex in vertices {
            self.vertices.push(Vertex {
                position: [vertex.x, vertex.y, vertex.z],
                normal: [normal.x, normal.y, normal.z],
                uv: [0.0, 0.0],
            });
        }
        self.indices.extend(&[base, base + 1, base + 2]);
        self.current_vertex += 3;
    }

    #[inline]
    pub fn build(self, device: &wgpu::Device) -> GeometryBuffer {
        GeometryBuffer::new(device, &self.indices, &self.vertices)
    }

    pub fn add_cube(&mut self, position: Vector3<f32>, rotation: Quaternion<f32>) {
        // Transform matrix
        let transform = Matrix4::from_translation(position) * Matrix4::from(rotation);
        let start_vertex = self.current_vertex;

        // Add transformed vertices
        for vertex in &VERTICES {
            let pos = transform * Vector3::from(vertex.position).extend(1.0);
            let normal = rotation * Vector3::from(vertex.normal);

            self.vertices.push(Vertex {
                position: [pos.x, pos.y, pos.z],
                normal: [normal.x, normal.y, normal.z],
                uv: vertex.uv,
            });
        }
        // Add indices with offset
        self.indices
            .extend(INDICES.iter().map(|&i| start_vertex as u16 + i));
        self.current_vertex += VERTICES.len() as u32;
    }

    #[inline]
    pub fn add_cube_bad(&mut self, position: Vector3<f32>, rotation: Quaternion<f32>) {
        for (normal, corners) in &CUBE_FACES {
            self.add_face(position - Vector3::unit_z(), rotation, *corners, *normal);
        }
    }
    pub fn add_face(
        &mut self,
        position: Vector3<f32>,
        rotation: Quaternion<f32>,
        corners: [Vector3<f32>; 4],
        normal: Vector3<f32>,
    ) {
        let normal = rotation * normal;
        let base = self.current_vertex as u16;

        // Add vertices with proper UV coordinates
        let uvs = [
            [0.0, 0.0], // bottom-left
            [1.0, 0.0], // bottom-right
            [1.0, 1.0], // top-right
            [0.0, 1.0], // top-left
        ];

        for (i, corner) in corners.iter().enumerate() {
            let pos = position + rotation * corner;
            self.vertices.push(Vertex {
                position: [pos.x, pos.y, pos.z],
                normal: [normal.x, normal.y, normal.z],
                uv: uvs[i],
            });
        }

        // Add indices for two triangles (quad)
        self.indices
            .extend(&[base, base + 1, base + 2, base, base + 2, base + 3]);
        self.current_vertex += 4;
    }
}

const CUBE_FACES: [(Vector3<f32>, [Vector3<f32>; 4]); 6] = [
    // Front face (normal +z)
    (
        Vector3::new(0.0, 0.0, 1.0),
        [
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(1.0, 0.0, 1.0),
            Vector3::new(1.0, 1.0, 1.0),
            Vector3::new(0.0, 1.0, 1.0),
        ],
    ),
    // Back face (normal -z)
    (
        Vector3::new(0.0, 0.0, -1.0),
        [
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(1.0, 1.0, 0.0),
        ],
    ),
    // Top face (normal +y)
    (
        Vector3::new(0.0, 1.0, 0.0),
        [
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(1.0, 1.0, 0.0),
            Vector3::new(1.0, 1.0, 1.0),
            Vector3::new(0.0, 1.0, 1.0),
        ],
    ),
    // Bottom face (normal -y)
    (
        Vector3::new(0.0, -1.0, 0.0),
        [
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 1.0),
            Vector3::new(0.0, 0.0, 1.0),
        ],
    ),
    // Right face (normal +x)
    (
        Vector3::new(1.0, 0.0, 0.0),
        [
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 1.0),
            Vector3::new(1.0, 1.0, 1.0),
            Vector3::new(1.0, 1.0, 0.0),
        ],
    ),
    // Left face (normal -x)
    (
        Vector3::new(-1.0, 0.0, 0.0),
        [
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, 1.0, 1.0),
        ],
    ),
];

// bad reason : not correct uv / normals
// not all sides display the correct texture ...
pub const VERTICES: [Vertex; 8] = [
    Vertex {
        position: [0.0, 0.0, 0.0],
        normal: [0.0, 0.0, 1.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.0, 1.0, 0.0],
        normal: [0.0, 0.0, 1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0, 0.0],
        normal: [0.0, 0.0, 1.0],
        uv: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, 0.0, 0.0],
        normal: [0.0, 0.0, 1.0],
        uv: [1.0, 0.0],
    },
    Vertex {
        position: [0.0, 0.0, -1.0],
        normal: [0.0, 0.0, -1.0],
        uv: [0.0, 0.0],
    },
    Vertex {
        position: [0.0, 1.0, -1.0],
        normal: [0.0, 0.0, -1.0],
        uv: [0.0, 1.0],
    },
    Vertex {
        position: [1.0, 1.0, -1.0],
        normal: [0.0, 0.0, -1.0],
        uv: [1.0, 1.0],
    },
    Vertex {
        position: [1.0, 0.0, -1.0],
        normal: [0.0, 0.0, -1.0],
        uv: [1.0, 0.0],
    },
];

pub const INDICES: [u16; 36] = [
    1, 0, 2, 3, 2, 0, // Front face (z=0)
    4, 5, 6, 6, 7, 4, // Back face (z=-1)
    0, 4, 7, 3, 0, 7, // Bottom (y=0)
    5, 1, 6, 1, 2, 6, // Top (y=1)
    6, 2, 7, 2, 3, 7, // Right (x=1)
    4, 0, 5, 0, 1, 5, // Left (x=0)
];
