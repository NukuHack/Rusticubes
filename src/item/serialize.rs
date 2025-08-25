
use crate::item::items::{ItemStack, CustomData};
use crate::item::inventory::{Inventory, ItemContainer, Slot, AreaType};
use crate::item::material::{ArmorType, ToolType, MaterialLevel, EquipmentType, BasicConversion};
use crate::item::item_lut::{
	ItemComp, ItemFlags, ItemExtendedData, PropertyValue, PropertyType,
	ToolData, ArmorData, EquipmentData, EquipmentSetStruct, BitStorage, TierStorage
};
use crate::fs::binary::{BinarySerializable, FixedBinarySerializable, BINARY_SIZE_STRING};
use std::num::{NonZeroU16, NonZeroU32};

impl BinarySerializable for ItemFlags {
	fn to_binary(&self) -> Vec<u8> {
		self.inner().to_le_bytes().to_vec()
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.len() < u32::BINARY_SIZE { return None; }
		let result = u32::from_binary(&bytes[0..u32::BINARY_SIZE])?;
		Some(Self::new(result))
	}
	fn binary_size(&self) -> usize {
		Self::BINARY_SIZE
	}
}
impl FixedBinarySerializable for ItemFlags {
	const BINARY_SIZE: usize = u32::BINARY_SIZE;
}

// MaterialLevel serialization
impl BinarySerializable for MaterialLevel {
	fn to_binary(&self) -> Vec<u8> {
		vec![self.to_u8()]
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		Self::from_u8(*bytes.first()?)
	}
	fn binary_size(&self) -> usize {
		Self::BINARY_SIZE
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
		Self::from_u8(*bytes.first()?)
	}
	fn binary_size(&self) -> usize {
		Self::BINARY_SIZE
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
		Self::from_u8(*bytes.first()?)
	}
	fn binary_size(&self) -> usize {
		Self::BINARY_SIZE
	}
}
impl FixedBinarySerializable for ArmorType {
	const BINARY_SIZE: usize = 1;
}
// PropertyType serialization
impl BinarySerializable for PropertyType {
	fn to_binary(&self) -> Vec<u8> {
		vec![*self as u8]
	}
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		Self::from_u8(*bytes.first()?)
	}
	fn binary_size(&self) -> usize {
		Self::BINARY_SIZE
	}
}
impl FixedBinarySerializable for PropertyType {
	const BINARY_SIZE: usize = 1;
}



// Add BinarySerializable implementation for EquipmentSetStruct
impl<T, S, TS> BinarySerializable for EquipmentSetStruct<T, S, TS>
where
	T: EquipmentType + BasicConversion<T>,
	S: BitStorage + BinarySerializable + FixedBinarySerializable,
	TS: TierStorage + BinarySerializable + FixedBinarySerializable,
{
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::new();
		data.extend_from_slice(&self.slots.to_binary());
		data.extend_from_slice(&self.tiers.to_binary());
		data
	}
	
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.len() < S::BINARY_SIZE + TS::BINARY_SIZE {
			return None;
		}
		
		let slots = S::from_binary(&bytes[0..S::BINARY_SIZE])?;
		let tiers = TS::from_binary(&bytes[S::BINARY_SIZE..S::BINARY_SIZE + TS::BINARY_SIZE])?;
		let mut out = EquipmentSetStruct::<T, S, TS>::new();
		out.slots = slots;
		out.tiers = tiers;
		
		Some(out)
	}
	
	fn binary_size(&self) -> usize {
		S::BINARY_SIZE + TS::BINARY_SIZE
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
			EquipmentData::Single { equip_type, tier } => {
				data.push(1); // Variant tag
				data.extend_from_slice(&equip_type.to_binary());
				data.extend_from_slice(&tier.to_binary());
			}
			EquipmentData::Multiple(set) => {
				data.push(2); // Variant tag
				let set_data = set.to_binary();
				data.extend_from_slice(&set_data);
			}
		}
		
		data
	}
	
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		match bytes.first()? {
			1 => {
				if bytes.len() < 1 + T::BINARY_SIZE + MaterialLevel::BINARY_SIZE {
					return None;
				}
				let equip_type = T::from_binary(&bytes[1..1 + T::BINARY_SIZE])?;
				let tier = MaterialLevel::from_binary(&bytes[1 + T::BINARY_SIZE..1 + T::BINARY_SIZE + MaterialLevel::BINARY_SIZE])?;
				Some(EquipmentData::Single { equip_type, tier })
			}
			2 => {
				let set = S::from_binary(&bytes[1..])?;
				Some(EquipmentData::Multiple(set))
			}
			_ => None,
		}
	}
	
	fn binary_size(&self) -> usize {
		match self {
			EquipmentData::Single { .. } => 1 + T::BINARY_SIZE + MaterialLevel::BINARY_SIZE,
			EquipmentData::Multiple(set) => 1 + set.binary_size(),
		}
	}
}

