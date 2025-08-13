
#[cfg(test)]
mod tests {
	use crate::fs::json::{JsonParser, JsonSerializable};
	use std::num::NonZeroU32;
	use crate::item::material::{ArmorType, ToolType, MaterialLevel, EquipmentType, BasicConversion};
	use crate::item::item_lut::{
		ItemComp, ItemFlags, ItemExtendedData,
		ToolData, ToolSet
	};
	use crate::fs::binary::BinarySerializable;


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

		let apple_item_data = ItemComp::new("apple").with_stack(32).with_data(extended_data.clone());
		
		let serialized = apple_item_data.to_binary();
		let deserialized = ItemComp::from_binary(&serialized).unwrap();
		let data = deserialized.data.clone().unwrap();
		
		assert_eq!(extended_data.len(), data.len());
		assert_eq!(extended_data.get_durability(), data.get_durability());
		assert_eq!(extended_data.get_damage(), data.get_damage());
	}
	#[test]
	fn item_comp_serialization() {
		let apple_item_data = ItemComp::new("poopy_head");
		let serialized = apple_item_data.clone().to_binary();
		let deserialized = ItemComp::from_binary(&serialized).unwrap();
		
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

	#[test]
	fn item_comp_serial() {

		let item = ItemComp::new("brick_grey").with_flag(ItemFlags::new(ItemFlags::IS_BLOCK));
		let item_re = ItemComp::from_binary(&item.to_binary()).expect("Should deserialize correctly");

		if item == item_re {
			println!("equal");
		}
	}





	#[test]
	fn item_test() {
		let item = ItemComp {
			name: "apple",
			max_stack: 64,
			flags: ItemFlags(1), // IS_BLOCK
			data: None,
		};


		let json_data = "{
			\"name\":\"apple\",
			\"max_stack\": 64,
			\"flags\": 1,
			\"data\": null
		}";
		let mut parser = JsonParser::new(json_data);
		let result = parser.parse_self().expect("Should parse item correctly");
		let parsed_item = ItemComp::from_json(&result).expect("Should make item correctly");


		assert_eq!(item, parsed_item);
	}

	#[test]
	fn item_test_data() {
		let item = ItemComp {
			name: "bread",
			max_stack: 50,
			flags: ItemFlags(0),
			data: Some(
			ItemExtendedData::new().with_durability(NonZeroU32::new(200).unwrap())
				.with_damage(10)
			),
		};


		let json_data = r#"{
			"name":"bread",
			"max_stack": 50,
			"flags": 0,
			"data": {
				"data" : {
					"durability" : 200,
					"damage" : 10
				}
			}
		},"#;
		let mut parser = JsonParser::new(json_data);
		let result = parser.parse_self().expect("Should parse item correctly");
		let parsed_item = ItemComp::from_json(&result).expect("Should make item correctly");


		assert_eq!(item, parsed_item);
	}

	#[test]
	fn item_test_tooldata() {
		let item = ItemComp {
			name: "bread",
			max_stack: 50,
			flags: ItemFlags(0),
			data: Some(
			ItemExtendedData::new().with_durability(NonZeroU32::new(200).unwrap())
				.with_damage(10)
				.with_tool_data(ToolData::Single{ equip_type: ToolType::Stone , tier: MaterialLevel::Calcite })
			),
		};


		let json_data = r#"{
			"name":"bread",
			"max_stack": 50,
			"flags": 0,
			"data": {
				"data" : {
					"durability" : 200,
					"damage" : 10,
					"tool_data" : {
						"equip_type" : "Stone",
						"material" : "Calcite"
					}
				}
			}
		},"#;
		let mut parser = JsonParser::new(json_data);
		let result = parser.parse_self().expect("Should parse item correctly");
		let parsed_item = ItemComp::from_json(&result).expect("Should make item correctly");


		assert_eq!(item, parsed_item);
	}

	#[test]
	fn item_test_tool_multipledata() {
		let mut data = ToolSet::new();
		data.add_equipment(ToolType::from_str("Stone").expect("REASON"), MaterialLevel::from_str("Calcite").expect("REASON"));
		data.add_equipment(ToolType::from_str("Wood").expect("REASON"), MaterialLevel::from_str("Calcite").expect("REASON"));
		data.add_equipment(ToolType::from_str("Dirt").expect("REASON"), MaterialLevel::from_str("Calcite").expect("REASON"));
		data.add_equipment(ToolType::from_str("Crop").expect("REASON"), MaterialLevel::from_str("Calcite").expect("REASON"));
		data.add_equipment(ToolType::from_str("String").expect("REASON"), MaterialLevel::from_str("Calcite").expect("REASON"));
		data.add_equipment(ToolType::from_str("Metal").expect("REASON"), MaterialLevel::from_str("Calcite").expect("REASON"));
		
		let item = ItemComp {
			name: "bread",
			max_stack: 50,
			flags: ItemFlags(0),
			data: Some(
			ItemExtendedData::new().with_durability(NonZeroU32::new(200).unwrap())
				.with_damage(10)
				.with_tool_data(ToolData::Multiple(data))
			),
		};


		let json_data = r#"
		{
			"name":"bread",
			"max_stack": 50,
			"flags": 0,
			"data": {
				"data" : {
					"durability" : 200,
					"damage" : 10,
					"tool_data" : [
						{
							"equip_type" : "Stone",
							"material" : "Calcite"
						},
						{
							"equip_type" : "Wood",
							"material" : "Calcite"
						},
						{
							"equip_type" : "Dirt",
							"material" : "Calcite"
						},
						{
							"equip_type" : "Crop",
							"material" : "Calcite"
						},
						{
							"equip_type" : "String",
							"material" : "Calcite"
						},
						{
							"equip_type" : "Metal",
							"material" : "Calcite"
						}
					]
				}
			}
		}
		"#;
		let mut parser = JsonParser::new(json_data);
		let result = parser.parse_self().expect("Should parse item correctly");
		let parsed_item = ItemComp::from_json(&result).expect("Should make item correctly");


		assert_eq!(item, parsed_item);
	}
}
