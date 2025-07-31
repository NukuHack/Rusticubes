use std::mem;
use std::num::NonZeroU32;
use crate::game::material::MaterialLevel;
use crate::game::items::ItemId;
use crate::game::item_lut::{
	ItemComp, ItemFlags, ItemExtendedData, PropertyValue, PropertyVariantTag,
	ToolData, ArmorData, ToolType, ArmorType, EquipmentData, EquipmentSetStruct,
	EquipmentTypeSet, BitStorage, TierStorage, EquipmentType
};
use crate::hs::binary::{BinarySerializable, FixedBinarySerializable};

impl BinarySerializable for ItemId {
	fn to_binary(&self) -> Vec<u8> {
		self.inner().to_le_bytes().to_vec()
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.len() < 2 { return None; }
		let result = u16::from_le_bytes([bytes[0], bytes[1]]);
		Some(Self(result))
	}
	fn binary_size(&self) -> usize {
		mem::size_of::<u16>()
	}
}
impl FixedBinarySerializable for ItemId {
	const BINARY_SIZE: usize = 2;
}

impl BinarySerializable for ItemFlags {
	fn to_binary(&self) -> Vec<u8> {
		self.inner().to_le_bytes().to_vec()
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.len() < 4 { return None; }
		let result = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
		Some(Self::new(result))
	}
	fn binary_size(&self) -> usize {
		mem::size_of::<u32>()
	}
}
impl FixedBinarySerializable for ItemFlags {
	const BINARY_SIZE: usize = 4;
}

// MaterialLevel serialization
impl BinarySerializable for MaterialLevel {
	fn to_binary(&self) -> Vec<u8> {
		vec![*self as u8]
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.is_empty() { return None; }
		// Assuming MaterialLevel can be safely transmuted from u8
		Self::from_u8(bytes[0])
	}
	fn binary_size(&self) -> usize {
		1
	}
}
impl FixedBinarySerializable for MaterialLevel {
	const BINARY_SIZE: usize = 1;
}

// ToolType and ArmorType serialization
impl BinarySerializable for ToolType {
	fn to_binary(&self) -> Vec<u8> {
		vec![*self as u8]
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.is_empty() { return None; }
		Self::from_u8(bytes[0])
	}
	fn binary_size(&self) -> usize {
		1
	}
}
impl FixedBinarySerializable for ToolType {
	const BINARY_SIZE: usize = 1;
}

impl BinarySerializable for ArmorType {
	fn to_binary(&self) -> Vec<u8> {
		vec![*self as u8]
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.is_empty() { return None; }
		Self::from_u8(bytes[0])
	}
	fn binary_size(&self) -> usize {
		1
	}
}
impl FixedBinarySerializable for ArmorType {
	const BINARY_SIZE: usize = 1;
}

// PropertyVariantTag serialization
impl BinarySerializable for PropertyVariantTag {
	fn to_binary(&self) -> Vec<u8> {
		vec![*self as u8]
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.is_empty() { return None; }
		Self::from_u8(bytes[0]) // already returns an option
	}
	fn binary_size(&self) -> usize {
		1
	}
}
impl FixedBinarySerializable for PropertyVariantTag {
	const BINARY_SIZE: usize = 1;
}



// Add BinarySerializable implementation for EquipmentSetStruct
impl<T, S, TS> BinarySerializable for EquipmentSetStruct<T, S, TS>
where
	T: EquipmentType,
	S: BitStorage + BinarySerializable + FixedBinarySerializable,
	TS: TierStorage + BinarySerializable + FixedBinarySerializable,
{
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::new();
		data.extend_from_slice(&self.types.to_binary());
		data.extend_from_slice(&self.tiers.to_binary());
		data
	}
	
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.len() < S::BINARY_SIZE + TS::BINARY_SIZE {
			return None;
		}
		
		let types = EquipmentTypeSet::from_binary(&bytes[0..S::BINARY_SIZE])?;
		let tiers = TS::from_binary(&bytes[S::BINARY_SIZE..S::BINARY_SIZE + TS::BINARY_SIZE])?;
		
		Some(Self { types, tiers })
	}
	
	fn binary_size(&self) -> usize {
		S::BINARY_SIZE + TS::BINARY_SIZE
	}
}


// EquipmentTypeSet serialization
impl<T: EquipmentType, S: BitStorage> BinarySerializable for EquipmentTypeSet<T, S>
where 
	S: BinarySerializable + FixedBinarySerializable
{
	fn to_binary(&self) -> Vec<u8> {
		self.slots.to_binary()
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		let slots = S::from_binary(bytes)?;
		Some(Self {
			slots,
			_phantom: std::marker::PhantomData,
		})
	}
	fn binary_size(&self) -> usize {
		S::BINARY_SIZE
	}
}

