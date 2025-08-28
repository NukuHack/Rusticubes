use crate::utils::color::Color;
use crate::utils::time::Time;
use glam::{Vec2, IVec3, UVec3, Vec3};
use std::num::NonZero;

// Trait for binary serialization
pub trait BinarySerializable {
	fn to_binary(&self) -> Vec<u8>;
	fn from_binary(bytes: &[u8]) -> Option<Self> where Self: Sized;
	fn binary_size(&self) -> usize;
}

// Fixed-size serialization trait for types with known sizes
pub trait FixedBinarySize: BinarySerializable {
	const BINARY_SIZE: usize;
}

// Basic type implementations
macro_rules! impl_basic_types {
	($($t:ty),*) => { $(
	impl BinarySerializable for $t {
		fn to_binary(&self) -> Vec<u8> {
			self.to_le_bytes().to_vec()
		}
		fn from_binary(bytes: &[u8]) -> Option<Self> {
			bytes.get(..Self::BINARY_SIZE)
				.and_then(|slice| slice.try_into().ok())
				.map(Self::from_le_bytes)
		}
		fn binary_size(&self) -> usize {
			Self::BINARY_SIZE
		}
	}
	impl FixedBinarySize for $t {
		const BINARY_SIZE: usize = std::mem::size_of::<Self>();
	}
	)* };
}
// Special case for u8
impl BinarySerializable for u8 {
	fn to_binary(&self) -> Vec<u8> { vec![*self] }
	fn from_binary(bytes: &[u8]) -> Option<Self> { bytes.first().copied() }
	fn binary_size(&self) -> usize { 1 }
}
impl FixedBinarySize for u8 {
	const BINARY_SIZE: usize = 1;
}

impl_basic_types!(u16, u32, u64, u128, i8, i16, i32, i64, i128);

// Conversion-based implementations
macro_rules! impl_through_conversion {
	($($source:ty => $target:ty),*) => { $(
	impl BinarySerializable for $source {
		fn to_binary(&self) -> Vec<u8> {
			(*self as $target).to_binary()
		}
		fn from_binary(bytes: &[u8]) -> Option<Self> {
			<$target>::from_binary(bytes).map(|x| x as Self)
		}
		fn binary_size(&self) -> usize {
			Self::BINARY_SIZE
		}
	}
	impl FixedBinarySize for $source {
		const BINARY_SIZE: usize = std::mem::size_of::<$target>();
	}
	)* };
}
impl_through_conversion!(
	f32 => u32,
	f64 => u64,
	usize => u64,
	isize => u64
);

// Optimized NonZero implementations with Option<NonZero<T>> handling
macro_rules! impl_nonzero {
	($($t:ty),*) => { $(
	impl BinarySerializable for NonZero<$t> {
		fn to_binary(&self) -> Vec<u8> {
			self.get().to_binary()
		}
		fn from_binary(bytes: &[u8]) -> Option<Self> {
			let value = <$t>::from_binary(&bytes[0..Self::BINARY_SIZE])?;
			NonZero::<$t>::new(value)
		}
		fn binary_size(&self) -> usize {
			Self::BINARY_SIZE
		}
	}
	impl FixedBinarySize for NonZero<$t> {
		const BINARY_SIZE: usize = std::mem::size_of::<$t>();
	}
	// Optimized Option<NonZero<T>> - uses 0 value to represent None
	impl BinarySerializable for Option<NonZero<$t>> {
		fn to_binary(&self) -> Vec<u8> {
			let value = match self {
				Some(non_zero) => non_zero.get(),
				None => 0,
			};
			value.to_binary()
		}
		fn from_binary(bytes: &[u8]) -> Option<Self> {
			let value = <$t>::from_binary(&bytes[0..Self::BINARY_SIZE])?;
			match value {
				0 => Some(None),
				n => NonZero::<$t>::new(n).map(Some),
			}
		}
		fn binary_size(&self) -> usize {
			Self::BINARY_SIZE
		}
	}
	impl FixedBinarySize for Option<NonZero<$t>> {
		const BINARY_SIZE: usize = std::mem::size_of::<$t>();
	}
	)* };
}
impl_nonzero!(u8, u16, u32, u64, u128, i8, i16, i32, i64, i128);


/// Public macro for implementing BinarySerializable for Option<T>
/// Use this for any custom types that need Option serialization
/// 
/// # Example
/// ```rust
/// // After implementing BinarySerializable for your custom types:
/// impl_option_binary!(Block, Chunk, Inventory, Item, ItemStack);
/// ```
#[macro_export]
macro_rules! impl_option_binary {
	($($t:ty),* $(,)?) => { $(
	impl BinarySerializable for Option<$t> {
		fn to_binary(&self) -> Vec<u8> {
			match self {
				Some(value) => {
					let mut result = vec![1u8]; // Presence flag (1 = present)
					result.extend(value.to_binary());
					result
				}
				None => vec![0u8], // Presence flag (0 = absent)
			}
		}
		fn from_binary(bytes: &[u8]) -> Option<Self> {
			if bytes.is_empty() { return None; }
			
			let presence_flag = bytes[0];
			match presence_flag {
				0 => Some(None),
				1 => {
					let value_bytes = &bytes[1..];
					<$t>::from_binary(value_bytes).map(Some)
				}
				_ => None, // Invalid presence flag
			}
		}
		fn binary_size(&self) -> usize {
			match self {
				Some(value) => 1 + value.binary_size(), // 1 byte for flag + value size
				None => 1, // Just the flag byte
			}
		}
	}
	)* };
}

