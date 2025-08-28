
use crate::item::filter::ItemFilter;
use crate::item::items::{ItemStack, CustomData};
use crate::item::inventory::{Inventory, ItemContainer, Slot, AreaType};
use crate::item::material::{ArmorType, ToolType, MaterialLevel, EquipmentType, BasicConversion};
use crate::item::item_lut::{
	ItemComp, ItemFlags, ItemExtendedData, PropertyValue, PropertyType,
	ToolData, ArmorData, EquipmentData, EquipmentSetStruct, BitStorage, TierStorage
};
use crate::fs::binary::{BinarySerializable, FixedBinarySize, BINARY_SIZE_STRING};
use crate::impl_option_binary;
use std::num::NonZero;

impl_option_binary!(PropertyValue, PropertyType, CustomData, Box<CustomData>, ItemExtendedData, ItemFilter, ItemStack);

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
impl FixedBinarySize for ItemFlags {
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
impl FixedBinarySize for MaterialLevel {
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
impl FixedBinarySize for ToolType {
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
impl FixedBinarySize for ArmorType {
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
impl FixedBinarySize for PropertyType {
	const BINARY_SIZE: usize = 1;
}



// Add BinarySerializable implementation for EquipmentSetStruct
impl<T, S, TS> BinarySerializable for EquipmentSetStruct<T, S, TS>
where
	T: EquipmentType + BasicConversion<T>,
	S: BitStorage + BinarySerializable + FixedBinarySize,
	TS: TierStorage + BinarySerializable + FixedBinarySize,
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
	T: EquipmentType + BinarySerializable + FixedBinarySize,
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
				data.extend_from_slice(&value.to_binary());
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
				let non_zero = NonZero::<u32>::from_binary(rest_bytes)?;
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
			data.extend_from_slice(&self.data[i].to_binary());
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
			
			let maybe_property = Option::<PropertyValue>::from_binary(&bytes[offset..])?;
			offset += maybe_property.binary_size();
			
			result.data[i] = maybe_property;
		}
		
		Some(result)
	}
	
	fn binary_size(&self) -> usize {
		let mut size = 1; // For the length byte
		for i in 0..self.len as usize {
			size += self.data[i].binary_size();
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
		data.extend_from_slice(&self.data.to_binary());
		
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
		let data = Option::<ItemExtendedData>::from_binary(&bytes[offset..])?;
		
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
		size += self.data.binary_size(); // extended data
		size
	}
}

impl BinarySerializable for CustomData {
	fn to_binary(&self) -> Vec<u8> {
		let mut data = Vec::new();
		
		// Serialize name (optional string)
		data.extend_from_slice(&self.name.to_binary());
		
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
		let name = Option::<String>::from_binary(&bytes[offset..])?;
		offset += name.binary_size();
		
		// Deserialize durability
		let durability = Option::<NonZero<u16>>::from_binary(&bytes[offset..])?;
		
		Some(CustomData {
			name,
			durability,
		})
	}
	
	fn binary_size(&self) -> usize {
		let mut size = 0;
		size += self.name.binary_size(); // string bytes
		size += self.durability.binary_size(); // u16 for durability
		
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
		
		// Serialize custom data (optional data)
		data.extend_from_slice(&self.data.to_binary());
		
		data
	}
	
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		if bytes.is_empty() { return None; }
		
		let mut offset = 0;
		
		// Deserialize name
		let name = String::from_binary(&bytes[offset..])?;
		offset += name.binary_size();
		
		// Deserialize stack count
		if offset >= bytes.len() { return None; }
		let stack = u32::from_binary(&bytes[offset..offset+u32::BINARY_SIZE])?;
		offset += u32::BINARY_SIZE;
		
		// Deserialize custom data
		let data = Option::<Box<CustomData>>::from_binary(&bytes[offset..])?;
		
		Some(ItemStack::create(name,stack,data))
	}
	
	fn binary_size(&self) -> usize {
		let mut size = self.name().to_string().binary_size(); // string bytes
		size += u32::BINARY_SIZE; // stack count
		size += self.data.binary_size(); // optional boxed data
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

impl FixedBinarySize for Slot {
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

impl FixedBinarySize for AreaType {
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
		for maybe_item in self.iter() {
			data.extend_from_slice(&maybe_item.to_binary());
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
			
			let maybe_item = Option::<ItemStack>::from_binary(&bytes[offset..])?;
			offset += maybe_item.binary_size();
			items.push(maybe_item);
		}
		
		Some(ItemContainer::from_raw(size, items))
	}
	
	fn binary_size(&self) -> usize {
		let mut size = Slot::BINARY_SIZE; // size
		size += usize::BINARY_SIZE; // item count
		
		for maybe_item in self.iter() {
			size += maybe_item.binary_size();
		}
		
		size
	}
}


impl BinarySerializable for ItemFilter {
	fn to_binary(&self) -> Vec<u8> {
		// Implement based on your ItemFilter structure
		// This is a placeholder - adjust based on your actual ItemFilter implementation
		Vec::new()
	}
	
	fn from_binary(bytes: &[u8]) -> Option<Self> {
		// Implement based on your ItemFilter structure
		Some(ItemFilter::default()) // Placeholder
	}
	
	fn binary_size(&self) -> usize {
		// Implement based on your ItemFilter structure
		0
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
		
		// Serialize storage pointer (we can't serialize raw pointers)
		// will update it to a World pos or something later
		
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
		//offset += crafting_def.binary_size();
				
		Some(Inventory::from_raw(
			armor,
			items,
			hotbar,
			crafting_def,
			None
		))
	}
	
	fn binary_size(&self) -> usize {
		let mut size = 0;
		
		size += self.armor().binary_size();
		size += self.inv().binary_size();
		size += self.hotbar().binary_size();
		size += self.crafting_def.binary_size();
		
		size
	}
}
