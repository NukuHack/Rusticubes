
use glam::{Vec3, IVec3};
use crate::block::main::Chunk;

/// Compact chunk coordinate representation (64 bits)
/// Format: [X:26 (signed), Y:12 (signed), Z:26 (signed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkCoord(u64);
impl From<ChunkCoord> for u64 {
	fn from(a: ChunkCoord) -> Self { 
		a.0 // Access the inner u64 value
	}
}
impl Into<ChunkCoord> for u64 {
	fn into(self) -> ChunkCoord {
		ChunkCoord(self) // Access the inner u64 value
	}
}
impl ChunkCoord {
	pub const ZERO:Self = Self::new(0,0,0);
	// Use bit shifts that are powers of 2 for better optimization
	//const Z_SHIFT: u8 = 0;
	const Y_SHIFT: u8 = 26;
	const X_SHIFT: u8 = 38;

	// Masks should match the shift counts
	const Z_MASK: u64 = (1 << 26) - 1;
	const Y_MASK: u64 = (1 << 12) - 1;
	const X_MASK: u64 = (1 << 26) - 1;

	/// Creates a new chunk coordinate
	#[inline] pub const fn new(x: i32, y: i32, z: i32) -> Self {
		Self(Self::pack(x, y, z))
	}

	/// Packs coordinates into a u64
	#[inline] pub const fn pack(x: i32, y: i32, z: i32) -> u64 {
		((x as u64 & Self::X_MASK) << Self::X_SHIFT)
			| ((y as u64 & Self::Y_MASK) << Self::Y_SHIFT)
			| (z as u64 & Self::Z_MASK)
	}

	/// Extracts (x, y, z) coordinates
	#[inline] pub const fn unpack(self) -> (i32, i32, i32) {
		(self.x(), self.y(), self.z())
	}
	/// Extracts (x, y, z) coordinates and make them world pos
	#[inline] pub const fn unpack_to_worldpos(self) -> (i32, i32, i32) {
		(self.x()*Chunk::SIZE_I, self.y()*Chunk::SIZE_I, self.z()*Chunk::SIZE_I)
	}

	/// Extracts X coordinate with sign extension
	#[inline] pub const fn x(self) -> i32 {
		((self.0 >> Self::X_SHIFT) as i32)
			.wrapping_shl(6)
			.wrapping_shr(6)
	}

	/// Extracts Y coordinate with sign extension
	#[inline] pub const fn y(self) -> i32 {
		((self.0 >> Self::Y_SHIFT) as i32 & Self::Y_MASK as i32)
			.wrapping_shl(20)
			.wrapping_shr(20)
	}

	/// Extracts Z coordinate with sign extension
	#[inline] pub const fn z(self) -> i32 {
		(self.0 as i32 & Self::Z_MASK as i32)
			.wrapping_shl(6)
			.wrapping_shr(6)
	}

	/// Converts to u64 
	#[inline] pub fn into_u64(self) -> u64 {
		u64::from(self)
	}

	/// Converts to world position (chunk min corner)
	#[inline] pub fn to_world_pos(self) -> Vec3 {
		let chunk_size = Chunk::SIZE_I;
		Vec3::new(
			(self.x() * chunk_size) as f32,
			(self.y() * chunk_size) as f32,
			(self.z() * chunk_size) as f32,
		)
	}

	/// Creates from world position
	#[inline] pub fn from_world_pos(world_pos: IVec3) -> Self {
		let chunk_size = Chunk::SIZE_I;
		Self::new(
			world_pos.x.div_euclid(chunk_size),
			world_pos.y.div_euclid(chunk_size),
			world_pos.z.div_euclid(chunk_size),
		)
	}
	#[inline] pub fn from_world_posf(world_pos: Vec3) -> Self {
		let chunk_size = Chunk::SIZE_I;
		Self::new(
			world_pos.x.div_euclid(chunk_size as f32) as i32,
			world_pos.y.div_euclid(chunk_size as f32) as i32,
			world_pos.z.div_euclid(chunk_size as f32) as i32,
		)
	}

	/// Offsets the chunk coordinate by `dx`, `dy`, `dz` (wrapping not applied since chunks are infinite)
	#[inline] pub const fn offset(&self, dx: i32, dy: i32, dz: i32) -> Self {
		Self::new(
			self.x().wrapping_add(dx),
			self.y().wrapping_add(dy),
			self.z().wrapping_add(dz),
		)
	}

	/// Returns the 6 directly adjacent chunk coordinates (no diagonals)
	#[inline] pub const fn get_adjacent(&self) -> [ChunkCoord; 6] {
		[
			self.offset(-1, 0, 0), // -X
			self.offset(1, 0, 0),  // +X
			self.offset(0, 0, -1), // -Z
			self.offset(0, 0, 1),  // +Z
			self.offset(0, 1, 0),  // +Y
			self.offset(0, -1, 0), // -Y
		]
	}

	/// Checks if this chunk is adjacent to another chunk (direct neighbors)
	#[inline] pub fn is_adjacent(&self, other: ChunkCoord) -> bool {
		let dx = self.x().abs_diff(other.x());
		let dy = self.y().abs_diff(other.y());
		let dz = self.z().abs_diff(other.z());
		dx <= 1 && dy <= 1 && dz <= 1 && (dx + dy + dz) > 0
	}

	/// Returns an iterator over all 26 neighboring chunks
	pub fn neighbors(&self) -> impl Iterator<Item = ChunkCoord> {
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
impl std::ops::Mul<i32> for ChunkCoord {
	type Output = Self;
	fn mul(self, rhs: i32) -> Self {
		let (x,y,z) = self.unpack();
		Self::new(
			x * rhs,
			y * rhs,
			z * rhs
		)
	}
}
impl std::ops::Add for ChunkCoord {
	type Output = Self;
	fn add(self, rhs: Self) -> Self {
		let (x,y,z) = self.unpack();
		let (x2,y2,z2) = rhs.unpack();
		Self::new(
			x + x2,
			y + y2,
			z + z2
		)
	}
}

/// Compact position within a chunk (0-15 on each axis)
// 2 ; 4 ; 8 ; 16 ; 32
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LocalPos(u16);

impl LocalPos {
	pub const ZERO:Self = Self::new(0,0,0);
	pub const CORNER:Self = Self::new(Chunk::SIZE as u8 -1, Chunk::SIZE as u8 -1, Chunk::SIZE as u8 -1);
	pub const OFFSET:u8 = 5;
	pub const MASK: u16 = (1 << Self::OFFSET) - 1;

	/// Creates a new LocalPos from x,y,z coordinates (0-15)
	#[inline] pub const fn new(x: u8, y: u8, z: u8) -> Self {
		Self((x as u16) << Self::OFFSET*0 | ((y as u16) << Self::OFFSET*1) | ((z as u16) << Self::OFFSET*2))
	}

	/// Universal constructor from any convertible type
	#[inline]
	pub fn from<T: Into<LocalPos>>(value: T) -> Self {
		value.into()
	}
	/// Creates a new LocalPos from a linear index (0-4095)
	#[inline] pub const fn from_index(index: u16) -> Self {
		debug_assert!(index < Chunk::VOLUME as u16, "Index must be 0-Chunk::VOLUME");
		Self(index)
	}

	/// Gets the x coordinate (0-15)
	#[inline] pub const fn x(&self) -> u8 {
		((self.0 >> Self::OFFSET*0) & Self::MASK) as u8
	}

	/// Gets the y coordinate (0-15)
	#[inline] pub const fn y(&self) -> u8 {
		((self.0 >> Self::OFFSET*1) & Self::MASK) as u8
	}

	/// Gets the z coordinate (0-15)
	#[inline] pub const fn z(&self) -> u8 {
		((self.0 >> Self::OFFSET*2) & Self::MASK) as u8
	}

	/// Gets the linear index (0-4095)
	#[inline] pub const fn index(&self) -> u16 {
		self.0
	}

	/// Offsets the position by dx, dy, dz (wrapping within chunk)
	#[inline] pub const fn offset(&self, dx: i8, dy: i8, dz: i8) -> Self {
		let chunk_size = Chunk::SIZE_I;
		let x = (self.x() as i32 + dx as i32).rem_euclid(chunk_size) as u8;
		let y = (self.y() as i32 + dy as i32).rem_euclid(chunk_size) as u8;
		let z = (self.z() as i32 + dz as i32).rem_euclid(chunk_size) as u8;
		Self::new(x, y, z)
	}

	/// Returns the 6 directly adjacent block positions within the chunk (no diagonals)
	#[inline] pub const fn get_adjacent(&self) -> [LocalPos; 6] {
		[
			self.offset(-1, 0, 0), // -X
			self.offset(1, 0, 0),  // +X
			self.offset(0, 0, -1), // -Z
			self.offset(0, 0, 1),  // +Z
			self.offset(0, 1, 0),  // +Y
			self.offset(0, -1, 0), // -Y
		]
	}

	#[inline] pub fn to_chunk_coord(&self) -> ChunkCoord {
		ChunkCoord::new(
			self.x() as i32,
			self.y() as i32,
			self.z() as i32
		)
	}

	/// Checks if this position is adjacent to another position
	#[inline] pub const fn is_adjacent(&self, other: LocalPos) -> bool {
		let dx = self.x().abs_diff(other.x());
		let dy = self.y().abs_diff(other.y());
		let dz = self.z().abs_diff(other.z());
		dx <= 1 && dy <= 1 && dz <= 1 && (dx + dy + dz) > 0
	}

	/// Returns an iterator over all 26 neighboring positions
	pub fn neighbors(&self) -> impl Iterator<Item = LocalPos> {
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
	
	impl From<(f32, f32, f32)> for LocalPos {
		#[inline]
		fn from((x, y, z): (f32, f32, f32)) -> Self {
			let chunk_size = Chunk::SIZE_I;
			Self::new(
				(x.floor() as i32).rem_euclid(chunk_size) as u8,
				(y.floor() as i32).rem_euclid(chunk_size) as u8,
				(z.floor() as i32).rem_euclid(chunk_size) as u8
			)
		}
	}
	// (u8, u8, u8) conversions
	impl From<(u8, u8, u8)> for LocalPos {
		#[inline]
		fn from((x, y, z): (u8, u8, u8)) -> Self {
			Self::new(x, y, z)
		}
	}
	// (usize, usize, usize) conversions
	impl From<(usize, usize, usize)> for LocalPos {
		#[inline]
		fn from((x, y, z): (usize, usize, usize)) -> Self {
			Self::new(x as u8, y as u8, z as u8)
		}
	}
	impl From<LocalPos> for (u8, u8, u8) {
		#[inline]
		fn from(pos: LocalPos) -> Self {
			(pos.x(), pos.y(), pos.z())
		}
	}

	// Vec3 conversions
	impl From<Vec3> for LocalPos {
		#[inline]
		fn from(vec: Vec3) -> Self {
			let chunk_size = Chunk::SIZE_I;
			Self::new(
				(vec.x.floor() as i32).rem_euclid(chunk_size) as u8,
				(vec.y.floor() as i32).rem_euclid(chunk_size) as u8,
				(vec.z.floor() as i32).rem_euclid(chunk_size) as u8
			)
		}
	}
	impl From<LocalPos> for Vec3 {
		#[inline]
		fn from(pos: LocalPos) -> Self {
			Vec3::new(pos.x() as f32, pos.y() as f32, pos.z() as f32)
		}
	}
	impl From<LocalPos> for IVec3 {
		#[inline]
		fn from(pos: LocalPos) -> Self {
			IVec3::new(pos.x() as i32, pos.y() as i32, pos.z() as i32)
		}
	}
	impl From<IVec3> for LocalPos {
		#[inline]
		fn from(vec: IVec3) -> Self {
			let chunk_size = Chunk::SIZE_I;
			Self::new(
				(vec.x as i32).rem_euclid(chunk_size) as u8,
				(vec.y as i32).rem_euclid(chunk_size) as u8,
				(vec.z as i32).rem_euclid(chunk_size) as u8
			)
		}
	}

	// u16 conversions
	impl From<u16> for LocalPos {
		#[inline]
		fn from(index: u16) -> Self {
			Self::from_index(index)
		}
	}
	impl From<LocalPos> for u16 {
		#[inline]
		fn from(pos: LocalPos) -> Self {
			pos.index()
		}
	}

	// usize conversions (example of additional type)
	impl From<usize> for LocalPos {
		#[inline]
		fn from(index: usize) -> Self {
			Self::from_index(index as u16)
		}
	}
	impl From<LocalPos> for usize {
		#[inline]
		fn from(pos: LocalPos) -> Self {
			pos.index() as usize
		}
	}
}


