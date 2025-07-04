
use crate::block::main::Chunk;

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

/// Compact position within a chunk (0-15 on each axis)
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BlockPosition(u16);

impl BlockPosition {
    /// Creates a new BlockPosition from x,y,z coordinates (0-15)
    #[inline]
    // Current implementation:
    pub const fn new(x: u8, y: u8, z: u8) -> Self {
        Self((x as u16) << 0 | ((y as u16) << 4) | ((z as u16) << 8))
    }

    /// Universal constructor from any convertible type
    #[inline]
    pub fn from<T: Into<BlockPosition>>(value: T) -> Self {
        value.into()
    }
    /// Creates a new BlockPosition from a linear index (0-4095)
    #[inline]
    pub const fn from_index(index: u16) -> Self {
        debug_assert!(index < 4096, "Index must be 0-4095");
        Self(index)
    }

    /// Gets the x coordinate (0-15)
    #[inline]
    pub const fn x(&self) -> u8 {
        (self.0 & 0x000F) as u8
    }

    /// Gets the y coordinate (0-15)
    #[inline]
    pub const fn y(&self) -> u8 {
        ((self.0 >> 4) & 0x000F) as u8
    }

    /// Gets the z coordinate (0-15)
    #[inline]
    pub const fn z(&self) -> u8 {
        ((self.0 >> 8) & 0x000F) as u8
    }

    /// Gets the linear index (0-4095)
    #[inline]
    pub const fn index(&self) -> u16 {
        self.0
    }

    /// Offsets the position by dx, dy, dz (wrapping within chunk)
    #[inline]
    pub fn offset(&self, dx: i8, dy: i8, dz: i8) -> Self {
        let x = (self.x() as i16 + dx as i16).rem_euclid(16) as u8;
        let y = (self.y() as i16 + dy as i16).rem_euclid(16) as u8;
        let z = (self.z() as i16 + dz as i16).rem_euclid(16) as u8;
        Self::new(x, y, z)
    }

    /// Checks if this position is adjacent to another position
    #[inline]
    pub fn is_adjacent(&self, other: BlockPosition) -> bool {
        let dx = self.x().abs_diff(other.x());
        let dy = self.y().abs_diff(other.y());
        let dz = self.z().abs_diff(other.z());
        dx <= 1 && dy <= 1 && dz <= 1 && (dx + dy + dz) > 0
    }

    /// Returns an iterator over all 26 neighboring positions
    pub fn neighbors(&self) -> impl Iterator<Item = BlockPosition> {
        let pos = *self;
        (-1..=1).flat_map(move |dx| {
            (-1..=1).flat_map(move |dy| {
                (-1..=1).filter_map(move |dz| {
                    if dx == 0 && dy == 0 && dz == 0 {
                        None
                    } else {
                        Some(pos.offset(dx, dy, dz))
                    }
                })
            })
        })
    }
}


// Bidirectional conversions grouped by type
mod conversions {
    use super::*;

    // (u8, u8, u8) conversions
    impl From<(u8, u8, u8)> for BlockPosition {
        #[inline]
        fn from((x, y, z): (u8, u8, u8)) -> Self {
            Self::new(x, y, z)
        }
    }
    impl From<BlockPosition> for (u8, u8, u8) {
        #[inline]
        fn from(pos: BlockPosition) -> Self {
            (pos.x(), pos.y(), pos.z())
        }
    }

    // Vec3 conversions
    impl From<Vec3> for BlockPosition {
        #[inline]
        fn from(vec: Vec3) -> Self {
            Self::new(vec.x as u8, vec.y as u8, vec.z as u8)
        }
    }
    impl From<BlockPosition> for Vec3 {
        #[inline]
        fn from(pos: BlockPosition) -> Self {
            Vec3::new(pos.x().into(), pos.y().into(), pos.z().into())
        }
    }

    // u16 conversions
    impl From<u16> for BlockPosition {
        #[inline]
        fn from(index: u16) -> Self {
            Self::from_index(index)
        }
    }
    impl From<BlockPosition> for u16 {
        #[inline]
        fn from(pos: BlockPosition) -> Self {
            pos.index()
        }
    }