// EquipmentData serialization
// Fix EquipmentData serialization with proper trait bounds
impl<T, S> BinarySerializable for EquipmentData<T, S>
where
	T: EquipmentType + BinarySerializable + FixedBinarySerializable,
	S: BinarySerializable,
{
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::new();
		
		match self {
			EquipmentData::None => {
				data.push(0); // Variant tag
			}
			EquipmentData::Single { equip_type, tier } => {
				data.push(1); // Variant tag
				data.extend_from_slice(&equip_type.to_binary());
				data.extend_from_slice(&tier.to_binary());
			}
			EquipmentData::Multiple(set) => {
				data.push(2); // Variant tag
				let set_data = set.to_binary();
				// Store size as u32 little-endian
				data.extend_from_slice(&(set_data.len() as u32).to_le_bytes());
				data.extend_from_slice(&set_data);
			}
		}
		
		data
	}
	
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.is_empty() { return None; }
		
		match bytes[0] {
			0 => Some(EquipmentData::None),
			1 => {
				if bytes.len() < 1 + T::BINARY_SIZE + MaterialLevel::BINARY_SIZE {
					return None;
				}
				let equip_type = T::from_binary(&bytes[1..1 + T::BINARY_SIZE])?;
				let tier = MaterialLevel::from_binary(&bytes[1 + T::BINARY_SIZE..1 + T::BINARY_SIZE + MaterialLevel::BINARY_SIZE])?;
				Some(EquipmentData::Single { equip_type, tier })
			}
			2 => {
				if bytes.len() < 5 { return None; } // 1 byte tag + 4 bytes size
				let size = u32::from_le_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]) as usize;
				if bytes.len() < 5 + size { return None; }
				let set = S::from_binary(&bytes[5..5 + size])?;
				Some(EquipmentData::Multiple(set))
			}
			_ => None,
		}
	}
	
	fn binary_size(&self) -> usize {
		match self {
			EquipmentData::None => 1,
			EquipmentData::Single { .. } => 1 + T::BINARY_SIZE + MaterialLevel::BINARY_SIZE,
			EquipmentData::Multiple(set) => 1 + 4 + set.binary_size(),
		}
	}
}

// Fix PropertyValue serialization - correct variable name
impl BinarySerializable for PropertyValue {
	fn to_binary(&self) -> Vec<u8> {
		let mut data: Vec<u8> = Vec::new();
		let tag = self.to_tag().to_binary();
		
		data.extend_from_slice(&tag);
		match self {
			PropertyValue::Durability(value) => {
				data.extend_from_slice(&value.get().to_binary());
			}
			PropertyValue::ToolData(tool_data) => {
				data.extend_from_slice(&tool_data.to_binary());
			}
			PropertyValue::ArmorData(armor_data) => {
				data.extend_from_slice(&armor_data.to_binary());
			}
			PropertyValue::Hunger(value) => {
				data.extend_from_slice(&value.to_binary());
			}
			PropertyValue::ArmorValue(value) => {
				data.extend_from_slice(&value.to_binary());
			}
			PropertyValue::Damage(value) => {
				data.extend_from_slice(&value.to_binary());
			}
			PropertyValue::Speed(value) => {
				data.extend_from_slice(&value.to_binary());
			}
		}
		
		data
	}
	
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.is_empty() { return None; }
		let tag = PropertyVariantTag::from_u8(*bytes.first()?)?;
		let rest_bytes = &bytes[PropertyVariantTag::BINARY_SIZE..]; // Fixed typo: rest_bites -> rest_bytes
		#[allow(unreachable_patterns)]
		match tag {
			PropertyVariantTag::Durability => {
				let value = u32::from_binary(rest_bytes)?;
				let non_zero = NonZeroU32::new(value)?;
				Some(PropertyValue::Durability(non_zero))
			}
			PropertyVariantTag::ToolData => {
				let tool_data = ToolData::from_binary(rest_bytes)?;
				Some(PropertyValue::ToolData(tool_data))
			}
			PropertyVariantTag::ArmorData => {
				let armor_data = ArmorData::from_binary(rest_bytes)?;
				Some(PropertyValue::ArmorData(armor_data))
			}
			PropertyVariantTag::Hunger => {
				let value = i16::from_binary(rest_bytes)?;
				Some(PropertyValue::Hunger(value))
			}
			PropertyVariantTag::ArmorValue => {
				let value = i16::from_binary(rest_bytes)?;
				Some(PropertyValue::ArmorValue(value))
			}
			PropertyVariantTag::Damage => {
				let value = i16::from_binary(rest_bytes)?;
				Some(PropertyValue::Damage(value))
			}
			PropertyVariantTag::Speed => {
				let value = i16::from_binary(rest_bytes)?;
				Some(PropertyValue::Speed(value))
			}
			_ => None,
		}
	}
	
	fn binary_size(&self) -> usize {
		let size = match self {
			PropertyValue::Durability(_) => u32::BINARY_SIZE,
			PropertyValue::ToolData(tool_data) => tool_data.binary_size(),
			PropertyValue::ArmorData(armor_data) => armor_data.binary_size(),
			PropertyValue::Hunger(_) => i16::BINARY_SIZE,
			PropertyValue::ArmorValue(_) => i16::BINARY_SIZE,
			PropertyValue::Damage(_) => i16::BINARY_SIZE,
			PropertyValue::Speed(_) => i16::BINARY_SIZE,
		};
		PropertyVariantTag::BINARY_SIZE + size
	}
}

