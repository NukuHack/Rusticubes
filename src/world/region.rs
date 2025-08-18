
use crate::block::math::ChunkCoord;


/// Compact region coordinate representation (64 bits)
/// Format: [X:26 (signed), Y:12 (signed), Z:26 (signed)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Region(u64);

impl Region {
	pub const ZERO:Self = Self::new(0,0,0);
	pub const SIZE:i32 = 32;
	pub const PREFIX: &str = "r.";
	pub const SUFFIX: &str = ".dat";
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

	/// Converts to ChunkCoord at the region's origin
	#[inline]
	pub const fn to_chunk_coord(self) -> ChunkCoord {
		ChunkCoord::new(
			self.x() * Self::SIZE,
			self.y() * Self::SIZE,
			self.z() * Self::SIZE,
		)
	}
	/// Converts to ChunkCoord at the region's origin
	#[inline]
	pub const fn from_chunk_coord(chunk_coord: ChunkCoord) -> Self {
		Self::new(
			chunk_coord.x().div_euclid(Self::SIZE),
			chunk_coord.y().div_euclid(Self::SIZE),
			chunk_coord.z().div_euclid(Self::SIZE),
		)
	}
}

// Implement basic arithmetic operations
impl std::ops::Add for Region {
	type Output = Self;
	fn add(self, rhs: Self) -> Self {
		Self::new(self.x() + rhs.x(), self.y() + rhs.y(), self.z() + rhs.z())
	}
}

impl std::ops::Mul<i32> for Region {
	type Output = Self;
	fn mul(self, rhs: i32) -> Self {
		Self::new(self.x() * rhs, self.y() * rhs, self.z() * rhs)
	}
}

impl From<Region> for u64 {
	fn from(a: Region) -> Self { 
		a.0 // Access the inner u64 value
	}
}
impl Into<Region> for u64 {
	fn into(self) -> Region {
		Region(self) // Access the inner u64 value
	}
}
