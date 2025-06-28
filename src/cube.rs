
use std::f32::consts::FRAC_PI_2;
use super::cube_math::{self,ChunkCoord};
use super::cube_render::{ChunkMeshBuilder, GeometryBuffer};
#[allow(unused_imports)]
use super::debug;
use ahash::AHasher;
use glam::{Quat, Vec3};
use std::{
    collections::{HashMap, HashSet},
    f32::consts::{PI, TAU},
    hash::BuildHasherDefault,
};
use wgpu::util::DeviceExt;

// Type aliases for better readability
type FastMap<K, V> = HashMap<K, V, BuildHasherDefault<AHasher>>;


/// Axis enumeration with positive/negative variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    Xplus,
    Xminus,
    Yplus,
    Yminus,
    Zplus,
    Zminus,
}

/// Represents all 36 possible block rotations (6 front directions × 6 up directions)
/// The enum variants follow the pattern "FrontFaceUpFace" (e.g., XplusYplus means front faces +X and up faces +Y).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BlockRotation {
    // X+ front facing (with all possible up directions)
    XplusYplus, XplusYminus, XplusZplus, XplusZminus,
    // X- front facing
    XminusYplus, XminusYminus, XminusZplus, XminusZminus,
    // Y+ front facing
    YplusXplus, YplusXminus, YplusZplus, YplusZminus,
    // Y- front facing
    YminusXplus, YminusXminus, YminusZplus, YminusZminus,
    // Z+ front facing
    ZplusXplus, ZplusXminus, ZplusYplus, ZplusYminus,
    // Z- front facing
    ZminusXplus, ZminusXminus, ZminusYplus, ZminusYminus,
}

#[allow(dead_code)]
impl BlockRotation {
    /// Converts from the old packed u8 format to the new enum
    pub fn from_packed(packed: u8) -> Self {
        // Extract the 3 axes (each 2 bits)
        let x = (packed & 0b0000_0011) >> 0;
        let y = (packed & 0b0000_1100) >> 2;
        let z = (packed & 0b0011_0000) >> 4;
        
        // Convert to the new enum (implementation depends on your exact rotation mapping)
        // This is a placeholder - you'll need to define your exact conversion logic
        match (x, y, z) {
            (0, 0, 0) => BlockRotation::XplusYplus,
            // ... fill in all 36 cases
            _ => BlockRotation::XplusYplus, // default
        }
    }

    /// Converts to the old packed u8 format (for compatibility if needed)
    pub fn to_packed(self) -> u8 {
        match self {
            BlockRotation::XplusYplus => 0b0000_0000,
            // ... fill in all 36 cases
            _ => 0,
        }
    }

    /// Converts to a byte for storage
    #[inline]
    pub fn to_byte(self) -> u8 {
        self as u8
    }
    
    /// Creates from a byte
    #[inline]
    pub fn from_byte(byte: u8) -> Option<Self> {
        if byte < 36 {
            // SAFETY: We've verified the value is within enum bounds
            Some(unsafe { std::mem::transmute(byte) })
        } else {
            None
        }
    }

    /// Converts to a quaternion
    pub fn to_quat(self) -> Quat {
        match self {
            // +X facing
            BlockRotation::XplusYplus => Quat::IDENTITY,
            BlockRotation::XplusYminus => Quat::from_rotation_x(PI),
            BlockRotation::XplusZplus => Quat::from_rotation_y(FRAC_PI_2),
            BlockRotation::XplusZminus => Quat::from_rotation_y(-FRAC_PI_2),
            
            // -X facing
            BlockRotation::XminusYplus => Quat::from_rotation_z(PI),
            BlockRotation::XminusYminus => Quat::from_rotation_x(PI) * Quat::from_rotation_z(PI),
            BlockRotation::XminusZplus => Quat::from_rotation_y(FRAC_PI_2) * Quat::from_rotation_z(PI),
            BlockRotation::XminusZminus => Quat::from_rotation_y(-FRAC_PI_2) * Quat::from_rotation_z(PI),
            
            // +Y facing
            BlockRotation::YplusXplus => Quat::from_rotation_x(-FRAC_PI_2),
            BlockRotation::YplusXminus => Quat::from_rotation_x(-FRAC_PI_2) * Quat::from_rotation_z(PI),
            BlockRotation::YplusZplus => Quat::from_rotation_x(-FRAC_PI_2) * Quat::from_rotation_y(FRAC_PI_2),
            BlockRotation::YplusZminus => Quat::from_rotation_x(-FRAC_PI_2) * Quat::from_rotation_y(-FRAC_PI_2),
            
            // -Y facing
            BlockRotation::YminusXplus => Quat::from_rotation_x(FRAC_PI_2),
            BlockRotation::YminusXminus => Quat::from_rotation_x(FRAC_PI_2) * Quat::from_rotation_z(PI),
            BlockRotation::YminusZplus => Quat::from_rotation_x(FRAC_PI_2) * Quat::from_rotation_y(FRAC_PI_2),
            BlockRotation::YminusZminus => Quat::from_rotation_x(FRAC_PI_2) * Quat::from_rotation_y(-FRAC_PI_2),
            
            // +Z facing
            BlockRotation::ZplusXplus => Quat::from_rotation_y(PI),
            BlockRotation::ZplusXminus => Quat::from_rotation_y(PI) * Quat::from_rotation_z(PI),
            BlockRotation::ZplusYplus => Quat::from_rotation_x(FRAC_PI_2) * Quat::from_rotation_y(PI),
            BlockRotation::ZplusYminus => Quat::from_rotation_x(-FRAC_PI_2) * Quat::from_rotation_y(PI),
            
            // -Z facing
            BlockRotation::ZminusXplus => Quat::IDENTITY,
            BlockRotation::ZminusXminus => Quat::from_rotation_z(PI),
            BlockRotation::ZminusYplus => Quat::from_rotation_x(FRAC_PI_2),
            BlockRotation::ZminusYminus => Quat::from_rotation_x(-FRAC_PI_2),
        }
    }
    
