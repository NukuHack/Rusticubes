
use crate::item::inventory::Slot;
use std::num::NonZeroU32;
use crate::item::material::{ToolType, ArmorType, MaterialLevel, BasicConversion};
use crate::item::item_lut::{
	ItemComp, ItemFlags, ItemExtendedData,
	ToolData, ToolSet, ArmorData, ArmorSet
};
use crate::fs::json::{JsonValue, JsonSerializable, JsonError};
use std::result::Result;

impl JsonSerializable for ItemComp {
	fn from_json(json: &JsonValue) -> Result<ItemComp, JsonError> {
		let obj = match json {
			JsonValue::Object(map) => map,
			_ => return Err(JsonError::Custom("Input JsonValue is not an Object".into())),
		};

		// Extract basic fields
		let name = match obj.get("name") {
			Some(JsonValue::String(s)) => s.clone(),
			_ => return Err(JsonError::MissingField("name is not found or incorrect type".into())),
		};

		let max_stack = match obj.get("max_stack") {
			Some(JsonValue::Number(n)) => *n as u32,
			_ => 64, // default value
		};

		let flags = match obj.get("flags") {
			Some(JsonValue::Number(n)) => ItemFlags(*n as u32),
			_ => ItemFlags::empty(),
		};

		// Handle extended data
		let data = if let Some(data_value) = obj.get("data") {
			match parse_extended_data(data_value) {
				Ok(result) => result ,
				Err(e) => { println!("Error: {:?}", e); None },
			}
		} else {
			None
		};

		Ok(ItemComp {
			name: name.into(),
			max_stack,
			flags,
			data,
		})
	}

	fn to_json(&self) -> JsonValue {
		todo!()
	}
}

