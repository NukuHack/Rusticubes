
use std::num::NonZeroU32;
use std::collections::HashMap;
use crate::item::material::{ToolType, ArmorType, MaterialLevel, EquipmentType, BasicConversion};
use crate::item::item_lut::{
	ItemComp, ItemFlags, ItemExtendedData,
	ToolData, ToolSet
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
			Some(JsonValue::String(s)) => s.as_str(),
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
		let data = match obj.get("data") {
			Some(JsonValue::Object(data_obj)) => {
				parse_extended_data(data_obj)
			},
			_ => None,
		};

		Ok(ItemComp {
			name: Box::leak(name.to_string().into_boxed_str()),
			max_stack,
			flags,
			data,
		})
	}

	fn to_json(&self) -> JsonValue {
		todo!()
	}
}

fn parse_extended_data(data_obj: &HashMap<String, JsonValue>) -> Option<ItemExtendedData> {
	let inner_data = match data_obj.get("data") {
		Some(JsonValue::Object(inner)) => inner,
		_ => return None,
	};

	let mut extended_data = ItemExtendedData::new();

	// Parse durability
	if let Some(JsonValue::Number(durability)) = inner_data.get("durability") {
		if let Some(nz) = NonZeroU32::new(*durability as u32) {
			extended_data = extended_data.with_durability(nz);
		}
	}

	// Parse damage
	if let Some(JsonValue::Number(damage)) = inner_data.get("damage") {
		extended_data = extended_data.with_damage(*damage as i16);
	}

	// Parse tool data
	if let Some(tool_data) = inner_data.get("tool_data") {
		match tool_data {
			JsonValue::Object(tool_obj) => {
				// Handle single material tool
				let Some(JsonValue::String(material)) = tool_obj.get("material") else { return None; };

				let Some(JsonValue::String(equip_type)) = tool_obj.get("equip_type") else { return None; };

				let tier = MaterialLevel::from_str(material).unwrap_or(MaterialLevel::from_u8(0)?);
				let tool_type = ToolType::from_str(equip_type.as_str())?;
				let tool_data = ToolData::single(tool_type, tier);
				extended_data = extended_data.with_tool_data(tool_data);
			},
			JsonValue::Array(tiers) => {
				let mut tool_set = ToolSet::new();
				
				for tier_val in tiers.iter() {
					let JsonValue::Object(tier_obj) = tier_val else { return None; };

					// Handle single material tool
					let Some(JsonValue::String(material)) = tier_obj.get("material") else { return None; };

					let Some(JsonValue::String(equip_type)) = tier_obj.get("equip_type") else { return None; };

					let tier = MaterialLevel::from_str(material).unwrap_or(MaterialLevel::from_u8(0)?);
					let tool_type = ToolType::from_str(equip_type.as_str())?;
					tool_set.add_equipment(tool_type, tier);
				}
				
				let tool_data = ToolData::Multiple(tool_set);
				extended_data = extended_data.with_tool_data(tool_data);
			},
			_ => (),
		}
	}

	Some(extended_data)
}