    // usize conversions (example of additional type)
    impl From<usize> for BlockPosition {
        #[inline]
        fn from(index: usize) -> Self {
            Self::from_index(index as u16)
        }
    }
    impl From<BlockPosition> for usize {
        #[inline]
        fn from(pos: BlockPosition) -> Self {
            pos.index() as usize
        }
    }
}

/// Axis enumeration for rotation
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum AxisBasic {
    X,
    Y,
    Z,
}

/// Axis enumeration with positive/negative variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    Xplus,
    Xminus,
    Yplus,
    Yminus,
    Zplus,
    Zminus,
}

use glam::{Quat,Vec3};

/// All 24 possible block rotations (6 faces × 4 orientations each).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)] // Ensures `as u8` is safe
pub enum BlockRotation {
    XplusYplus, XplusYminus, XplusZplus, XplusZminus,
    XminusYplus, XminusYminus, XminusZplus, XminusZminus,
    YplusXplus, YplusXminus, YplusZplus, YplusZminus,
    YminusXplus, YminusXminus, YminusZplus, YminusZminus,
    ZplusXplus, ZplusXminus, ZplusYplus, ZplusYminus,
    ZminusXplus, ZminusXminus, ZminusYplus, ZminusYminus,
}

// ===== Lookup Tables ===== //

/// Precomputed quaternions for all 24 rotations.
#[allow(dead_code)]
const QUATERNIONS: [Quat; 24] = [
    // +X facing (front)
    Quat::from_xyzw(0.0, 0.0, 0.0, 1.0),                     // X+Y+ (identity)
    Quat::from_xyzw(1.0, 0.0, 0.0, 0.0),                     // X+Y- (180° X)
    Quat::from_xyzw(0.0, 0.70710678, 0.0, 0.70710678),       // X+Z+ (90° Y)
    Quat::from_xyzw(0.0, -0.70710678, 0.0, 0.70710678),      // X+Z- (270° Y)
    // -X facing
    Quat::from_xyzw(0.0, 0.0, 1.0, 0.0),                     // X-Y+ (180° Z)
    Quat::from_xyzw(0.70710678, 0.0, 0.70710678, 0.0),       // X-Y- (180° X + Z)
    Quat::from_xyzw(0.5, -0.5, 0.5, 0.5),                    // X-Z+ (90° Y + 180° Z)
    Quat::from_xyzw(-0.5, -0.5, 0.5, 0.5),                   // X-Z- (270° Y + 180° Z)
    // +Y facing
    Quat::from_xyzw(-0.70710678, 0.0, 0.0, 0.70710678),      // Y+X+ (270° X)
    Quat::from_xyzw(-0.5, 0.5, 0.5, 0.5),                    // Y+X- (270° X + 180° Z)
    Quat::from_xyzw(-0.5, 0.5, -0.5, 0.5),                   // Y+Z+ (270° X + 90° Y)
    Quat::from_xyzw(-0.5, 0.5, 0.5, -0.5),                   // Y+Z- (270° X + 270° Y)
    // -Y facing
    Quat::from_xyzw(0.70710678, 0.0, 0.0, 0.70710678),       // Y-X+ (90° X)
    Quat::from_xyzw(0.5, 0.5, 0.5, 0.5),                     // Y-X- (90° X + 180° Z)
    Quat::from_xyzw(0.5, 0.5, -0.5, 0.5),                    // Y-Z+ (90° X + 90° Y)
    Quat::from_xyzw(0.5, 0.5, 0.5, -0.5),                    // Y-Z- (90° X + 270° Y)
    // +Z facing
    Quat::from_xyzw(0.0, 1.0, 0.0, 0.0),                     // Z+X+ (180° Y)
    Quat::from_xyzw(0.70710678, 0.70710678, 0.0, 0.0),       // Z+X- (180° Y + 180° Z)
    Quat::from_xyzw(0.0, 0.70710678, 0.70710678, 0.0),       // Z+Y+ (180° Y + 90° X)
    Quat::from_xyzw(0.0, 0.70710678, -0.70710678, 0.0),      // Z+Y- (180° Y + 270° X)
    // -Z facing
    Quat::from_xyzw(0.0, 0.0, 0.0, 1.0),                     // Z-X+ (identity)
    Quat::from_xyzw(0.0, 0.0, 1.0, 0.0),                     // Z-X- (180° Z)
    Quat::from_xyzw(0.70710678, 0.0, 0.0, 0.70710678),       // Z-Y+ (90° X)
    Quat::from_xyzw(-0.70710678, 0.0, 0.0, 0.70710678),      // Z-Y- (270° X)
];
/// Maps `BlockRotation` variants to their `u8` values (0..23).
#[allow(dead_code)]
const ROTATION_TO_BYTE: [u8; 24] = [
    0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23,
];
/// Maps `u8` values (0..23) back to `BlockRotation`.
#[allow(dead_code)]
const BYTE_TO_ROTATION: [BlockRotation; 24] = [
    BlockRotation::XplusYplus, BlockRotation::XplusYminus, BlockRotation::XplusZplus, BlockRotation::XplusZminus,
    BlockRotation::XminusYplus, BlockRotation::XminusYminus, BlockRotation::XminusZplus, BlockRotation::XminusZminus,
    BlockRotation::YplusXplus, BlockRotation::YplusXminus, BlockRotation::YplusZplus, BlockRotation::YplusZminus,
    BlockRotation::YminusXplus, BlockRotation::YminusXminus, BlockRotation::YminusZplus, BlockRotation::YminusZminus,
    BlockRotation::ZplusXplus, BlockRotation::ZplusXminus, BlockRotation::ZplusYplus, BlockRotation::ZplusYminus,
    BlockRotation::ZminusXplus, BlockRotation::ZminusXminus, BlockRotation::ZminusYplus, BlockRotation::ZminusYminus,
];