fn parse_extended_data(data_value: &JsonValue) -> Result<Option<ItemExtendedData>, JsonError> {
	let data_obj = data_value.as_object()
		.ok_or_else(|| JsonError::Custom("Extended data must be an object".into()))?;

	let mut extended_data = ItemExtendedData::new();

	for (key, value) in data_obj {
		match key.as_str() {
			"durability" => {
				let durability = value.as_f64()
					.and_then(|n| NonZeroU32::new(n as u32))
					.ok_or_else(|| JsonError::Custom("Invalid durability value".into()))?;
				extended_data = extended_data.with_durability(durability);
			},
			"damage" => {
				let damage = value.as_f64()
					.map(|n| n as i16)
					.ok_or_else(|| JsonError::Custom("Invalid damage value".into()))?;
				extended_data = extended_data.with_damage(damage);
			},
			"hunger" => {
				let hunger = value.as_f64()
					.map(|n| n as i16)
					.ok_or_else(|| JsonError::Custom("Invalid hunger value".into()))?;
				extended_data = extended_data.with_hunger(hunger);
			},
			"armor_value" => {
				let armor_value = value.as_f64()
					.map(|n| n as i16)
					.ok_or_else(|| JsonError::Custom("Invalid armor_value".into()))?;
				extended_data = extended_data.with_armor(armor_value);
			},
			"speed" => {
				let speed = value.as_f64()
					.map(|n| n as i16)
					.ok_or_else(|| JsonError::Custom("Invalid speed value".into()))?;
				extended_data = extended_data.with_speed(speed);
			},
			"tool_data" => {
				match value {
					JsonValue::Object(tool_obj) => {
						let material = tool_obj.get("material")
							.and_then(|v| v.as_str())
							.ok_or_else(|| JsonError::MissingField("'tool_data.material' is missing or not a string".into()))?;

						let equip_type = tool_obj.get("type")
							.and_then(|v| v.as_str())
							.ok_or_else(|| JsonError::MissingField("'tool_data.equip_type' is missing or not a string".into()))?;

						let tier = MaterialLevel::from_str(material)
							.ok_or_else(|| JsonError::Custom(format!("Invalid material level: {}", material).into()))?;
						let tool_type = ToolType::from_str(equip_type)
							.ok_or_else(|| JsonError::Custom(format!("Invalid tool type: {}", equip_type).into()))?;
						
						let tool_data = ToolData::single(tool_type, tier);
						extended_data = extended_data.with_tool(tool_data);
					},
					JsonValue::Array(tiers) => {
						let mut tool_set = ToolSet::new();
						
						for tier_val in tiers {
							let tier_obj = tier_val.as_object()
								.ok_or_else(|| JsonError::Custom("Tool tier entry must be an object".into()))?;

							let material = tier_obj.get("material")
								.and_then(|v| v.as_str())
								.ok_or_else(|| JsonError::MissingField("tool tier 'material' is missing or not a string".into()))?;

							let equip_type = tier_obj.get("type")
								.and_then(|v| v.as_str())
								.ok_or_else(|| JsonError::MissingField("tool tier 'type' is missing or not a string".into()))?;

							let tier = MaterialLevel::from_str(material)
								.ok_or_else(|| JsonError::Custom(format!("Invalid material level: {}", material).into()))?;
							let tool_type = ToolType::from_str(equip_type)
								.ok_or_else(|| JsonError::Custom(format!("Invalid tool type: {}", equip_type).into()))?;
							
							tool_set.add_equipment(tool_type, tier);
						}
						
						let tool_data = ToolData::Multiple(tool_set);
						extended_data = extended_data.with_tool(tool_data);
					},
					_ => return Err(JsonError::Custom("tool_data must be either an object or array".into())),
				}
			},
			"armor_data" => {
				match value {
					JsonValue::Object(armor_obj) => {
						let material = armor_obj.get("material")
							.and_then(|v| v.as_str())
							.ok_or_else(|| JsonError::MissingField("'armor_data.material' is missing or not a string".into()))?;

						let equip_type = armor_obj.get("type")
							.and_then(|v| v.as_str())
							.ok_or_else(|| JsonError::MissingField("'armor_data.type' is missing or not a string".into()))?;

						let tier = MaterialLevel::from_str(material)
							.ok_or_else(|| JsonError::Custom(format!("Invalid material level: {}", material).into()))?;
						let armor_type = ArmorType::from_str(equip_type)
							.ok_or_else(|| JsonError::Custom(format!("Invalid armor type: {}", equip_type).into()))?;
						
						let armor_data = ArmorData::single(armor_type, tier);
						extended_data = extended_data.with_equpment(armor_data);
					},
					JsonValue::Array(tiers) => {
						let mut armor_set = ArmorSet::new();
						
						for tier_val in tiers {
							let tier_obj = tier_val.as_object()
								.ok_or_else(|| JsonError::Custom("Armor tier entry must be an object".into()))?;

							let material = tier_obj.get("material")
								.and_then(|v| v.as_str())
								.ok_or_else(|| JsonError::MissingField("armor tier 'material' is missing or not a string".into()))?;

							let equip_type = tier_obj.get("type")
								.and_then(|v| v.as_str())
								.ok_or_else(|| JsonError::MissingField("armor tier 'type' is missing or not a string".into()))?;

							let tier = MaterialLevel::from_str(material)
								.ok_or_else(|| JsonError::Custom(format!("Invalid material level: {}", material).into()))?;
							let armor_type = ArmorType::from_str(equip_type)
								.ok_or_else(|| JsonError::Custom(format!("Invalid armor type: {}", equip_type).into()))?;
							
							armor_set.add_equipment(armor_type, tier);
						}
						
						let armor_data = ArmorData::Multiple(armor_set);
						extended_data = extended_data.with_equpment(armor_data);
					},
					_ => return Err(JsonError::Custom("armor_data must be either an object or array".into())),
				}
			},
			"storage_data" => {
				match value {
					JsonValue::Object(slot_obj) => {
						let rows = slot_obj.get("rows")
							.and_then(|v| v.as_f64())
							.map(|n| n as u8)
							.ok_or_else(|| JsonError::MissingField("storage_data.rows is missing or not a correct u8 value".into()))?;

						let cols = slot_obj.get("cols")
							.and_then(|v| v.as_f64())
							.map(|n| n as u8)
							.ok_or_else(|| JsonError::MissingField("storage_data.cols is missing or not a correct u8 value".into()))?;
						
						let slot_data = Slot::custom(rows, cols);
						extended_data = extended_data.with_slot(slot_data);
					},
					JsonValue::Array(_tiers) => {
						return Err(JsonError::Custom("storage_data for Array is not yet implemented".into()));
					},
					_ => return Err(JsonError::Custom("storage_data must be either an object or array".into())),
				}
			},
			_ => {
				return Err(JsonError::Custom(format!("This: '{:?}' is not an allowed data type in items.", key).into()));
			}
		}
	}

	Ok(Some(extended_data))
}