// Fix PropertyValue serialization - correct variable name
impl BinarySerializable for PropertyValue {
	fn to_binary(&self) -> Vec<u8> {
		let mut data: Vec<u8> = Vec::new();
		let tag = self.to_type().to_binary();
		
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
			},
			PropertyValue::Slot(value) => {
				data.push(value.rows());
				data.push(value.cols());
			}
		}
		
		data
	}
	
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.is_empty() { return None; }
		let tag = PropertyType::from_u8(*bytes.first()?)?;
		let rest_bytes = &bytes[PropertyType::BINARY_SIZE..];
		match tag {
			PropertyType::Durability => {
				let value = u32::from_binary(rest_bytes)?;
				let non_zero = NonZeroU32::new(value)?;
				Some(PropertyValue::Durability(non_zero))
			}
			PropertyType::ToolData => {
				let tool_data = ToolData::from_binary(rest_bytes)?;
				Some(PropertyValue::ToolData(tool_data))
			}
			PropertyType::ArmorData => {
				let armor_data = ArmorData::from_binary(rest_bytes)?;
				Some(PropertyValue::ArmorData(armor_data))
			}
			PropertyType::Hunger => {
				let value = i16::from_binary(rest_bytes)?;
				Some(PropertyValue::Hunger(value))
			}
			PropertyType::ArmorValue => {
				let value = i16::from_binary(rest_bytes)?;
				Some(PropertyValue::ArmorValue(value))
			}
			PropertyType::Damage => {
				let value = i16::from_binary(rest_bytes)?;
				Some(PropertyValue::Damage(value))
			}
			PropertyType::Speed => {
				let value = i16::from_binary(rest_bytes)?;
				Some(PropertyValue::Speed(value))
			},
			PropertyType::Slot => {
				let row = rest_bytes[0];
				let col = rest_bytes[1];
				Some(PropertyValue::Slot((row, col).into()))
			}
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
			PropertyValue::Slot(_) => u8::BINARY_SIZE * 2,
		};
		PropertyType::BINARY_SIZE + size
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

impl BinarySerializable for ItemComp {
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::new();
		
		// Serialize basic fields
		data.extend_from_slice(&self.name.clone().to_string().to_binary());
		data.extend_from_slice(&self.max_stack.to_binary());
		data.extend_from_slice(&self.flags.to_binary());
		
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
		if bytes.len() < 10 { return None; } // Minimum size
		let mut offset = 0;
		
		// Deserialize name
		let name_len = u16::from_binary(&bytes[offset..offset+BINARY_SIZE_STRING])? as usize;
		let name = <&'static str>::from_binary(&bytes[offset..offset+BINARY_SIZE_STRING+name_len])?;
		offset += BINARY_SIZE_STRING + name_len;

		// Deserialize max_stack (4 bytes)
		let max_stack = u32::from_binary(&bytes[offset..offset+u32::BINARY_SIZE])?;
		offset += u32::BINARY_SIZE;
		// Deserialize flags (2 bytes)
		let flags = ItemFlags::from_binary(&bytes[offset..offset+ItemFlags::BINARY_SIZE])?;
		offset += ItemFlags::BINARY_SIZE;

		// Deserialize extended data
		let has_data = bytes[offset] != 0;
		offset += 1;
		let data = if has_data {
			Some(ItemExtendedData::from_binary(&bytes[offset..])?)
		} else {
			None
		};
		
		Some(ItemComp {
			name: name.into(),
			max_stack,
			flags,
			data,
		})
	}
	
	fn binary_size(&self) -> usize {
		let mut size = self.name.clone().to_string().binary_size(); // name
		size += u32::BINARY_SIZE + ItemFlags::BINARY_SIZE; // max_stack + flags
		size += 1; // has_data flag
		if let Some(data) = &self.data {
			size += data.binary_size();
		}
		size
	}
}

impl BinarySerializable for CustomData {
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::new();
		
