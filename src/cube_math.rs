
use std::f32::consts::PI;
use glam::Quat;
use super::cube::Chunk;
use std::f32::consts::FRAC_PI_2;
use glam::Vec3;

/// Axis enumeration for rotation
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub enum AxisBasic {
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