
use std::path::PathBuf;

pub struct AppConfig {
	pub window_title: String,
	pub initial_window_size: winit::dpi::PhysicalSize<f32>,
	pub min_window_size: winit::dpi::PhysicalSize<f32>,
	pub initial_window_position: winit::dpi::PhysicalPosition<f32>,
	pub theme: Option<winit::window::Theme>
}

impl Default for AppConfig {
	#[inline]
	 fn default() -> Self {
		Self {
			window_title: "Default App".into(),
			initial_window_size: winit::dpi::PhysicalSize::new(1280.0, 720.0),
			min_window_size: winit::dpi::PhysicalSize::new(600.0, 400.0),
			initial_window_position: winit::dpi::PhysicalPosition::new(100.0,100.0),
			theme: Some(winit::window::Theme::Dark),
		}
	}
}
impl AppConfig {
	#[inline]
	pub fn new(size: winit::dpi::PhysicalSize<u32>) -> Self {
		let width:f32 = 1280.0; let height:f32 = 720.0;
		let x:f32 = (size.width as f32 - width) / 2.0;
		let y:f32 = (size.height as f32 - height) / 2.0;
		Self {
			window_title: "WGPU App".into(),
			initial_window_size: winit::dpi::PhysicalSize::new(width, height),
			min_window_size: winit::dpi::PhysicalSize::new(width/3.0, height/3.0),
			initial_window_position: winit::dpi::PhysicalPosition::new(x,y),
			..Self::default()
		}
	}
}
#[inline]
pub fn get_save_path() -> PathBuf {
	let mut path = if cfg!(windows) {
		dirs::document_dir()
			.unwrap_or_else(|| PathBuf::from("."))
			.join("My Games")
	} else if cfg!(target_os = "macos") {
		dirs::home_dir()
			.unwrap_or_else(|| PathBuf::from("."))
			.join("Library/Application Support")
	} else {
		// Linux and others
		dirs::data_local_dir()
			.unwrap_or_else(|| PathBuf::from("."))
	};

	path.push("Rusticubes");
	path
}
#[inline]
pub fn ensure_save_dir() -> std::io::Result<PathBuf> {
	let path = get_save_path();
	std::fs::create_dir_all(&path)?;
	Ok(path)
}


// 4. Better configuration and constants management
pub struct InventoryConfig {
    pub slot_size: f32,
    pub slot_padding: f32,
    pub section_spacing: f32,
    pub panel_padding: f32,
    pub border_width: f32,
    pub colors: InventoryColors,
}

pub struct InventoryColors {
    pub panel_bg: (u8, u8, u8),
    pub panel_border: (u8, u8, u8, u8),
    pub inventory_slot: (u8, u8, u8),
    pub inventory_border: (u8, u8, u8, u8),
    pub hotbar_slot: (u8, u8, u8),
    pub hotbar_border: (u8, u8, u8, u8),
    pub armor_slot: (u8, u8, u8),
    pub armor_border: (u8, u8, u8, u8),
    pub storage_slot: (u8, u8, u8),
    pub storage_border: (u8, u8, u8, u8),
    pub crafting_slot: (u8, u8, u8),
    pub crafting_border: (u8, u8, u8, u8),
}

impl Default for InventoryConfig {
    fn default() -> Self {
        Self {
            slot_size: 0.08,
            slot_padding: 0.02,
            section_spacing: 0.12,
            panel_padding: 0.05,
            border_width: 0.003,
            colors: InventoryColors::default(),
        }
    }
}

impl Default for InventoryColors {
    fn default() -> Self {
        Self {
            panel_bg: (25, 25, 40),
            panel_border: (60, 60, 90, 255),
            inventory_slot: (40, 40, 60),
            inventory_border: (80, 80, 120, 255),
            hotbar_slot: (50, 50, 70),
            hotbar_border: (90, 90, 130, 255),
            armor_slot: (60, 60, 80),
            armor_border: (100, 100, 140, 255),
            storage_slot: (40, 40, 60),
            storage_border: (80, 80, 120, 255),
            crafting_slot: (60, 80, 60),
            crafting_border: (80, 120, 80, 255),
        }
    }
}