/// Axis enumeration for rotation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisBasic {
	X,
	Y,
	Z,
}
impl AxisBasic {
	/// Constant-time equality check
	#[inline]
	pub const fn eq(self, other: Self) -> bool {
		matches!(
			(self, other),
			(AxisBasic::X, AxisBasic::X) |
			(AxisBasic::Y, AxisBasic::Y) |
			(AxisBasic::Z, AxisBasic::Z)
		)
	}
}

/// Axis enumeration with positive/negative variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Axis {
	Xplus = 0,
	Xminus = 1,
	Yplus = 2,
	Yminus = 3,
	Zplus = 4,
	Zminus = 5,
}

impl Axis {
	/// Convert from u8 value
	pub const fn from_u8(value: u8) -> Option<Self> {
		match value {
			0 => Some(Axis::Xplus),
			1 => Some(Axis::Xminus),
			2 => Some(Axis::Yplus),
			3 => Some(Axis::Yminus),
			4 => Some(Axis::Zplus),
			5 => Some(Axis::Zminus),
			_ => None,
		}
	}
	
	/// Convert to u8
	pub const fn to_u8(self) -> u8 {
		self as u8
	}

	// Helper functions
	pub const fn to_vec(&self) -> (i8, i8, i8) {
		match self {
			Axis::Xplus => (1, 0, 0),
			Axis::Xminus => (-1, 0, 0),
			Axis::Yplus => (0, 1, 0),
			Axis::Yminus => (0, -1, 0),
			Axis::Zplus => (0, 0, 1),
			Axis::Zminus => (0, 0, -1),
		}
	}
	
