
use glam::Vec3;

#[allow(dead_code)]
pub struct GameState {
    pub worldname: String,
    pub player: super::player::Player,
    pub world: super::cube::World, // lol main data storage :)
    pub save_path: std::path::PathBuf,
}

impl GameState {
    pub fn new(worldname: &str) -> Self {
        let player = super::player::Player::new(super::camera::CameraConfig::new(Vec3::new(0.5, 1.8, 2.0)));
        
        // Create the save path
        let save_path = std::path::PathBuf::from(&super::config::get_state().save_path)
            .join("saves")
            .join(worldname);
        
        // Create directories if they don't exist
        if let Err(e) = std::fs::create_dir_all(&save_path) {
            eprintln!("Failed to create save directory at {:?}: {}", save_path, e);
            // You might want to handle this error differently depending on your needs
        }

        Self {
            worldname: worldname.to_string(),
            player,
            world: super::cube::World::empty(),
            save_path,
        }
    }
}