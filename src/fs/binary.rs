
use crate::utils::time::Time;
use glam::IVec3;

// Trait for binary serialization

pub trait BinarySerializable {
	fn to_binary(&self) -> Vec<u8>;
	fn from_binary(bytes: &[u8]) -> Option<Self> where Self: Sized;
	fn binary_size(&self) -> usize;
}

// Fixed-size serialization trait for types with known sizes
pub trait FixedBinarySerializable: BinarySerializable {
	const BINARY_SIZE: usize;
}


// serialize basic types



// BitStorage implementations
impl BinarySerializable for u8 {
	fn to_binary(&self) -> Vec<u8> {
		vec![*self]
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		bytes.first().copied()
	}
	fn binary_size(&self) -> usize {
		Self::BINARY_SIZE
	}
}
impl FixedBinarySerializable for u8 {
	const BINARY_SIZE: usize = 1;
}
impl BinarySerializable for u16 {
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
impl FixedBinarySerializable for u16 {
	const BINARY_SIZE: usize = 2;
}
impl BinarySerializable for u32 {
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
impl FixedBinarySerializable for u32 {
	const BINARY_SIZE: usize = 4;
}
impl BinarySerializable for u64 {
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
impl FixedBinarySerializable for u64 {
	const BINARY_SIZE: usize = 8;
}
impl BinarySerializable for u128 {
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
impl FixedBinarySerializable for u128 {
	const BINARY_SIZE: usize = 16;
}


// Usize kind of stuff



impl BinarySerializable for usize {
	fn to_binary(&self) -> Vec<u8> {
		self.to_le_bytes().to_vec()
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		bytes.get(..Self::BINARY_SIZE)
			.and_then(|slice| slice.try_into().ok())
			.map(Self::from_le_bytes)
	}
	fn binary_size(&self) -> Self {
		Self::BINARY_SIZE
	}
}
impl FixedBinarySerializable for usize {
	const BINARY_SIZE: Self = u64::BINARY_SIZE; // assuming this is a 64bit architecture
}
impl BinarySerializable for isize {
	fn to_binary(&self) -> Vec<u8> {
		(*self as usize).to_binary()
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		usize::from_binary(bytes).map(|x| x as isize)
	}
	fn binary_size(&self) -> usize {
		Self::BINARY_SIZE
	}
}
impl FixedBinarySerializable for isize {
	const BINARY_SIZE: usize = u64::BINARY_SIZE; // assuming this is a 64bit architecture
}



// signed int-s



impl BinarySerializable for i16 {
	fn to_binary(&self) -> Vec<u8> {
		(*self as u16).to_binary()
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		u16::from_binary(bytes).map(|x| x as i16)
	}
	fn binary_size(&self) -> usize {
		Self::BINARY_SIZE
	}
}
impl FixedBinarySerializable for i16 {
	const BINARY_SIZE: usize = 2;
}
impl BinarySerializable for i32 {
	fn to_binary(&self) -> Vec<u8> {
		(*self as u32).to_binary()
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		u32::from_binary(bytes).map(|x| x as i32)
	}
	fn binary_size(&self) -> usize {
		Self::BINARY_SIZE
	}
}
impl FixedBinarySerializable for i32 {
	const BINARY_SIZE: usize = 4;
}




// some glam types
impl BinarySerializable for IVec3 {
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::new();
		data.extend_from_slice(&self.x.to_binary());
		data.extend_from_slice(&self.y.to_binary());
		data.extend_from_slice(&self.z.to_binary());
		data
	}
	
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.len() < 12 {
			return None;
		}
		let x = i32::from_binary(&bytes[0..4])?;
		let y = i32::from_binary(&bytes[4..8])?;
		let z = i32::from_binary(&bytes[8..12])?;
		Some(IVec3::new(x, y, z))
	}
	
	fn binary_size(&self) -> usize {
		12
	}
}

impl FixedBinarySerializable for IVec3 {
	const BINARY_SIZE: usize = 12;
}


// Yeah boy ... rewrite everything from scratch ... like strings

/// IMPORTANT !!! the current max char leng is u16 so 60K basic char, or bytes, one char is one byte usually, but here is the breakdown :
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

impl BinarySerializable for String {
	fn to_binary(&self) -> Vec<u8> {
		string_to_binary(self)
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		let s = string_from_binary(bytes)?;
		Some(s.to_string())
	}
	fn binary_size(&self) -> usize {
		string_binary_size(self)
	}
}

type StatString = &'static str;
impl BinarySerializable for StatString {
	fn to_binary(&self) -> Vec<u8> {
		string_to_binary(self)
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		let s = string_from_binary(bytes)?;
		let leaked: &'static str = Box::leak(s.to_string().clone().into_boxed_str());
		Some(leaked)
	}
	fn binary_size(&self) -> usize {
		string_binary_size(self)
	}
}
pub const BINARY_SIZE_STRING: usize = 2;

fn string_to_binary(s: &str) -> Vec<u8> {
	let mut data = Vec::with_capacity(BINARY_SIZE_STRING + s.len()); // Use 2 bytes for length to handle longer strings
	data.extend_from_slice(&(s.len() as u16).to_binary());
	data.extend_from_slice(s.as_bytes());
	data
}
fn string_from_binary(bytes: &[u8]) -> Option<&str> {
	if bytes.len() < BINARY_SIZE_STRING {
		return None;
	}
	let len = u16::from_binary(&bytes[0..u16::BINARY_SIZE])? as usize;
	if bytes.len() < BINARY_SIZE_STRING + len {
		return None;
	}
	
	std::str::from_utf8(&bytes[BINARY_SIZE_STRING..BINARY_SIZE_STRING + len]).ok()
}
fn string_binary_size(s: &str) -> usize {
	BINARY_SIZE_STRING + s.len() // 2 bytes for length + string bytes
}


// extra 



// Implement BinarySerializable for Time using the trait pattern
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
		if bytes.len() < Self::BINARY_SIZE {
			return None;
		}
		let mut offset:usize = 0;
		let year = u16::from_binary(&bytes[offset..offset+u16::BINARY_SIZE])?; offset += u16::BINARY_SIZE;
		let month = bytes[offset]; offset += 1;
		let day = bytes[offset]; offset += 1;
		let hour = bytes[offset]; offset += 1;
		let minute = bytes[offset]; offset += 1;
		let second = bytes[offset];
		
		Some(Self {
			year,
			month,
			day,
			hour,
			minute,
			second,
		})
	}
	fn binary_size(&self) -> usize {
		Self::BINARY_SIZE
	}
}
impl FixedBinarySerializable for Time {
	const BINARY_SIZE: usize = 7; // 2 for year ; month, day, hour, minute, second each get 1
}