	pub const fn from_vec((x, y, z): (i8, i8, i8)) -> Self {
		match (x, y, z) {
			(1, 0, 0) => Axis::Xplus,
			(-1, 0, 0) => Axis::Xminus,
			(0, 1, 0) => Axis::Yplus,
			(0, -1, 0) => Axis::Yminus,
			(0, 0, 1) => Axis::Zplus,
			(0, 0, -1) => Axis::Zminus,
			_ => panic!("Invalid axis vector"),
		}
	}
	
	/// Get the basic axis (ignoring direction)
	pub const fn basic(self) -> AxisBasic {
		match self {
			Axis::Xplus | Axis::Xminus => AxisBasic::X,
			Axis::Yplus | Axis::Yminus => AxisBasic::Y,
			Axis::Zplus | Axis::Zminus => AxisBasic::Z,
		}
	}
	
	/// Get the opposite axis
	pub const fn opposite(self) -> Self {
		match self {
			Axis::Xplus => Axis::Xminus,
			Axis::Xminus => Axis::Xplus,
			Axis::Yplus => Axis::Yminus,
			Axis::Yminus => Axis::Yplus,
			Axis::Zplus => Axis::Zminus,
			Axis::Zminus => Axis::Zplus,
		}
	}
	
	/// Check if two axes are compatible (not the same or opposite)
	pub const fn is_compatible_with(self, other: Axis) -> bool {
		!self.basic().eq(other.basic())
	}
}

