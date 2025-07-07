
use crate::block::math::{self, BlockPosition, BlockRotation};
use crate::render::meshing::{ChunkMeshBuilder, GeometryBuffer};
#[allow(unused_imports)]
use crate::stopwatch;
use glam::{Quat, Vec3};
use std::f32::consts::{PI, TAU};

type Material = u16;
type DensityField = u32;

/// Represents a block in the world with optimized storage
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum Block {
    None = 0,
    Simple(Material, BlockRotation),  // material, rotation
    Marching(Material, DensityField),         // material, density field (27 bits - 4)
}

#[allow(dead_code)]
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
        Self::Simple(0, BlockRotation::XplusYplus)
    }

    /// Creates a new simple block with default material
    #[inline]
    pub const fn new(material: Material) -> Self {
        Self::Simple(material, BlockRotation::XplusYplus)
    }
    /// Creates a new marching cubes block with no point set
    #[inline]
    pub const fn new_march(material: Material) -> Self {
        Self::Marching(material, 0)
    }

    /// Creates a block from a quaternion rotation
    #[inline]
    pub fn new_quat(rotation: Quat) -> Self {
        Self::Simple(1, BlockRotation::from_quat(rotation))
    }

    /// Creates a new marching cubes block with center point set
    #[inline]
    pub const fn new_dot() -> Self {
        Self::Marching(1, 0x20_00)
    }


    /// Extracts rotation
    #[inline]
    pub fn get_rotation(&self) -> Option<BlockRotation> {
        match self {
            Block::Simple(_, rot) => Some(*rot),
            _ => None,
        }
    }

    /// Converts packed rotation to quaternion
    #[inline]
    pub fn to_quat(&self) -> Quat {
        self.get_rotation()
            .map(|rot| rot.to_quat())
            .unwrap_or_else(|| Quat::IDENTITY)
    }

    /// Rotates the block around an axis by N 90° steps
    #[inline]
    pub fn rotate(&mut self, axis: math::AxisBasic, steps: u8) {
        if let Block::Simple(_, rotation) = self {
            *rotation = rotation.rotate(axis, steps);
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        match self {
            Block::Simple(material, _) | Block::Marching(material, _) => *material == 0,
            Block::None => true,
        }
    }

    #[inline]
    pub fn is_marching(&self) -> bool {
        matches!(self, Block::Marching { .. })
    }

    #[inline]
    pub fn texture_coords(&self) -> [f32; 2] {
        let material: f32 = self.material() as f32;
        [(material - 1.0).max(0.0), material]
    }

    #[inline]
    pub fn material(&self) -> Material {
        match self {
            Block::Simple(material, _) | Block::Marching(material, _) => *material,
            Block::None => 0,
        }
    }

    #[inline]
    pub fn set_material(&mut self, material: Material) {
        match self {
            Block::Simple(mat, _) | Block::Marching(mat, _) => *mat = material,
            Block::None => {}
        }
    }

    /// Converts quaternion to packed rotation format
    #[inline]
    pub fn quat_to_rotation(rotation: Quat) -> u8 {
        // Extract Euler angles using efficient quaternion conversion
        let (x, y, z) = rotation.to_euler(glam::EulerRot::ZYX);

        // Convert to 2-bit values (0-3) representing 90-degree increments
        let x = ((x.rem_euclid(TAU) / (PI / 2.0)).round() as u8) & 0x3;
        let y = ((y.rem_euclid(TAU) / (PI / 2.0)).round() as u8) & 0x3;
        let z = ((z.rem_euclid(TAU) / (PI / 2.0)).round() as u8) & 0x3;

        // Pack into a single byte
        x | (y << 2) | (z << 4)
    }

    /// Sets all rotation axes at once
    #[inline]
    pub fn set_rotation(&mut self, rotaio: BlockRotation) {
        if let Block::Simple(_, rotation) = self {
            *rotation = rotaio;
        }
    }

    /// Sets a point in the 3x3x3 density field
    #[inline]
    pub fn set_point(&mut self, x: u8, y: u8, z: u8, value: bool) {
        if let Block::Marching(_, points) = self {
            debug_assert!(x < 3 && y < 3 && z < 3, "Coordinates must be 0-2");
            let bit_pos = x as u32 + (y as u32) * 3 + (z as u32) * 9;
            *points = (*points & !(1 << bit_pos)) | ((value as u32) << bit_pos);
        }
    }

    /// Gets a point from the 3x3x3 density field
    #[inline]
    pub fn get_point(&self, x: u8, y: u8, z: u8) -> Option<bool> {
        match self {
            Block::Marching(_, points) => {
                debug_assert!(x < 3 && y < 3 && z < 3, "Coordinates must be 0-2");
                let bit_pos = x as u32 + (y as u32) * 3 + (z as u32) * 9;
                Some((*points & (1u32 << bit_pos)) != 0)
            }
            _ => None,
        }
    }

    pub fn get_march(&mut self) -> Option<Block> {
        match self {
            Block::Marching(_, _) => None,
            _ => Some(Self::new_march(self.material())),
        }
    }
}