		// Serialize name (1 byte flag + optional string)
		if let Some(name) = &self.name {
			data.push(1); // Has name flag
			data.extend_from_slice(&name.to_binary());
		} else {
			data.push(0); // No name flag
		}
		
		// Serialize durability (1 byte flag + optional u16)
		if let Some(durability) = &self.durability {
			data.push(1); // Has durability flag
			data.extend_from_slice(&durability.get().to_binary());
		} else {
			data.push(0); // No durability flag
		}
		
		data
	}
	
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.len() < 2 { return None; } // Minimum 2 bytes for flags
		
		let mut offset = 0;
		
		// Deserialize name
		let has_name = bytes[offset] != 0;
		offset += 1;
		let name = if has_name {
			let name_len = u16::from_binary(&bytes[offset..offset+2])? as usize;
			let name = String::from_binary(&bytes[offset..offset+name_len+2])?;
			offset += name_len + 2;
			Some(name)
		} else {
			None
		};
		
		// Deserialize durability
		let has_durability = bytes[offset] != 0;
		offset += 1;
		let durability = if has_durability {
			if bytes.len() < offset + 2 { return None; }
			let value = u16::from_binary(&bytes[offset..offset+2])?;
			//offset += 2;
			NonZeroU16::new(value)
		} else {
			None
		};
		
		Some(CustomData {
			name,
			durability,
		})
	}
	
	fn binary_size(&self) -> usize {
		let mut size = 2; // Flags for name and durability
		
		if let Some(name) = &self.name {
			size += 2 + name.len(); // 2 bytes for length + string bytes
		}
		
		if self.durability.is_some() {
			size += 2; // u16 for durability
		}
		
		size
	}
}

impl BinarySerializable for ItemStack {
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::new();
		
		// Serialize name (string)
		data.extend_from_slice(&self.name().to_string().to_binary());
		
		// Serialize stack count (4 bytes)
		data.extend_from_slice(&self.stack.to_binary());
		
		// Serialize custom data (1 byte flag + optional data)
		if let Some(custom_data) = &self.data {
			data.push(1); // Has data flag
			data.extend_from_slice(&custom_data.to_binary());
		} else {
			data.push(0); // No data flag
		}
		
		data
	}
	
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.is_empty() { return None; }
		
		let mut offset = 0;
		
		// Deserialize name
		let name_len = u16::from_binary(&bytes[offset..offset+2])? as usize;
		let name = String::from_binary(&bytes[offset..offset+name_len+2])?;
		offset += name_len + 2;
		
		// Deserialize stack count
		if offset >= bytes.len() { return None; }
		let stack = u32::from_binary(&bytes[offset..offset+u32::BINARY_SIZE])?;
		offset += u32::BINARY_SIZE;
		
		// Deserialize custom data
		if offset >= bytes.len() { return None; }
		let has_data = bytes[offset] != 0;
		offset += 1;
		let data = if has_data {
			Some(Box::new(CustomData::from_binary(&bytes[offset..])?))
		} else {
			None
		};
		
		Some(ItemStack::create(name,stack,data))
	}
	
	fn binary_size(&self) -> usize {
		let mut size = self.name().to_string().binary_size(); // string bytes
		size += u32::BINARY_SIZE; // stack count
		size += 1; // has_data flag (1 byte)
		if let Some(data) = &self.data {
			size += data.binary_size();
		}
		size
	}
}


// Implement BinarySerializable for Slot
impl BinarySerializable for Slot {
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::new();
		data.push(self.rows());
		data.push(self.cols());
		data
	}
	
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.len() < 2 { return None; }
		Some(Slot::custom(bytes[0],bytes[1]))
	}
	
	fn binary_size(&self) -> usize {
		Self::BINARY_SIZE
	}
}

impl FixedBinarySerializable for Slot {
	const BINARY_SIZE: usize = 2; // rows (1 byte) + cols (1 byte)
}

// Implement BinarySerializable for AreaType
impl BinarySerializable for AreaType {
	fn to_binary(&self) -> Vec<u8> {
		vec![*self as u8]
	}
	
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		match bytes.first()? {
			0 => Some(AreaType::Panel),
			1 => Some(AreaType::Inventory),
			2 => Some(AreaType::Hotbar),
			3 => Some(AreaType::Armor),
			4 => Some(AreaType::Storage),
			5 => Some(AreaType::Output),
			_ => None,
		}
	}
	
	fn binary_size(&self) -> usize {
		1
	}
}