// Use the public macro for built-in types
impl_option_binary!(
	u8, u16, u32, u64, u128, 
	i8, i16, i32, i64, i128, 
	f32, f64, usize, isize,
	IVec3, UVec3, Vec3, Color, Vec2, Time, String
);

// Generic Box implementation
impl<T: BinarySerializable> BinarySerializable for Box<T> {
	fn to_binary(&self) -> Vec<u8> {
		(**self).to_binary()
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		T::from_binary(bytes).map(Box::new)
	}
	fn binary_size(&self) -> usize {
		(**self).binary_size()
	}
}

// Vector types
macro_rules! impl_vector_binary {
	($($vec_type:ident<$component_type:ty> { $($field:ident),+ }),* $(,)?) => { $(
	impl BinarySerializable for $vec_type {
		fn to_binary(&self) -> Vec<u8> {
			let mut data = Vec::new();
			$(data.extend_from_slice(&self.$field.to_binary());)+
			data
		}
		#[allow(unused_assignments)]
		fn from_binary(bytes: &[u8]) -> Option<Self> {
			let component_size = std::mem::size_of::<$component_type>();
			let field_count = [$(stringify!($field)),+].len();
			let total_size = component_size * field_count;
			
			if bytes.len() < total_size {
				return None;
			}
			
			let mut offset = 0;
			$(
				let $field = <$component_type>::from_binary(&bytes[offset..offset + component_size])?;
				offset += component_size;
			)+
			
			Some($vec_type { $($field),+ })
		}
		fn binary_size(&self) -> usize {
			Self::BINARY_SIZE
		}
	}
	impl FixedBinarySize for $vec_type {
		const BINARY_SIZE: usize = std::mem::size_of::<$component_type>() * [$(stringify!($field)),+].len();
	}
	)* };
}

impl_vector_binary! {
	IVec3<i32> { x, y, z },
	UVec3<u32> { x, y, z },
	Vec3<f32> { x, y, z },
	Color<u8> { r, g, b, a },
	Vec2<f32> { x, y },
}

/// IMPORTANT !!! the current max char length is u16 so 65K basic char, or bytes, one char is one byte usually, but here is the breakdown :
/*

let s = "aé中🦀";
println!("Bytes: {:?}", s.as_bytes());
// Output: [97, 195, 169, 228, 184, 173, 240, 159, 166, 128]
// Breakdown:
// 'a' -> 97 (1 byte)
// 'é' -> 195, 169 (2 bytes)
// '中' -> 228, 184, 173 (3 bytes)
// '🦀' -> 240, 159, 166, 128 (4 bytes) 

*/
pub const BINARY_SIZE_STRING: usize = u16::BINARY_SIZE; // for the length marker what is u16

// String implementation
impl BinarySerializable for String {
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::with_capacity(BINARY_SIZE_STRING + self.len());
		data.extend_from_slice(&(self.len() as u16).to_binary());
		data.extend_from_slice(self.as_bytes());
		data
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.len() < BINARY_SIZE_STRING {
			return None;
		}
		let len = u16::from_binary(&bytes[0..BINARY_SIZE_STRING])? as usize;
		if bytes.len() < BINARY_SIZE_STRING + len {
			return None;
		}
		
		let string_bytes = &bytes[BINARY_SIZE_STRING..BINARY_SIZE_STRING + len];
		String::from_utf8(string_bytes.to_vec()).ok()
	}
	fn binary_size(&self) -> usize {
		BINARY_SIZE_STRING + self.len()
	}
}

// &'static str implementation
impl BinarySerializable for &'static str {
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::with_capacity(BINARY_SIZE_STRING + self.len());
		data.extend_from_slice(&(self.len() as u16).to_binary());
		data.extend_from_slice(self.as_bytes());
		data
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.len() < BINARY_SIZE_STRING {
			return None;
		}
		let len = u16::from_binary(&bytes[0..BINARY_SIZE_STRING])? as usize;
		if bytes.len() < BINARY_SIZE_STRING + len {
			return None;
		}
		
		let string_bytes = &bytes[BINARY_SIZE_STRING..BINARY_SIZE_STRING + len];
		let s = std::str::from_utf8(string_bytes).ok()?;
		let leaked: &'static str = Box::leak(s.to_string().into_boxed_str());
		Some(leaked)
	}
	fn binary_size(&self) -> usize {
		BINARY_SIZE_STRING + self.len()
	}
}

// Time implementation
impl BinarySerializable for Time {
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::with_capacity(Self::BINARY_SIZE);
		data.extend_from_slice(&self.year.to_binary());
		data.push(self.month);
		data.push(self.day);
		data.push(self.hour);
		data.push(self.minute);
		data.push(self.second);
		data
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.len() < Self::BINARY_SIZE { return None; }
		let year = u16::from_binary(&bytes[0..u16::BINARY_SIZE])?;
		Some(Self {
			year,
			month: bytes[2],
			day: bytes[3],
			hour: bytes[4],
			minute: bytes[5],
			second: bytes[6],
		})
	}
	fn binary_size(&self) -> usize {
		Self::BINARY_SIZE
	}
}
impl FixedBinarySize for Time {
	const BINARY_SIZE: usize = u16::BINARY_SIZE + u8::BINARY_SIZE * 5;
}