/// Compact block rotation representation using only 1 byte
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)] // Same representation as u8
pub struct BlockRotation(u8);

impl BlockRotation {
	/// Create a new BlockRotation from primary and secondary axes
	#[inline]
	pub const fn new(primary: Axis, secondary: Axis) -> Self {
		assert!(primary.is_compatible_with(secondary), "Primary and secondary axes must be different");
		// Store primary in lower 3 bits, secondary in upper 3 bits
		let value = (secondary.to_u8() << 3) | primary.to_u8();
		Self(value)
	}
	
	/// Get the primary axis (facing direction)
	#[inline]
	pub const fn primary_axis(self) -> Axis {
		// Safe because we validate during construction
		Axis::from_u8(self.0 & 0b111).unwrap()
	}
	#[inline]
	pub const fn secondary_axis(self) -> Axis {
		// Safe because we validate during construction
		Axis::from_u8((self.0 >> 3) & 0b111).unwrap()
	}
	
	#[inline]
	pub const fn as_u8(self) -> u8 {
		self.0
	}
	#[inline]
	pub const fn from_u8(value: u8) -> Option<Self> {
		let primary_bits = value & 0b111;
		let secondary_bits = (value >> 3) & 0b111;
		
		// Validate both axis values
		let Some(primary) = Axis::from_u8(primary_bits) else { return None; };
		let Some(secondary) = Axis::from_u8(secondary_bits) else { return None; };
		// Validate that axes are compatible (not the same or opposite)
		if !primary.is_compatible_with(secondary) { return None; }
		
		return Some(Self(value));
	}
	
