

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



// BitStorage implementations for u8 and u16
impl BinarySerializable for u8 {
	fn to_binary(&self) -> Vec<u8> {
		vec![*self]
	}
	
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.is_empty() { return None; }
		Some(bytes[0])
	}
	
	fn binary_size(&self) -> usize {
		1
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
		if bytes.len() < 2 { return None; }
		Some(u16::from_le_bytes([bytes[0], bytes[1]]))
	}
	fn binary_size(&self) -> usize {
		2
	}
}
impl FixedBinarySerializable for u16 {
	const BINARY_SIZE: usize = 2;
}
impl BinarySerializable for i16 {
	fn to_binary(&self) -> Vec<u8> {
		(*self as u16).to_binary()
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		let f = u16::from_binary(bytes);
		if let Some(x) = f {
			return Some(x as i16);
		}
		None
	}
	fn binary_size(&self) -> usize {
		2
	}
}
impl FixedBinarySerializable for i16 {
	const BINARY_SIZE: usize = 2;
}
impl BinarySerializable for u32 {
	fn to_binary(&self) -> Vec<u8> {
		self.to_le_bytes().to_vec()
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.len() < 4 { return None; }
		Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
	}
	fn binary_size(&self) -> usize {
		4
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
		if bytes.len() < 8 { return None; }
		Some(u64::from_le_bytes([
			bytes[0], bytes[1], bytes[2], bytes[3],
			bytes[4], bytes[5], bytes[6], bytes[7]
			]))
	}
	fn binary_size(&self) -> usize {
		8
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
		if bytes.len() < 16 { return None; }
		Some(u128::from_le_bytes([
			bytes[0], bytes[1], bytes[2], bytes[3],
			bytes[4], bytes[5], bytes[6], bytes[7],
			bytes[8], bytes[9], bytes[10], bytes[11],
			bytes[12], bytes[13], bytes[14], bytes[15]
			]))
	}
	fn binary_size(&self) -> usize {
		16
	}
}
impl FixedBinarySerializable for u128 {
	const BINARY_SIZE: usize = 16;
}



// Yeah boy ... rewrite everything from scratch ... like strings



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
const BINARY_SIZE_STAT_STRING: usize = 2;

fn string_to_binary(s: &str) -> Vec<u8> {
	let mut data = Vec::with_capacity(BINARY_SIZE_STAT_STRING + s.len()); // Use 2 bytes for length to handle longer strings
	data.extend_from_slice(&(s.len() as u16).to_le_bytes());
	data.extend_from_slice(s.as_bytes());
	data
}

fn string_from_binary(bytes: &[u8]) -> Option<&str> {
	if bytes.len() < BINARY_SIZE_STAT_STRING {
		return None;
	}
	
	let len = u16::from_le_bytes([bytes[0], bytes[1]]) as usize;
	if bytes.len() < BINARY_SIZE_STAT_STRING + len {
		return None;
	}
	
	std::str::from_utf8(&bytes[BINARY_SIZE_STAT_STRING..BINARY_SIZE_STAT_STRING + len]).ok()
}

fn string_binary_size(s: &str) -> usize {
	BINARY_SIZE_STAT_STRING + s.len() // 2 bytes for length + string bytes
}