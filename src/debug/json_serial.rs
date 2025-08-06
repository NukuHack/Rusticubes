
#[cfg(test)]
mod tests {
	use crate::fs::json::{JsonValue, JsonSerializable, JsonParser, JsonError};
	use std::collections::HashMap;

	#[test]
	fn string_serialization() {
		let s = "hello".to_string();
		let json = s.to_json();
		assert_eq!(json, JsonValue::String("hello".to_string()));
		
		let deserialized: String = String::from_json(&json).unwrap();
		assert_eq!(deserialized, "hello");
	}

	#[test]
	fn boolean_serialization() {
		let b = true;
		let json = b.to_json();
		assert_eq!(json, JsonValue::Bool(true));
		
		let deserialized: bool = bool::from_json(&json).unwrap();
		assert!(deserialized);
	}

	#[test]
	fn float_serialization() {
		let f = 3.14f64;
		let json = f.to_json();
		assert_eq!(json, JsonValue::Number(3.14));
		
		let deserialized: f64 = f64::from_json(&json).unwrap();
		assert!((deserialized - 3.14).abs() < f64::EPSILON);
	}

	#[test]
	fn hashmap_serialization() {
		let mut map = HashMap::new();
		map.insert("key1".to_string(), 42u32);
		map.insert("key2".to_string(), 100u32);
		
		let json = map.to_json();
		match json {
			JsonValue::Object(ref obj) => {
				assert_eq!(obj.len(), 2);
				assert_eq!(obj["key1"], JsonValue::Number(42.0));
				assert_eq!(obj["key2"], JsonValue::Number(100.0));
			}
			_ => panic!("Expected JsonValue::Object"),
		}
		
		let deserialized: HashMap<String, u32> = HashMap::from_json(&json).unwrap();
		assert_eq!(deserialized["key1"], 42);
		assert_eq!(deserialized["key2"], 100);
	}

	#[test]
	fn array_serialization() {
		let arr = [1u32, 2, 3];
		let json = arr.to_json();
		let expected = JsonValue::Array(vec![
			JsonValue::Number(1.0),
			JsonValue::Number(2.0),
			JsonValue::Number(3.0),
		]);
		assert_eq!(json, expected);
		
		let deserialized: [u32; 3] = <[u32; 3]>::from_json(&json).unwrap();
		assert_eq!(deserialized, [1, 2, 3]);
	}

	#[test]
	fn tuple_serialization() {
		let tuple = (42u32, "hello".to_string());
		let json = tuple.to_json();
		let expected = JsonValue::Array(vec![
			JsonValue::Number(42.0),
			JsonValue::String("hello".to_string()),
		]);
		assert_eq!(json, expected);
		
		let deserialized: (u32, String) = <(u32, String)>::from_json(&json).unwrap();
		assert_eq!(deserialized, (42, "hello".to_string()));
	}

	#[test]
	fn json_value_serialization() {
		let value = JsonValue::Bool(true);
		let json = value.to_json();
		assert_eq!(json, JsonValue::Bool(true));
		
		let deserialized: JsonValue = JsonValue::from_json(&json).unwrap();
		assert_eq!(deserialized, JsonValue::Bool(true));
	}

	#[test]
	fn parse_complex_json() {
		let json_data = r#"
		{
			"name": "John Doe",
			"age": 30,
			"is_student": false,
			"courses": ["Math", "Science"],
			"address": {
				"street": "123 Main St",
				"city": "Anytown"
			},
			"metadata": null
		}"#;

		let mut parser = JsonParser::new(json_data);
		let result = parser.parse_self().expect("Should parse correctly");
		
		if let JsonValue::Object(map) = result {
			assert_eq!(map["name"], JsonValue::String("John Doe".to_string()));
			assert_eq!(map["age"], JsonValue::Number(30.0));
			assert_eq!(map["is_student"], JsonValue::Bool(false));
			
			if let JsonValue::Array(courses) = &map["courses"] {
				assert_eq!(courses.len(), 2);
			} else {
				panic!("Expected courses array");
			}
			
			if let JsonValue::Object(address) = &map["address"] {
				assert_eq!(address["street"], JsonValue::String("123 Main St".to_string()));
			} else {
				panic!("Expected address object");
			}
			
			assert_eq!(map["metadata"], JsonValue::Null);
		} else {
			panic!("Expected root object");
		}
	}

