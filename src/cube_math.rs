use super::cube::Chunk;
use crate::cube::Block;
use crate::cube::BlockStorage;
use glam::Vec3;

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
impl Into<u64> for ChunkCoord {
    fn into(self) -> u64 {
        self.0 // Access the inner u64 value
    }
}
#[allow(dead_code)]
impl ChunkCoord {
    // Use bit shifts that are powers of 2 for better optimization
    const Z_SHIFT: u8 = 0;
    const Y_SHIFT: u8 = 26;
    const X_SHIFT: u8 = 38;

    // Masks should match the shift counts
    const Z_MASK: u64 = (1 << 26) - 1;
    const Y_MASK: u64 = (1 << 12) - 1;
    const X_MASK: u64 = (1 << 26) - 1;

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
    pub fn to_world_pos(self) -> Vec3 {
        let chunk_size = Chunk::SIZE_I;
        Vec3::new(
            (self.x() * chunk_size) as f32,
            (self.y() * chunk_size) as f32,
            (self.z() * chunk_size) as f32,
        )
    }

    /// Creates from world position
    #[inline]
    pub fn from_world_pos(world_pos: Vec3) -> Self {
        let chunk_size = Chunk::SIZE_I;
        Self::new(
            world_pos.x.div_euclid(chunk_size as f32) as i32,
            world_pos.y.div_euclid(chunk_size as f32) as i32,
            world_pos.z.div_euclid(chunk_size as f32) as i32,
        )
    }
}

impl Chunk {
    /// Serializes the chunk into a compact byte representation
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(1024); // Initial reasonable capacity

        // 1. Write palette (variable length)
        output.push(self.palette.len() as u8); // Palette size (1 byte)
        for block in &self.palette {
            match block {
                Block::None => {
                    output.push(0); // Type marker for None
                }
                Block::Simple(material, rotation) => {
                    output.push(1); // Type marker for Simple
                    output.extend_from_slice(&material.to_le_bytes());
                    output.push(*rotation);
                }
                Block::Marching(material, points) => {
                    output.push(2); // Type marker for Marching
                    output.extend_from_slice(&material.to_le_bytes());
                    output.extend_from_slice(&points.to_le_bytes());
                }
            }
        }

        // 2. Write storage type (1 byte)
        match &self.storage {
            BlockStorage::Uniform(idx) => {
                output.push(0); // Storage type marker for Uniform
                output.push(*idx);
            }
            BlockStorage::Sparse(indices) => {
                output.push(1); // Storage type marker for Sparse

                // Use RLE (Run-Length Encoding) for sparse data since there are often runs of air
                let mut rle_count = 1u8;
                let mut current = indices[0];

                for &idx in indices.iter().skip(1) {
                    if idx == current && rle_count < 255 {
                        rle_count += 1;
                    } else {
                        output.push(current);
                        output.push(rle_count);
                        current = idx;
                        rle_count = 1;
                    }
                }
                // Write the last run
                output.push(current);
                output.push(rle_count);
            }
        }

        output
    }

    /// Deserializes a chunk from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        let mut cursor = 0;

        // 1. Read palette
        let palette_size = bytes.get(cursor).ok_or("Missing palette size")?;
        cursor += 1;

        let mut palette = Vec::with_capacity(*palette_size as usize);
        for _ in 0..*palette_size {
            let block_type = bytes.get(cursor).ok_or("Missing block type")?;
            cursor += 1;

            let block = match block_type {
                0 => Block::None,
                1 => {
                    if cursor + 3 > bytes.len() {
                        return Err("Invalid Simple block data");
                    }
                    let material = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]);
                    let rotation = bytes[cursor + 2];
                    cursor += 3;
                    Block::Simple(material, rotation)
                }
                2 => {
                    if cursor + 6 > bytes.len() {
                        return Err("Invalid Marching block data");
                    }
                    let material = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]);
                    let points = u32::from_le_bytes([
                        bytes[cursor + 2],
                        bytes[cursor + 3],
                        bytes[cursor + 4],
                        bytes[cursor + 5],
                    ]);
                    cursor += 6;
                    Block::Marching(material, points)
                }
                _ => return Err("Unknown block type"),
            };
            palette.push(block);
        }

        // 2. Read storage
        let storage_type = bytes.get(cursor).ok_or("Missing storage type")?;
        cursor += 1;

        let storage = match storage_type {
            0 => {
                // Uniform storage
                let idx = bytes.get(cursor).ok_or("Missing uniform index")?;
                cursor += 1;
                BlockStorage::Uniform(*idx)
            }
            1 => {
                // Sparse storage with RLE
                let mut indices = Box::new([0u8; 4096]);
                let mut pos = 0;

                while pos < 4096 && cursor + 1 < bytes.len() {
                    let value = bytes[cursor];
                    let count = bytes[cursor + 1] as usize;
                    cursor += 2;

                    let end = (pos + count).min(4096);
                    indices[pos..end].fill(value);
                    pos = end;
                }

                // If we didn't fill the entire array (corrupt data?), fill rest with air
                if pos < 4096 {
                    indices[pos..].fill(0);
                }

                BlockStorage::Sparse(indices)
            }
            _ => return Err("Unknown storage type"),
        };

        Ok(Chunk {
            palette,
            storage,
            dirty: true, // Mark as dirty to regenerate mesh
            mesh: None,
            bind_group: None,
        })
    }
}
