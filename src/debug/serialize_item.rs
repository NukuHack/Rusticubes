
use std::num::NonZeroU32;
#[allow(unused_imports)]
use crate::game::material::MaterialLevel;
use crate::game::items::ItemId;
#[allow(unused_imports)]
use crate::game::item_lut::{
	ItemComp, ItemFlags, ItemExtendedData, PropertyVariantTag,
	ToolData, ArmorData, ToolType, ArmorType, EquipmentData, EquipmentSetStruct,
	EquipmentTypeSet, BitStorage, TierStorage, EquipmentType
};
use crate::hs::binary::BinarySerializable;


	
#[test]
fn item_id_serialization() {
	let item_id = ItemId(1234);
	let serialized = item_id.to_binary();
	let deserialized = ItemId::from_binary(&serialized).unwrap();
	assert_eq!(item_id, deserialized);
}

#[test]
fn item_flags_serialization() {
	let flags = ItemFlags::new(ItemFlags::IS_BLOCK | ItemFlags::IS_TOOL);
	let serialized = flags.to_binary();
	let deserialized = ItemFlags::from_binary(&serialized).unwrap();
	assert_eq!(flags, deserialized);
}

#[test]
fn extended_data_serialization() {
	let extended_data = ItemExtendedData::<4>::new()
		.with_durability(NonZeroU32::new(100).unwrap())
		.with_damage(50);
	
	let serialized = extended_data.to_binary();
	let deserialized = ItemExtendedData::<4>::from_binary(&serialized).unwrap();
	
	assert_eq!(extended_data.len(), deserialized.len());
	assert_eq!(extended_data.get_durability(), deserialized.get_durability());
	assert_eq!(extended_data.get_damage(), deserialized.get_damage());
}

#[test]
fn item_comp_data_serialization() {
	let extended_data = ItemExtendedData::<4>::new()
		.with_durability(NonZeroU32::new(100).unwrap())
		.with_damage(50);

	let apple_item_data = ItemComp::new(55, "apple").with_stack(32).with_data(extended_data.clone());
	
	let serialized = apple_item_data.to_binary();
	let deserialized = ItemComp::from_binary(&serialized).unwrap();
	let data = deserialized.data.clone().unwrap();
	
	assert_eq!(extended_data.len(), data.len());
	assert_eq!(extended_data.get_durability(), data.get_durability());
	assert_eq!(extended_data.get_damage(), data.get_damage());
}
#[test]
fn item_comp_serialization() {
	let apple_item_data = ItemComp::new(12345, "poopy_head");
	let serialized = apple_item_data.copy().to_binary();
	let deserialized = ItemComp::from_binary(&serialized).unwrap();
	
	assert_eq!(apple_item_data.id, deserialized.id);
	assert_eq!(apple_item_data.max_stack, deserialized.max_stack);
	assert_eq!(apple_item_data.flags, deserialized.flags);
	assert_eq!(apple_item_data.data, deserialized.data);
	assert_eq!(apple_item_data.name, deserialized.name);
}


type StatString = &'static str;
#[test]
fn string_serialization() {
	let data ="poopy_head";
	let serialized = &data.to_binary();
	let deserialized = StatString::from_binary(&serialized).unwrap();
	
	assert_eq!(data, deserialized);
}