	#[test]
	fn parse_string_with_escapes() {
		let json_data = r#""Hello,\nWorld!\t\"Quote\" and Unicode: \u03A9""#;
		let mut parser = JsonParser::new(json_data);
		let result = parser.parse_self().expect("Should parse correctly");
		
		if let JsonValue::String(s) = result {
			assert_eq!(s, "Hello,\nWorld!\t\"Quote\" and Unicode: Ω");
		} else {
			panic!("Expected string");
		}
	}

	#[test]
	fn parse_invalid_json() {
		let invalid_json = "{ \"key\": [1, 2, }";
		let mut parser = JsonParser::new(invalid_json);
		let result = parser.parse_self();
		assert!(result.is_err());
	}

	#[test]
	fn type_mismatch_errors() {
		let json_num = JsonValue::Number(42.0);
		let result: Result<String, _> = String::from_json(&json_num);
		assert!(matches!(result, Err(JsonError::Type { .. })));

		let json_str = JsonValue::String("42".to_string());
		let result: Result<u32, _> = u32::from_json(&json_str);
		assert!(matches!(result, Err(JsonError::Type { .. })));

		let json_bool = JsonValue::Bool(true);
		let result: Result<Vec<u32>, _> = Vec::<u32>::from_json(&json_bool);
		assert!(matches!(result, Err(JsonError::Type { .. })));
	}

	#[test]
	fn number_validation() {
		// Test u32 bounds
		let max_u32 = JsonValue::Number(u32::MAX as f64);
		let min_u32 = JsonValue::Number(u32::MIN as f64);
		assert!(u32::from_json(&max_u32).is_ok());
		assert!(u32::from_json(&min_u32).is_ok());

		let overflow_u32 = JsonValue::Number(u32::MAX as f64 + 1.0);
		assert!(u32::from_json(&overflow_u32).is_err());

		// Test negative for unsigned
		let negative = JsonValue::Number(-1.0);
		assert!(u32::from_json(&negative).is_err());

		// Test fractional numbers
		let fractional = JsonValue::Number(42.5);
		assert!(u32::from_json(&fractional).is_err());
		assert!(i32::from_json(&fractional).is_err());
		assert!(f64::from_json(&fractional).is_ok());
	}

	#[test]
	fn special_floats() {
		let nan = JsonValue::Number(f64::NAN);
		let inf = JsonValue::Number(f64::INFINITY);
		let neg_inf = JsonValue::Number(f64::NEG_INFINITY);

		// These should be rejected
		assert!(f64::from_json(&nan).is_err());
		assert!(f64::from_json(&inf).is_err());
		assert!(f64::from_json(&neg_inf).is_err());

		// Serialization of special floats should produce null
		assert_eq!(f64::NAN.to_json(), JsonValue::Null);
		assert_eq!(f64::INFINITY.to_json(), JsonValue::Null);
	}

	#[test]
	fn empty_collections() {
		let empty_vec: Vec<u32> = vec![];
		let json = empty_vec.to_json();
		assert_eq!(json, JsonValue::Array(vec![]));

		let empty_map: HashMap<String, u32> = HashMap::new();
		let json = empty_map.to_json();
		assert_eq!(json, JsonValue::Object(HashMap::new()));
	}

	#[test]
	fn parse_empty_string() {
		let json_data = "";
		let mut parser = JsonParser::new(json_data);
		let result = parser.parse_self();
		assert!(result.is_err());
	}

	#[test]
	fn parse_whitespace_only() {
		let json_data = "  \t\n\r ";
		let mut parser = JsonParser::new(json_data);
		let result = parser.parse_self();
		assert!(result.is_err());
	}
}
