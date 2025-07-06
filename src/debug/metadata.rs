
#[cfg(test)]
use crate::world::manager::{WorldData, load_world_data, save_world_data, update_world_data};
#[cfg(test)]
use crate::config;
#[cfg(test)]
use std::io::{self};
#[cfg(test)]
use crate::ext::time::Time;

// Test 1: Simple roundtrip serialization/deserialization with perfect data
#[test]
fn roundtrip_serialization() {
    let original = WorldData {
        version: "1.0.0".to_string(),
        creation_date: Time::now(),
        last_opened_date: Time::now(),
    };

    let bytes = original.to_bytes();
    let deserialized = WorldData::from_bytes(&bytes).unwrap();

    assert_eq!(original.version, deserialized.version);
    assert_eq!(original.creation_date, deserialized.creation_date);
    assert_eq!(original.last_opened_date, deserialized.last_opened_date);
}

// Test 2: File operations with correct data
#[test]
fn file_operations() -> io::Result<()> {
    let temp_dir = config::get_save_path().join("test");
    let path = temp_dir.as_path();

    // Test creating new data when file doesn't exist
    let loaded = load_world_data(path)?;
    assert_eq!(loaded.version, std::env!("CARGO_PKG_VERSION"));
    
    // Test saving and loading
    let mut data = WorldData::new();
    data.version = "test_version".to_string();
    save_world_data(path, &data)?;
    
    let loaded = load_world_data(path)?;
    assert_eq!(loaded.version, "test_version");
    
    // Test update functionality
    update_world_data(&path.to_path_buf())?;
    let updated = load_world_data(path)?;
    assert_eq!(updated.version, std::env!("CARGO_PKG_VERSION"));
    
    Ok(())
}

// Test 3: Malformed data handling
#[test]
fn deserialization_errors() {
    // Test empty input
    assert!(WorldData::from_bytes(&[]).is_err());
    
    // Test incomplete version length
    assert!(WorldData::from_bytes(&[1, 0, 0]).is_err());
    
    // Test version length longer than actual data
    let mut bad_data = vec![10, 0, 0, 0]; // Says version is 10 bytes long
    bad_data.extend_from_slice(b"short"); // But only provide 5 bytes
    assert!(WorldData::from_bytes(&bad_data).is_err());
    
    // Test incomplete time data
    let mut partial_time = vec![5, 0, 0, 0];
    partial_time.extend_from_slice(b"12345"); // Version
    partial_time.extend_from_slice(&[0; 5]); // Only half of first Time struct
    assert!(WorldData::from_bytes(&partial_time).is_err());
}
