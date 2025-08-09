
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
#[allow(dead_code)]
impl ChunkCoord {
	pub const ZERO:Self = Self::new(0,0,0);
	// Use bit shifts that are powers of 2 for better optimization
	const Z_SHIFT: u8 = 0;
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
		self.into()
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
pub struct BlockPosition(u16);

impl BlockPosition {
	pub const ZERO:Self = Self::new(0,0,0);
	pub const CORNER:Self = Self::new(Chunk::SIZE as u8 -1, Chunk::SIZE as u8 -1, Chunk::SIZE as u8 -1);
	pub const OFFSET:u8 = 4;
    pub const MASK: u16 = (1 << Self::OFFSET) - 1;

	/// Creates a new BlockPosition from x,y,z coordinates (0-15)
	#[inline] pub const fn new(x: u8, y: u8, z: u8) -> Self {
		Self((x as u16) << Self::OFFSET*0 | ((y as u16) << Self::OFFSET*1) | ((z as u16) << Self::OFFSET*2))
	}

	/// Universal constructor from any convertible type
	#[inline]
	pub fn from<T: Into<BlockPosition>>(value: T) -> Self {
		value.into()
	}
	/// Creates a new BlockPosition from a linear index (0-4095)
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
	#[inline] pub const fn get_adjacent(&self) -> [BlockPosition; 6] {
		[
			self.offset(-1, 0, 0), // -X
			self.offset(1, 0, 0),  // +X
			self.offset(0, 0, -1), // -Z
			self.offset(0, 0, 1),  // +Z
			self.offset(0, 1, 0),  // +Y
			self.offset(0, -1, 0), // -Y
		]
	}

	/// Checks if this position is adjacent to another position
	#[inline] pub const fn is_adjacent(&self, other: BlockPosition) -> bool {
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
	
	impl From<(f32, f32, f32)> for BlockPosition {
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
	impl From<(u8, u8, u8)> for BlockPosition {
		#[inline]
		fn from((x, y, z): (u8, u8, u8)) -> Self {
			Self::new(x, y, z)
		}
	}
	// (usize, usize, usize) conversions
	impl From<(usize, usize, usize)> for BlockPosition {
		#[inline]
		fn from((x, y, z): (usize, usize, usize)) -> Self {
			Self::new(x as u8, y as u8, z as u8)
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
			let chunk_size = Chunk::SIZE_I;
			Self::new(
				(vec.x.floor() as i32).rem_euclid(chunk_size) as u8,
				(vec.y.floor() as i32).rem_euclid(chunk_size) as u8,
				(vec.z.floor() as i32).rem_euclid(chunk_size) as u8
			)
		}
	}
	impl From<BlockPosition> for Vec3 {
		#[inline]
		fn from(pos: BlockPosition) -> Self {
			Vec3::new(pos.x().into(), pos.y().into(), pos.z().into())
		}
	}
	impl From<BlockPosition> for IVec3 {
		#[inline]
		fn from(pos: BlockPosition) -> Self {
			IVec3::new(pos.x().into(), pos.y().into(), pos.z().into())
		}
	}
	impl From<IVec3> for BlockPosition {
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

/// All 24 possible block rotations (6 faces × 4 orientations each).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)] // Ensures `as u8` is safe
pub enum BlockRotation {
	XplusYplus, XplusYminus, XplusZplus, XplusZminus,
	XminusYplus, XminusYminus, XminusZplus, XminusZminus,
	YplusXplus, YplusXminus, YplusZplus, YplusZminus,
	YminusXplus, YminusXminus, YminusZplus, YminusZminus,
	ZplusXplus, ZplusXminus, ZplusYplus, ZplusYminus,
	ZminusXplus, ZminusXminus, ZminusYplus, ZminusYminus,
}

impl BlockRotation {

	/// Returns the primary axis of this rotation
	#[inline] pub const fn primary_axis(self) -> Axis {
		match self {
			BlockRotation::XplusYplus | BlockRotation::XplusYminus |
			BlockRotation::XplusZplus | BlockRotation::XplusZminus => Axis::Xplus,
			
			BlockRotation::XminusYplus | BlockRotation::XminusYminus |
			BlockRotation::XminusZplus | BlockRotation::XminusZminus => Axis::Xminus,
			
			BlockRotation::YplusXplus | BlockRotation::YplusXminus |
			BlockRotation::YplusZplus | BlockRotation::YplusZminus => Axis::Yplus,
			
			BlockRotation::YminusXplus | BlockRotation::YminusXminus |
			BlockRotation::YminusZplus | BlockRotation::YminusZminus => Axis::Yminus,
			
			BlockRotation::ZplusXplus | BlockRotation::ZplusXminus |
			BlockRotation::ZplusYplus | BlockRotation::ZplusYminus => Axis::Zplus,
			
			BlockRotation::ZminusXplus | BlockRotation::ZminusXminus |
			BlockRotation::ZminusYplus | BlockRotation::ZminusYminus => Axis::Zminus,
		}
	}

	/// Returns the secondary axis of this rotation
	#[inline] pub const fn secondary_axis(self) -> Axis {
		match self {
			BlockRotation::XplusYplus | BlockRotation::XminusYplus |
			BlockRotation::YplusXplus | BlockRotation::YminusXplus  => Axis::Xplus,
			
			BlockRotation::YplusXminus | BlockRotation::YminusXminus |
			BlockRotation::ZplusXminus | BlockRotation::ZminusXminus => Axis::Xminus,
			
			BlockRotation::XplusZplus | BlockRotation::XminusZplus |
			BlockRotation::YplusZplus | BlockRotation::YminusZplus  => Axis::Zplus,
			
			BlockRotation::XplusZminus | BlockRotation::XminusZminus |
			BlockRotation::YplusZminus | BlockRotation::YminusZminus  => Axis::Zminus,
			
			BlockRotation::XplusYminus | BlockRotation::XminusYminus |
			BlockRotation::ZplusYminus | BlockRotation::ZminusYminus => Axis:: Yminus,

			_ => Axis::Zplus, // Remaining cases are Z variants
		}
	}

	/// Rotates the block around an axis by 90° steps (1 step = 90° clockwise)
	pub fn rotate(self, axis: AxisBasic, steps: u8) -> Self {
		let steps = steps % 4; // Normalize to 0-3
		if steps == 0 {
			return self;
		}

		let (primary, secondary) = match axis {
			AxisBasic::X => {
				// When rotating around X, Y and Z axes change
				let y_axis = match self.secondary_axis() {
					Axis::Yplus | Axis::Yminus => self.secondary_axis(),
					_ => Axis::Yplus, // Default if not Y
				};
				let z_axis = match self.secondary_axis() {
					Axis::Zplus | Axis::Zminus => self.secondary_axis(),
					_ => Axis::Zplus, // Default if not Z
				};
				(self.primary_axis(), if steps % 2 == 1 { z_axis } else { y_axis })
			},
			AxisBasic::Y => {
				// When rotating around Y, X and Z axes change
				let x_axis = match self.secondary_axis() {
					Axis::Xplus | Axis::Xminus => self.secondary_axis(),
					_ => Axis::Xplus,
				};
				let z_axis = match self.secondary_axis() {
					Axis::Zplus | Axis::Zminus => self.secondary_axis(),
					_ => Axis::Zplus,
				};
				(self.primary_axis(), if steps % 2 == 1 { x_axis } else { z_axis })
			},
			AxisBasic::Z => {
				// When rotating around Z, X and Y axes change
				let x_axis = match self.secondary_axis() {
					Axis::Xplus | Axis::Xminus => self.secondary_axis(),
					_ => Axis::Xplus,
				};
				let y_axis = match self.secondary_axis() {
					Axis::Yplus | Axis::Yminus => self.secondary_axis(),
					_ => Axis::Yplus,
				};
				(self.primary_axis(), if steps % 2 == 1 { y_axis } else { x_axis })
			},
		};

		// Reconstruct the new rotation based on primary and secondary axes
		match (primary, secondary) {
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

			_ => panic!("The rotation is incorrect"),
		}
	}

}