	/// Rotate the block around an axis by 90° steps (1 step = 90° clockwise)
	pub const fn rotate(self, axis: AxisBasic, steps: u8) -> Self {
		if steps % 4 == 0 { return self; }
		
		let primary = self.primary_axis();
		let secondary = self.secondary_axis();
		
		// Convert axes to vectors for easier rotation
		let primary_vec = primary.to_vec();
		let secondary_vec = secondary.to_vec();
		
		// Rotate both vectors around the specified axis
		let new_primary_vec = Self::rotate_vector(primary_vec, axis, steps%4);
		let new_secondary_vec = Self::rotate_vector(secondary_vec, axis, steps%4);
		
		// Convert back to axes
		let new_primary = Axis::from_vec(new_primary_vec);
		let new_secondary = Axis::from_vec(new_secondary_vec);
		
		Self::new(new_primary, new_secondary)
	}
	
	
	const fn rotate_vector((x, y, z): (i8, i8, i8), axis: AxisBasic, steps: u8) -> (i8, i8, i8) {
		// Since steps is guaranteed to be 1, 2, or 3, we can use a match
		match steps {
			1 => {
				match axis {
					AxisBasic::X => (x, -z, y),    // Rotate around X: (y, z) → (-z, y)
					AxisBasic::Y => (z, y, -x),    // Rotate around Y: (x, z) → (z, -x)
					AxisBasic::Z => (-y, x, z),    // Rotate around Z: (x, y) → (-y, x)
				}
			}
			2 => {
				// Two rotations = 180 degrees
				match axis {
					AxisBasic::X => (x, -y, -z),   // Rotate around X twice: (y, z) → (-y, -z)
					AxisBasic::Y => (-x, y, -z),   // Rotate around Y twice: (x, z) → (-x, -z)
					AxisBasic::Z => (-x, -y, z),   // Rotate around Z twice: (x, y) → (-x, -y)
				}
			}
			3 => {
				// Three rotations = 270 degrees (equivalent to -90 degrees)
				match axis {
					AxisBasic::X => (x, z, -y),    // Rotate around X: (y, z) → (z, -y)
					AxisBasic::Y => (-z, y, x),    // Rotate around Y: (x, z) → (-z, x)
					AxisBasic::Z => (y, -x, z),    // Rotate around Z: (x, y) → (y, -x)
				}
			}
			_ => (x, y, z), // Should never happen if steps is guaranteed to be 1-3
		}
	}
}

// Example usage and constants for all 24 rotations
impl BlockRotation {
	pub const XPLUS_YPLUS: Self = Self::new(Axis::Xplus, Axis::Yplus);
	pub const XPLUS_YMINUS: Self = Self::new(Axis::Xplus, Axis::Yminus);
	pub const XPLUS_ZPLUS: Self = Self::new(Axis::Xplus, Axis::Zplus);
	pub const XPLUS_ZMINUS: Self = Self::new(Axis::Xplus, Axis::Zminus);
	
	pub const XMINUS_YPLUS: Self = Self::new(Axis::Xminus, Axis::Yplus);
	pub const XMINUS_YMINUS: Self = Self::new(Axis::Xminus, Axis::Yminus);
	pub const XMINUS_ZPLUS: Self = Self::new(Axis::Xminus, Axis::Zplus);
	pub const XMINUS_ZMINUS: Self = Self::new(Axis::Xminus, Axis::Zminus);
	
	pub const YPLUS_XPLUS: Self = Self::new(Axis::Yplus, Axis::Xplus);
	pub const YPLUS_XMINUS: Self = Self::new(Axis::Yplus, Axis::Xminus);
	pub const YPLUS_ZPLUS: Self = Self::new(Axis::Yplus, Axis::Zplus);
	pub const YPLUS_ZMINUS: Self = Self::new(Axis::Yplus, Axis::Zminus);
	
	pub const YMINUS_XPLUS: Self = Self::new(Axis::Yminus, Axis::Xplus);
	pub const YMINUS_XMINUS: Self = Self::new(Axis::Yminus, Axis::Xminus);
	pub const YMINUS_ZPLUS: Self = Self::new(Axis::Yminus, Axis::Zplus);
	pub const YMINUS_ZMINUS: Self = Self::new(Axis::Yminus, Axis::Zminus);
	
	pub const ZPLUS_XPLUS: Self = Self::new(Axis::Zplus, Axis::Xplus);
	pub const ZPLUS_XMINUS: Self = Self::new(Axis::Zplus, Axis::Xminus);
	pub const ZPLUS_YPLUS: Self = Self::new(Axis::Zplus, Axis::Yplus);
	pub const ZPLUS_YMINUS: Self = Self::new(Axis::Zplus, Axis::Yminus);
	
	pub const ZMINUS_XPLUS: Self = Self::new(Axis::Zminus, Axis::Xplus);
	pub const ZMINUS_XMINUS: Self = Self::new(Axis::Zminus, Axis::Xminus);
	pub const ZMINUS_YPLUS: Self = Self::new(Axis::Zminus, Axis::Yplus);
	pub const ZMINUS_YMINUS: Self = Self::new(Axis::Zminus, Axis::Yminus);
}