/// Represents a chunk of blocks in the world
#[derive(Clone, PartialEq)]
pub struct Chunk {
    pub palette: Vec<Block>, // Max 256 entries (index 0 = air, indices 1-255 = blocks)
    pub storage: BlockStorage, // Palette indices for each block position
    pub dirty: bool,
    pub mesh: Option<GeometryBuffer>,
    pub bind_group: Option<wgpu::BindGroup>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BlockStorage {
    Uniform(u8),             // Single palette index for all blocks
    Sparse(Box<[u8; 4096]>), // Full index array
}

impl BlockStorage {
    /// Gets the palette index at the given position
    #[inline]
    pub fn get(&self, index: usize) -> u8 {
        match self {
            BlockStorage::Uniform(palette_idx) => *palette_idx,
            BlockStorage::Sparse(indices) => indices[index],
        }
    }

    /// Sets the palette index at the given position, converting to sparse if needed
    #[inline]
    fn set(&mut self, index: usize, palette_idx: u8) {
        match self {
            BlockStorage::Uniform(current_idx) => {
                if *current_idx != palette_idx {
                    // Convert to sparse storage
                    let mut indices = Box::new([*current_idx; 4096]);
                    indices[index] = palette_idx;
                    *self = BlockStorage::Sparse(indices);
                }
            }
            BlockStorage::Sparse(indices) => {
                indices[index] = palette_idx;
            }
        }
    }

    /// Attempts to optimize storage back to uniform if all indices are the same
    #[inline]
    fn try_optimize(&mut self) {
        if let BlockStorage::Sparse(indices) = self {
            let first = indices[0];
            if indices.iter().all(|&idx| idx == first) {
                *self = BlockStorage::Uniform(first);
            }
        }
    }
}

impl std::fmt::Debug for Chunk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Chunk")
            .field("dirty", &self.dirty)
            .field("is_empty", &self.is_empty())
            .field("has_bind_group", &self.bind_group.is_some())
            .field("has_mesh", &self.mesh.is_some())
            .finish()
    }
}

#[allow(dead_code)]
impl Chunk {
    pub const SIZE: usize = 16;
    pub const SIZE_I: i32 = Self::SIZE as i32;
    pub const VOLUME: usize = Self::SIZE.pow(3); // 4096
    const MAX_PALETTE_SIZE: usize = 256; // Index 0 = air, indices 1-255 = blocks

    /// Creates an empty chunk (all blocks are air)
    #[inline]
    pub fn empty() -> Self {
        Self {
            palette: vec![Block::None],        // Index 0 is always air
            storage: BlockStorage::Uniform(0), // All blocks point to air
            dirty: false,
            mesh: None,
            bind_group: None,
        }
    }

    /// Creates a new filled chunk (all blocks initialized to `Block::new()`)
    #[inline]
    pub fn new() -> Self {
        let mut chunk = Self::empty();
        let new_block = Block::new(1);
        let idx = chunk.palette_add(new_block);
        chunk.storage = BlockStorage::Uniform(idx);
        chunk.dirty = true;
        chunk
    }

    /// Adds a block to the palette, returning its index
    /// Returns existing index if block already exists
    #[inline]
    fn palette_add(&mut self, block: Block) -> u8 {
        // Air blocks always map to index 0
        if block.is_empty() {
            return 0;
        }

        // Check if block already exists in palette
        if let Some(idx) = self.palette.iter().position(|&b| b == block) {
            return idx as u8;
        }

        // Add new block to palette if there's space
        if self.palette.len() < Self::MAX_PALETTE_SIZE {
            let idx = self.palette.len();
            self.palette.push(block);
            idx as u8
        } else {
            // Palette is full, could implement LRU eviction here
            // For now, just return index 1 (first non-air block)
            eprintln!("Warning: Chunk palette is full, using fallback block");
            1
        }
    }