// ItemExtendedData serialization
impl<const N: usize> BinarySerializable for ItemExtendedData<N> {
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::new();
		
		// Store the number of properties
		data.push(self.len);
		
		// Store each property
		for i in 0..self.len as usize {
			if let Some(property) = &self.data[i] {
				data.extend_from_slice(&property.to_binary());
			}
		}
		
		data
	}
	
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.is_empty() { return None; }
		
		let len = bytes[0];
		if len as usize > N { return None; }
		
		let mut result = Self::new();
		result.len = len;
		
		let mut offset = 1;
		for i in 0..len as usize {
			if offset >= bytes.len() { return None; }
			
			let property = PropertyValue::from_binary(&bytes[offset..])?;
			let property_size = property.binary_size();
			
			result.data[i] = Some(property);
			offset += property_size;
		}
		
		Some(result)
	}
	
	fn binary_size(&self) -> usize {
		let mut size = 1; // For the length byte
		for i in 0..self.len as usize {
			if let Some(property) = &self.data[i] {
				size += property.binary_size();
			}
		}
		size
	}
}

// ItemComp serialization (excluding the name field since it's a static reference)
type StatString = &'static str;
const BINARY_SIZE_STAT_STRING: usize = 2;
impl BinarySerializable for ItemComp {
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::new();
		
		// Serialize basic fields
		data.extend_from_slice(&self.id.to_binary());
		data.extend_from_slice(&self.max_stack.to_le_bytes());
		data.extend_from_slice(&self.flags.to_binary());
		data.extend_from_slice(&self.name.to_binary());
		
		// Serialize extended data
		match &self.data {
			Some(extended_data) => {
				data.push(1); // Has data flag
				data.extend_from_slice(&extended_data.to_binary());
			}
			None => {
				data.push(0); // No data flag
			}
		}
		
		data
	}
	
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.len() < 15 { return None; } // Minimum size: 2 + 4 + 4 + 2 + 1
		let mut offset = 0;
		
		// Deserialize id (2 bytes)
		let id = ItemId::from_binary(&bytes[offset..offset+2])?;
		offset += 2;
		// Deserialize max_stack (4 bytes)
		let max_stack = u32::from_binary(&bytes[offset..offset+4])?;
		offset += 4;
		// Deserialize flags (2 bytes)
		let flags = ItemFlags::from_binary(&bytes[offset..offset+4])?;
		offset += 4;

		let name_len = u16::from_binary(&bytes[offset..offset+BINARY_SIZE_STAT_STRING])? as usize;
		let name:StatString = StatString::from_binary(&bytes[offset..offset+BINARY_SIZE_STAT_STRING+name_len])?;
		offset += BINARY_SIZE_STAT_STRING + name_len;

		// Deserialize extended data
		let has_data = bytes[offset] != 0;
		offset += 1;
		let data = if has_data {
			Some(ItemExtendedData::from_binary(&bytes[offset..])?)
		} else {
			None
		};
		
		Some(ItemComp {
			id,
			name,
			max_stack,
			flags,
			data,
		})
	}
	
	fn binary_size(&self) -> usize {
		let mut size = 2 + 4 + 4; // id + max_stack + flags
		size += self.name.binary_size(); // name
		size += 1; // has_data flag
		if let Some(data) = &self.data {
			size += data.binary_size();
		}
		size
	}
}