impl FixedBinarySerializable for AreaType {
	const BINARY_SIZE: usize = 1;
}

// Implement BinarySerializable for ItemContainer
impl BinarySerializable for ItemContainer {
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::new();
		
		// Serialize dimensions
		data.extend_from_slice(&self.size().to_binary());
		
		// Serialize items
		data.extend_from_slice(&self.items().len().to_binary());
		for item in self.iter() {
			if let Some(item_stack) = item {
				data.push(1); // Has item flag
				data.extend_from_slice(&item_stack.to_binary());
			} else {
				data.push(0); // Empty slot flag
			}
		}
		
		data
	}
	
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.is_empty() { return None; }
		let mut offset = 0;
		
		// Deserialize dimensions
		let size = Slot::from_binary(&bytes[offset..offset + Slot::BINARY_SIZE])?;
		offset += Slot::BINARY_SIZE;
		
		// Deserialize item count
		if offset >= bytes.len() { return None; }
		let item_count = usize::from_binary(&bytes[offset..offset + usize::BINARY_SIZE])?;
		offset += usize::BINARY_SIZE;
		
		// Check if we have enough bytes
		if bytes.len() < offset + item_count { return None; }
		
		let mut items = Vec::with_capacity(item_count);
		
		// Deserialize items
		for _ in 0..item_count {
			if offset >= bytes.len() { return None; }
			
			let has_item = bytes[offset] != 0;
			offset += 1;
			
			if has_item {
				let item_stack = ItemStack::from_binary(&bytes[offset..])?;
				offset += item_stack.binary_size();
				items.push(Some(item_stack));
			} else {
				items.push(None);
			}
		}
		
		Some(ItemContainer::from_raw(size, items))
	}
	
	fn binary_size(&self) -> usize {
		let mut size = Slot::BINARY_SIZE; // size
		size += usize::BINARY_SIZE; // item count
		
		for item in self.iter() {
			size += 1; // has_item flag
			if let Some(item_stack) = item {
				size += item_stack.binary_size();
			}
		}
		
		size
	}
}

// Implement BinarySerializable for Inventory
impl BinarySerializable for Inventory {
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::new();
				
		// Serialize containers
		data.extend_from_slice(&self.armor().to_binary());
		data.extend_from_slice(&self.inv().to_binary());
		data.extend_from_slice(&self.hotbar().to_binary());
		data.extend_from_slice(&self.crafting_def.to_binary());
		
		// Serialize layout (skip for now as it's complex UI data)
		data.push(0); // Layout flag (not serialized)
		
		// Serialize storage pointer (we can't serialize raw pointers, so we'll use a marker)
		data.push(if self.storage_ptr.is_some() { 1 } else { 0 });
		
		data
	}
	
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.is_empty() { return None; }
		let mut offset = 0;
		
		// Deserialize containers
		let armor = ItemContainer::from_binary(&bytes[offset..])?;
		offset += armor.binary_size();
		
		if offset >= bytes.len() { return None; }
		let items = ItemContainer::from_binary(&bytes[offset..])?;
		offset += items.binary_size();
		
		if offset >= bytes.len() { return None; }
		let hotbar = ItemContainer::from_binary(&bytes[offset..])?;
		offset += hotbar.binary_size();
		
		if offset >= bytes.len() { return None; }
		let crafting_def = ItemContainer::from_binary(&bytes[offset..])?;
		offset += crafting_def.binary_size();
				
		// Skip layout (1 byte flag)
		if offset >= bytes.len() { return None; }
		offset += 1;
		
		// Deserialize storage pointer flag
		if offset >= bytes.len() { return None; }
		let has_storage_ptr = bytes[offset] != 0;
		//offset += 1;
		
		Some(Inventory::from_raw(
			armor,
			items,
			hotbar,
			crafting_def,
			if has_storage_ptr {
				// We can't reconstruct the pointer, so we'll set it to null
				// In a real implementation, you might want to handle this differently
				Some(std::ptr::null_mut())
			} else {
				None
			}
		))
	}
	
	fn binary_size(&self) -> usize {
		let mut size = 0;
		
		size += self.armor().binary_size();
		size += self.inv().binary_size();
		size += self.hotbar().binary_size();
		size += self.crafting_def.binary_size();
		
		size += 1; // layout flag (not serialized)
		size += 1; // storage pointer flag
		
		size
	}
}