// ===== Safe Conversions ===== //
#[allow(dead_code)]
impl BlockRotation {
    /// Converts to a byte (0..23).
    #[inline]
    pub fn to_byte(self) -> u8 {
        ROTATION_TO_BYTE[self as usize]
    }

    /// Converts from a byte (returns `None` if invalid).
    #[inline]
    pub fn from_byte(byte: u8) -> Option<Self> {
        if byte < 24 {
            Some(BYTE_TO_ROTATION[byte as usize])
        } else {
            None
        }
    }

    /// Converts to a quaternion (uses precomputed LUT).
    #[inline]
    pub fn to_quat(self) -> Quat {
        QUATERNIONS[self as usize]
    }

    /// Finds the closest `BlockRotation` from a quaternion.
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

        // Determine secondary axis (up face)
        let secondary_axis = match primary_axis {
            Axis::Xplus | Axis::Xminus => {
                if up.y.abs() >= up.z.abs() {
                    if up.y > 0.0 { Axis::Yplus } else { Axis::Yminus }
                } else {
                    if up.z > 0.0 { Axis::Zplus } else { Axis::Zminus }
                }
            }
            Axis::Yplus | Axis::Yminus => {
                if up.x.abs() >= up.z.abs() {
                    if up.x > 0.0 { Axis::Xplus } else { Axis::Xminus }
                } else {
                    if up.z > 0.0 { Axis::Zplus } else { Axis::Zminus }
                }
            }
            Axis::Zplus | Axis::Zminus => {
                if up.x.abs() >= up.y.abs() {
                    if up.x > 0.0 { Axis::Xplus } else { Axis::Xminus }
                } else {
                    if up.y > 0.0 { Axis::Yplus } else { Axis::Yminus }
                }
            }
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
    pub fn rotate(self, axis: AxisBasic, steps: u8) -> Self {
        let quat = self.to_quat();
        let rotation = match axis {
            AxisBasic::X => Quat::from_rotation_x(steps as f32 * std::f32::consts::FRAC_PI_2),
            AxisBasic::Y => Quat::from_rotation_y(steps as f32 * std::f32::consts::FRAC_PI_2),
            AxisBasic::Z => Quat::from_rotation_z(steps as f32 * std::f32::consts::FRAC_PI_2),
        };
        Self::from_quat(rotation * quat)
    }
}