    /// Creates from a quaternion (finds closest matching orientation)
    pub fn from_quat(quat: Quat) -> Self {
        let forward = quat * Vec3::Z;
        let up = quat * Vec3::Y;
        
        // Determine primary axis (front face)
        let primary_axis = if forward.x.abs() >= forward.y.abs() && forward.x.abs() >= forward.z.abs() {
            if forward.x > 0.0 { Axis::Xplus } else { Axis::Xminus }
        } else if forward.y.abs() >= forward.z.abs() {
            if forward.y > 0.0 { Axis::Yplus } else { Axis::Yminus }
        } else {
            if forward.z > 0.0 { Axis::Zplus } else { Axis::Zminus }
        };
        
        // Determine secondary axis (up face), excluding the primary axis
        let secondary_axis = {
            let candidates = match primary_axis {
                Axis::Xplus | Axis::Xminus => [
                    (up.y.abs(), if up.y > 0.0 { Axis::Yplus } else { Axis::Yminus }),
                    (up.z.abs(), if up.z > 0.0 { Axis::Zplus } else { Axis::Zminus }),
                ],
                Axis::Yplus | Axis::Yminus => [
                    (up.x.abs(), if up.x > 0.0 { Axis::Xplus } else { Axis::Xminus }),
                    (up.z.abs(), if up.z > 0.0 { Axis::Zplus } else { Axis::Zminus }),
                ],
                Axis::Zplus | Axis::Zminus => [
                    (up.x.abs(), if up.x > 0.0 { Axis::Xplus } else { Axis::Xminus }),
                    (up.y.abs(), if up.y > 0.0 { Axis::Yplus } else { Axis::Yminus }),
                ],
            };
            // Select the axis with the larger component
            if candidates[0].0 >= candidates[1].0 { candidates[0].1 } else { candidates[1].1 }
        };
        
        // Combine into enum variant
        match (primary_axis, secondary_axis) {
            (Axis::Xplus, Axis::Yplus) => BlockRotation::XplusYplus,
            (Axis::Xplus, Axis::Yminus) => BlockRotation::XplusYminus,
            (Axis::Xplus, Axis::Zplus) => BlockRotation::XplusZplus,
            (Axis::Xplus, Axis::Zminus) => BlockRotation::XplusZminus,
            
            (Axis::Xminus, Axis::Yplus) => BlockRotation::XminusYplus,
            (Axis::Xminus, Axis::Yminus) => BlockRotation::XminusYminus,
            (Axis::Xminus, Axis::Zplus) => BlockRotation::XminusZplus,
            (Axis::Xminus, Axis::Zminus) => BlockRotation::XminusZminus,
            
            (Axis::Yplus, Axis::Xplus) => BlockRotation::YplusXplus,
            (Axis::Yplus, Axis::Xminus) => BlockRotation::YplusXminus,
            (Axis::Yplus, Axis::Zplus) => BlockRotation::YplusZplus,
            (Axis::Yplus, Axis::Zminus) => BlockRotation::YplusZminus,
            
            (Axis::Yminus, Axis::Xplus) => BlockRotation::YminusXplus,
            (Axis::Yminus, Axis::Xminus) => BlockRotation::YminusXminus,
            (Axis::Yminus, Axis::Zplus) => BlockRotation::YminusZplus,
            (Axis::Yminus, Axis::Zminus) => BlockRotation::YminusZminus,
            
            (Axis::Zplus, Axis::Xplus) => BlockRotation::ZplusXplus,
            (Axis::Zplus, Axis::Xminus) => BlockRotation::ZplusXminus,
            (Axis::Zplus, Axis::Yplus) => BlockRotation::ZplusYplus,
            (Axis::Zplus, Axis::Yminus) => BlockRotation::ZplusYminus,
            
            (Axis::Zminus, Axis::Xplus) => BlockRotation::ZminusXplus,
            (Axis::Zminus, Axis::Xminus) => BlockRotation::ZminusXminus,
            (Axis::Zminus, Axis::Yplus) => BlockRotation::ZminusYplus,
            (Axis::Zminus, Axis::Yminus) => BlockRotation::ZminusYminus,
            
            // These cases shouldn't happen due to our secondary axis selection
            _ => BlockRotation::XplusYplus,
        }
    }
    
