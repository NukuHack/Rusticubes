
use super::cube::{Block, BlockStorage, Chunk};
use glam::Vec3;
use std::io::{Error, ErrorKind};
use std::path::PathBuf;

/// Axis enumeration for rotation
#[allow(dead_code)]
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

#[allow(dead_code)]
const CHUNK_HEADER: [u8; 4] = *b"CHNK";
#[allow(dead_code)]
const CURRENT_VERSION: u8 = 1;

#[allow(dead_code)]
pub struct ChunkSerializer;

#[allow(dead_code)]
impl ChunkSerializer {
    pub fn save(chunk: &Chunk, world_path: &PathBuf, coord: ChunkCoord) -> std::io::Result<()> {
        let chunk_dir = world_path.join("chunks");
        std::fs::create_dir_all(&chunk_dir)?;

        let filename = format!("c.{}.{}.{}.dat", coord.x(), coord.y(), coord.z());
        let path = chunk_dir.join(filename);

        let data = Self::serialize(chunk)?;
        std::fs::write(path, data)
    }

    pub fn load(world_path: &PathBuf, coord: ChunkCoord) -> std::io::Result<Chunk> {
        let filename = format!("c.{}.{}.{}.dat", coord.x(), coord.y(), coord.z());
        let path = world_path.join("chunks").join(filename);

        let data = std::fs::read(path)?;
        Self::deserialize(&data)
    }

    // Simple checksum algorithm (Fletcher-16 variant)
    fn calculate_checksum(data: &[u8]) -> u32 {
        let mut sum1: u32 = 0;
        let mut sum2: u32 = 0;

        for &byte in data {
            sum1 = (sum1 + byte as u32) % 255;
            sum2 = (sum2 + sum1) % 255;
        }

        (sum2 << 8) | sum1
    }

    fn serialize(chunk: &Chunk) -> std::io::Result<Vec<u8>> {
        let mut buffer = Vec::new();

        // Header
        buffer.extend_from_slice(&CHUNK_HEADER);
        buffer.push(CURRENT_VERSION);

        // Palette
        buffer.push(chunk.palette.len() as u8);
        for block in &chunk.palette {
            match block {
                Block::None => buffer.push(0),
                Block::Simple(mat, rot) => {
                    buffer.push(1);
                    buffer.extend_from_slice(&mat.to_le_bytes());
                    buffer.push(*rot);
                }
                Block::Marching(mat, points) => {
                    buffer.push(2);
                    buffer.extend_from_slice(&mat.to_le_bytes());
                    buffer.extend_from_slice(&points.to_le_bytes());
                }
            }
        }

        // Block Data
        match &chunk.storage {
            BlockStorage::Uniform(idx) => {
                buffer.push(0); // Uniform marker
                buffer.push(*idx);
            }
            BlockStorage::Sparse(indices) => {
                buffer.push(1); // Sparse marker

                // RLE compression
                let mut current = indices[0];
                let mut count = 1u16;

                for &val in indices.iter().skip(1) {
                    if val == current && count < u16::MAX {
                        count += 1;
                    } else {
                        buffer.push(current);
                        buffer.extend_from_slice(&count.to_le_bytes());
                        current = val;
                        count = 1;
                    }
                }
                // Write last run
                buffer.push(current);
                buffer.extend_from_slice(&count.to_le_bytes());
            }
        }

        // Checksum
        let checksum = Self::calculate_checksum(&buffer);
        buffer.extend_from_slice(&checksum.to_le_bytes());

        Ok(buffer)
    }

    fn deserialize(data: &[u8]) -> std::io::Result<Chunk> {
        if data.len() < 4 || &data[0..4] != CHUNK_HEADER {
            return Err(Error::new(ErrorKind::InvalidData, "Invalid chunk header"));
        }

        let version = data[4];
        if version != CURRENT_VERSION {
            return Err(Error::new(ErrorKind::InvalidData, "Unsupported version"));
        }

        let data_len = data.len();
        if data_len < 8 {
            // Minimum viable chunk size (header + version + minimal data + checksum)
            return Err(Error::new(ErrorKind::InvalidData, "Data too short"));
        }

        // Verify checksum
        let checksum_data = &data[..data_len - 4];
        let calculated_checksum = Self::calculate_checksum(checksum_data);
        let stored_checksum = u32::from_le_bytes([
            data[data_len - 4],
            data[data_len - 3],
            data[data_len - 2],
            data[data_len - 1],
        ]);

        if calculated_checksum != stored_checksum {
            return Err(Error::new(ErrorKind::InvalidData, "Checksum mismatch"));
        }

        let mut pos = 5; // Skip header (4) + version (1)

        // Read palette
        let palette_size = data[pos] as usize;
        pos += 1;

        let mut palette = Vec::with_capacity(palette_size);
        for _ in 0..palette_size {
            let block_type = data[pos];
            pos += 1;

            let block = match block_type {
                0 => Block::None,
                1 => {
                    if pos + 3 > data_len - 4 {
                        return Err(Error::new(ErrorKind::InvalidData, "Invalid block data"));
                    }
                    let mat = u16::from_le_bytes([data[pos], data[pos + 1]]);
                    let rot = data[pos + 2];
                    pos += 3;
                    Block::Simple(mat, rot)
                }
                2 => {
                    if pos + 6 > data_len - 4 {
                        return Err(Error::new(ErrorKind::InvalidData, "Invalid block data"));
                    }
                    let mat = u16::from_le_bytes([data[pos], data[pos + 1]]);
                    let points = u32::from_le_bytes([
                        data[pos + 2],
                        data[pos + 3],
                        data[pos + 4],
                        data[pos + 5],
                    ]);
                    pos += 6;
                    Block::Marching(mat, points)
                }
                _ => return Err(Error::new(ErrorKind::InvalidData, "Unknown block type")),
            };
            palette.push(block);
        }

        // Read storage
        let storage = if pos >= data_len - 4 {
            return Err(Error::new(ErrorKind::InvalidData, "Missing storage data"));
        } else {
            match data[pos] {
                0 => {
                    // Uniform storage
                    pos += 1;
                    if pos >= data_len - 4 {
                        return Err(Error::new(ErrorKind::InvalidData, "Missing uniform index"));
                    }
                    BlockStorage::Uniform(data[pos])
                }
                1 => {
                    // Sparse storage
                    pos += 1;
                    let mut indices = Box::new([0u8; 4096]);
                    let mut idx = 0;

                    while idx < 4096 && pos + 2 < data_len - 4 {
                        let val = data[pos];
                        let count = u16::from_le_bytes([data[pos + 1], data[pos + 2]]) as usize;
                        pos += 3;

                        let end = (idx + count).min(4096);
                        indices[idx..end].fill(val);
                        idx = end;
                    }

                    // Fill remaining with air if incomplete
                    if idx < 4096 {
                        indices[idx..].fill(0);
                    }

                    BlockStorage::Sparse(indices)
                }
                _ => return Err(Error::new(ErrorKind::InvalidData, "Unknown storage type")),
            }
        };

        Ok(Chunk {
            palette,
            storage,
            dirty: true,
            mesh: None,
            bind_group: None,
        })
    }
}
