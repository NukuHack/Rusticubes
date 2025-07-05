
use crate::config;
use crate::ext::time;
use std::io::{Write,Read};
use std::path::{Path,PathBuf};
use std::io::Result;

pub fn get_world_names() -> Result<Vec<String>> {
    let path = config::get_save_path().join("saves");

    let mut folders = Vec::new();

    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            if let Some(folder_name) = path.file_name() {
                if let Some(name_str) = folder_name.to_str() {
                    folders.push(name_str.to_string());
                }
            }
        }
    }

    Ok(folders)
}

pub fn del_world(world_name: &str) {
    // Get the saves path
    let saves_path = match config::get_save_path().join("saves").canonicalize() {
        Ok(p) => p,
        Err(e) => {
            println!("Failed to access saves directory: {}", e);
            return;
        }
    };

    let target_path = saves_path.join(world_name);  // Fixed: use world_name instead of name
    if !target_path.exists() {
        println!("World '{}' does not exist", world_name);
        return;
    }
    if !target_path.is_dir() {
        println!("'{}' is not a directory", world_name);
        return;
    }
    // Try to delete the directory
    match std::fs::remove_dir_all(&target_path) {
        Ok(_) => {
            println!("Successfully deleted world '{}'", world_name);
            // Refresh UI after successful deletion
            let state = config::get_state();
            state.ui_manager.setup_ui();
        },
        Err(e) => {
            println!("Failed to delete world '{}': {}", world_name, e);
        }
    }
}



#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct WorldData {
    pub version: String,
    pub creation_date: time::Time,
    pub last_opened_date: time::Time,
}

impl WorldData {
    pub fn new() -> Self {
        WorldData {
            version: std::env!("CARGO_PKG_VERSION").to_string(),
            creation_date: time::Time::now(),
            last_opened_date: time::Time::now(),
        }
    }
    
    pub fn update_last_opened(&mut self) {
        self.last_opened_date = time::Time::now();
    }
}

pub fn load_world_data(path: &Path) -> std::io::Result<WorldData> {
    let file_path = path.join("world_data.json");
    
    std::fs::create_dir_all(path)?;
    
    match std::fs::File::open(&file_path) {
        Ok(mut file) => {
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            
            serde_json::from_str(&contents)
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let new_data = WorldData::new();
            save_world_data(path, &new_data)?;
            Ok(new_data)
        },
        Err(e) => Err(e),
    }
}

pub fn save_world_data(path: &Path, data: &WorldData) -> std::io::Result<()> {
    let file_path = path.join("world_data.json");
    
    let json = serde_json::to_string_pretty(data)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    
    let temp_path = file_path.with_extension("tmp");
    {
        let mut file = std::fs::File::create(&temp_path)?;
        file.write_all(json.as_bytes())?;
    }
    std::fs::rename(temp_path, file_path)?;
    
    Ok(())
}

pub fn update_world_data(path: &PathBuf) -> std::io::Result<()> {
    let mut world_data = load_world_data(path)?;
    let current_version = std::env!("CARGO_PKG_VERSION");
    
    if world_data.version != current_version {
        world_data.version = current_version.to_string();
        // Optionally update creation_date if you want to track version changes
        // world_data.creation_date = time::Time::now();
    }
    
    world_data.update_last_opened();
    
    save_world_data(path, &world_data)?;
    println!("World data updated");
    
    Ok(())
}