    /// Rotates the block around an axis by 90° steps
    pub fn rotate(self, axis: super::cube_math::Axis, steps: u8) -> Self {
        let quat = self.to_quat();
        let rotation = match axis {
            super::cube_math::Axis::X => Quat::from_rotation_x(steps as f32 * std::f32::consts::FRAC_PI_2),
            super::cube_math::Axis::Y => Quat::from_rotation_y(steps as f32 * std::f32::consts::FRAC_PI_2),
            super::cube_math::Axis::Z => Quat::from_rotation_z(steps as f32 * std::f32::consts::FRAC_PI_2),
        };
        Self::from_quat(rotation * quat)
    }
}



/// Represents a block in the world with optimized storage
#[derive(Clone, Copy, Debug, PartialEq)]
#[repr(u8)]
pub enum Block {
    None = 0,
    Simple(u16, BlockRotation),  // material, rotation
    Marching(u16, u32),         // material, density field (27 bits - 4)
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
    pub const fn new(material: u16) -> Self {
        Self::Simple(material, BlockRotation::XplusYplus)
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
    pub fn rotate(&mut self, axis: cube_math::Axis, steps: u8) {
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
    pub fn material(&self) -> u16 {
        match self {
            Block::Simple(material, _) | Block::Marching(material, _) => *material,
            Block::None => 0,
        }
    }

    #[inline]
    pub fn set_material(&mut self, material: u16) {
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
            _ => Some(*self),
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
    fn get(&self, index: usize) -> u8 {
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

    /// Converts (x, y, z) to array index (0..4095)
    #[inline]
    pub fn xyz_to_index(x: usize, y: usize, z: usize) -> usize {
        (x << 8) | (y << 4) | z
    }

    #[inline]
    pub fn local_to_index(local_pos: Vec3) -> usize {
        debug_assert!(
            local_pos.x >= 0.0
                && local_pos.x < Self::SIZE as f32
                && local_pos.y >= 0.0
                && local_pos.y < Self::SIZE as f32
                && local_pos.z >= 0.0
                && local_pos.z < Self::SIZE as f32,
            "Local position out of bounds: {:?}",
            local_pos
        );
        let x = local_pos.x as usize;
        let y = local_pos.y as usize;
        let z = local_pos.z as usize;
        Self::xyz_to_index(x, y, z)
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

    /// Converts world position to local chunk coordinates
    #[inline]
    pub fn world_to_local_pos(world_pos: Vec3) -> Vec3 {
        Vec3::new(
            world_pos.x.rem_euclid(Self::SIZE as f32),
            world_pos.y.rem_euclid(Self::SIZE as f32),
            world_pos.z.rem_euclid(Self::SIZE as f32),
        )
    }

    /// Packs local coordinates into u16
    #[inline]
    pub fn pack_position(local_pos: Vec3) -> u16 {
        debug_assert!(
            local_pos.x < (Self::SIZE as u32) as f32
                && local_pos.y < (Self::SIZE as u32) as f32
                && local_pos.z < (Self::SIZE as u32) as f32,
            "Position out of bounds"
        );
        ((local_pos.x as u16) << 8) | ((local_pos.y as u16) << 4) | (local_pos.z as u16)
    }

    /// Unpacks position key to local coordinates
    #[inline]
    pub fn unpack_position(pos: u16) -> Vec3 {
        Vec3::new(
            (((pos >> 8) & 0xF) as u32) as f32,
            (((pos >> 4) & 0xF) as u32) as f32,
            ((pos & 0xF) as u32) as f32,
        )
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

            let local_pos = Self::unpack_position(pos as u16);
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

        let idx = Chunk::local_to_index(pos);
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

/// Represents the game world containing chunks

/// Represents the game world containing chunks
#[derive(Debug, Clone)]
pub struct World {
    pub chunks: FastMap<ChunkCoord, Chunk>,
    pub loaded_chunks: HashSet<ChunkCoord>,
}

#[allow(dead_code)]
impl World {
    /// Creates an empty world
    #[inline]
    pub fn empty() -> Self {
        Self {
            chunks: FastMap::with_capacity_and_hasher(10_000, BuildHasherDefault::<AHasher>::default()),
            loaded_chunks: HashSet::with_capacity(10_000),
        }
    }

    #[inline]
    pub fn chunk_count(&self) -> usize {
        self.chunks.len()
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
    pub fn get_block(&self, world_pos: Vec3) -> &Block {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);
        let local_pos = Chunk::world_to_local_pos(world_pos);
        let idx = Chunk::local_to_index(local_pos);

        self.chunks
            .get(&chunk_coord)
            .map(|chunk| chunk.get_block(idx))
            .unwrap_or(&Block::None)
    }

    pub fn set_block(&mut self, world_pos: Vec3, block: Block) {
        let chunk_coord = ChunkCoord::from_world_pos(world_pos);

        if !self.chunks.contains_key(&chunk_coord) {
            self.set_chunk(chunk_coord, Chunk::empty());
        }

        if let Some(chunk) = self.chunks.get_mut(&chunk_coord) {
            let local_pos = Chunk::world_to_local_pos(world_pos);
            let index = Chunk::local_to_index(local_pos);

            // Only set if the block is actually different
            if chunk.get_block(index) != &block {
                chunk.set_block(index, block);
            }
        }
    }

    /// Loads a chunk from storage
    pub fn load_chunk(&mut self, chunk_coord: ChunkCoord, force: bool) -> bool {
        let state = super::config::get_state();
        let device = &state.render_context.device;
        let chunk_bind_group_layout = &state.render_context.chunk_bind_group_layout;

        let mut chunk = Chunk::empty();
        if force {
            chunk = match Chunk::load() {
                Some(c) => c,
                None => return false,
            };
        }

        // For palette-based chunks, we need a more sophisticated comparison
        if let Some(existing_chunk) = self.get_chunk(chunk_coord) {
            // Compare palette and storage instead of individual blocks
            if existing_chunk.palette == chunk.palette && existing_chunk.storage == chunk.storage {
                return false;
            }
        }

        // Create position buffer
        let position_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Chunk Position Buffer"),
            contents: bytemuck::cast_slice(&[
                <ChunkCoord as Into<u64>>::into(chunk_coord) as u64,
                0.0 as u64,
            ]),
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

        self.chunks.insert(
            chunk_coord,
            Chunk {
                bind_group: Some(bind_group),
                ..chunk
            },
        );
        self.loaded_chunks.insert(chunk_coord);

        true
    }

    /// Updates loaded chunks based on player position
    pub fn update_loaded_chunks(&mut self, center: Vec3, radius: f32, force: bool) {
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
                    if force || !self.loaded_chunks.contains(&coord) {
                        self.load_chunk(coord, false);
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
    #[inline]
    pub fn make_chunk_meshes(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
//        let timer = std::time::Instant::now();
        //let mut chunk_times = debug::RunningAverage::default();

        for chunk in self.chunks.values_mut() {
            // Skip empty chunks entirely
            if chunk.is_empty() {
                continue;
            }

            //let chunk_timer = std::time::Instant::now();
            chunk.make_mesh(device, queue, false);
            //let elapsed_micros = chunk_timer.elapsed().as_micros() as f32;
            //chunk_times.add(elapsed_micros.into());
        }
/*
        println!(
            "World mesh_gen_time: {:.2}ms",
            timer.elapsed().as_secs_f32() * 1000.0
        );
*/
    }

    pub fn render_chunks<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        for chunk in self.chunks.values() {
            // Skip empty chunks entirely - no mesh or bind group needed
            if chunk.is_empty() {
                continue;
            }

            if let (Some(mesh), Some(bind_group)) = (&chunk.mesh, &chunk.bind_group) {
                // Skip if mesh has no indices (shouldn't happen but good to check)
                if mesh.num_indices == 0 {
                    continue;
                }

                render_pass.set_bind_group(2, bind_group, &[]);
                render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                render_pass
                    .set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                render_pass.draw_indexed(0..mesh.num_indices, 0, 0..1);
            }
        }
    }
}