    /// Removes unused blocks from the palette and updates indices
    fn palette_compact(&mut self) {
        if matches!(self.storage, BlockStorage::Uniform(_)) {
            // For uniform storage, we only need the one block type
            let used_idx = match self.storage {
                BlockStorage::Uniform(idx) => idx,
                _ => unreachable!(),
            };

            if used_idx == 0 {
                // Only air is used
                self.palette = vec![Block::None];
            } else if used_idx < self.palette.len() as u8 {
                // Compact to just air + the used block
                let used_block = self.palette[used_idx as usize];
                self.palette = vec![Block::None, used_block];
                self.storage = BlockStorage::Uniform(1);
            }
            return;
        }

        // For sparse storage, find all used palette indices
        let mut used_indices = std::collections::HashSet::new();
        if let BlockStorage::Sparse(indices) = &self.storage {
            for &idx in indices.iter() {
                used_indices.insert(idx);
            }
        }

        // Create new compact palette
        let mut new_palette = Vec::new();
        let mut index_mapping = std::collections::HashMap::new();

        // Air always stays at index 0
        new_palette.push(Block::None);
        index_mapping.insert(0u8, 0u8);

        // Add used blocks in order
        for old_idx in 1..self.palette.len() as u8 {
            if used_indices.contains(&old_idx) {
                let new_idx = new_palette.len() as u8;
                new_palette.push(self.palette[old_idx as usize]);
                index_mapping.insert(old_idx, new_idx);
            }
        }

        // Update storage with new indices
        if let BlockStorage::Sparse(indices) = &mut self.storage {
            for idx in indices.iter_mut() {
                *idx = index_mapping[idx];
            }
        }

        self.palette = new_palette;
        self.storage.try_optimize();
    }

    #[inline]
    pub fn load() -> Option<Self> {
        Some(Self::new())
    }

    pub fn world_to_local_pos(world_pos: Vec3) -> BlockPosition {
        let x = (world_pos.x.floor() as i32).rem_euclid(Self::SIZE_I) as u8;
        let y = (world_pos.y.floor() as i32).rem_euclid(Self::SIZE_I) as u8;
        let z = (world_pos.z.floor() as i32).rem_euclid(Self::SIZE_I) as u8;
        BlockPosition::new(x, y, z)
    }

    #[inline]
    pub fn get_block(&self, index: usize) -> &Block {
        let palette_idx = self.storage.get(index);
        &self.palette[palette_idx as usize]
    }

    #[inline]
    pub fn get_block_mut(&mut self, index: usize) -> &mut Block {
        let palette_idx = self.storage.get(index);
        &mut self.palette[palette_idx as usize]
    }

    /// Checks if the chunk is completely empty (all blocks are air)
    #[inline]
    pub fn is_empty(&self) -> bool {
        match &self.storage {
            BlockStorage::Uniform(idx) => *idx == 0, // Index 0 is air
            BlockStorage::Sparse(indices) => indices.iter().all(|&idx| idx == 0),
        }
    }

    /// Sets a block at the given index
    pub fn set_block(&mut self, index: usize, block: Block) {
        let palette_idx = self.palette_add(block);
        self.storage.set(index, palette_idx);
        self.dirty = true;

        // Periodically compact the palette to avoid bloat
        if self.palette.len() > 64 {
            self.palette_compact();
        }
    }

    pub fn make_mesh(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, force: bool) {
        if !force && !self.dirty && self.mesh.is_some() {
            return;
        }

        // Early return if chunk is empty
        if self.is_empty() {
            if self.mesh.is_some() {
                self.mesh = Some(GeometryBuffer::empty(device));
                self.dirty = false;
            }
            return;
        }

        let mut builder = ChunkMeshBuilder::new();

        for pos in 0..Self::VOLUME {
            let block = *self.get_block(pos);
            if block.is_empty() {
                continue;
            }

            let local_pos = BlockPosition::from(pos).into();
            match block {
                Block::Marching(_, points) => {
                    builder.add_marching_cube(points, local_pos);
                }
                _ => {
                    builder.add_cube(local_pos, block.to_quat(), block.texture_coords(), self);
                }
            }
        }

        if let Some(mesh) = &mut self.mesh {
            mesh.update(device, queue, &builder.indices, &builder.vertices);
        } else {
            self.mesh = Some(GeometryBuffer::new(
                device,
                &builder.indices,
                &builder.vertices,
            ));
        }
        self.dirty = false;
    }

    /// Checks if a block position is empty or outside the chunk
    #[inline]
    pub fn is_block_cull(&self, pos: Vec3) -> bool {
        // Check if position is outside chunk bounds
        if pos.x < 0.0
            || pos.y < 0.0
            || pos.z < 0.0
            || pos.x >= Self::SIZE_I as f32
            || pos.y >= Self::SIZE_I as f32
            || pos.z >= Self::SIZE_I as f32
        {
            return true;
        }

        let idx:usize = BlockPosition::from(pos).into();
        let block = *self.get_block(idx);
        block.is_empty() || block.is_marching()
    }

    /// Returns a reference to the mesh if it exists
    #[inline]
    pub fn mesh(&self) -> Option<&GeometryBuffer> {
        self.mesh.as_ref()
    }

    /// Returns a reference to the bind group if it exists
    #[inline]
    pub fn bind_group(&self) -> Option<&wgpu::BindGroup> {
        self.bind_group.as_ref()
    }
}
