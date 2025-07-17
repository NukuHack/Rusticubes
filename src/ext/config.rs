
use crate::ui::inventory::AreaType;
use std::path::PathBuf;
use crate::ext::color::{Color, Border};

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
			.join("Games")
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




// the image color will be used for animations too because that is just a "changing image" so yeah
// button color will be used for multi-state button too
// label's color is not rendered
// image's color is and extra color on the image, most of the times leaving this on white is the best
// slider's color is used for the track color and the text_color used for the actual slider's handle
// checkbox's text_color is only used for optional text
// the divider is currently only used as a makeshift crosshair in game

// the extra is just used for cargo package showing not much else
// Extra is optional since not all elements need it
// The text color is optional since it is only used like half of the cases

#[derive(Debug, Clone, Copy)]
pub enum ElementVariant {
	Basic,
	Nice,
	Bad,
	Extra, 
}
#[derive(Debug, Clone, Copy)]
pub struct ElementStyle {
	pub color: Color,
	pub border: Border,
	pub text_color: Option<Color>,
}
impl ElementStyle {
	pub fn text_color(&self) -> Color {
		self.text_color.unwrap_or(self.color)
	}
}

#[derive(Debug, Clone)]
pub struct VariantStyles {
	pub basic: ElementStyle,
	pub nice: ElementStyle,
	pub bad: ElementStyle,
	pub extra: Option<ElementStyle>, 
}
impl VariantStyles {
	pub fn extra(&self) -> ElementStyle {
		self.extra.clone().unwrap_or(self.basic.clone())
	}
}

#[derive(Debug, Clone)]
pub struct UITheme {
	pub bg_panel: ElementStyle,
	pub title_label: ElementStyle,
	pub best_button: ElementStyle,
	pub worst_button: ElementStyle,
	pub okay_button: ElementStyle,
	pub deny_button: ElementStyle,
	
	pub buttons: VariantStyles,
	pub panels: VariantStyles,
	pub labels: VariantStyles,
	pub images: VariantStyles,
	pub checkboxs: VariantStyles, // i know it should be called checkboxes but this is ... okay
	pub sliders: VariantStyles,
	pub inputs: VariantStyles,
	pub dividers: VariantStyles,

	pub inv: InventoryConfig,
}

#[derive(Debug, Clone)]
pub struct InventoryConfig {
	pub slot_size: f32,
	pub slot_padding: f32,
	pub slot_border_width: f32,
	pub section_spacing: f32,
	pub panel_padding: f32,

	pub panel_bg: ElementStyle,
	pub inventory: ElementStyle,
	pub hotbar: ElementStyle,
	pub armor: ElementStyle,
	pub storage: ElementStyle,
	pub input: ElementStyle,
	pub crafting: ElementStyle,
}
impl InventoryConfig {
	pub fn get_style(&self, area_type : AreaType) -> &ElementStyle {
		match area_type {
			AreaType::Panel => &self.panel_bg,
			AreaType::Inventory => &self.inventory,
			AreaType::Hotbar => &self.hotbar,
			AreaType::Armor => &self.armor,
			AreaType::Storage => &self.storage,
			AreaType::Input => &self.input,
			AreaType::Output => &self.crafting,
		}
	}
}

impl Default for InventoryConfig {
	fn default() -> Self {
		let panel_bg = ElementStyle{
			color: Color::rgb(25, 25, 40),
			border: Border::rgbf(60, 60, 90, 0.005),
			text_color: None,
		};
		let inventory = ElementStyle{
			color: Color::rgb(40, 40, 60),
			border: Border::rgbf(80, 80, 120, 0.005),
			text_color: None,
		};
		let hotbar = ElementStyle{
			color: Color::rgb(50, 50, 70),
			border: Border::rgbf(90, 90, 130, 0.005),
			text_color: None,
		};
		let armor = ElementStyle{
			color: Color::rgb(60, 60, 80),
			border: Border::rgbf(100, 100, 140, 0.005),
			text_color: None,
		};
		let storage = ElementStyle{
			color: Color::rgb(40, 40, 60),
			border: Border::rgbf(80, 80, 120, 0.005),
			text_color: None,
		};
		let input = ElementStyle{
			color: Color::rgb(40, 40, 60),
			border: Border::rgbf(80, 80, 120, 0.005),
			text_color: None,
		};
		let crafting = ElementStyle{
			color: Color::rgb(60, 80, 60),
			border: Border::rgbf(80, 120, 80, 0.005),
			text_color: None,
		};
		Self {
			slot_size: 0.08,
			slot_padding: 0.02,
			slot_border_width: 0.003,
			section_spacing: 0.12,
			panel_padding: 0.05,

			panel_bg,
			inventory,
			hotbar,
			armor,
			storage,
			input,
			crafting,
		}
	}
}

impl Default for UITheme {
	fn default() -> Self {		
		Self {
			bg_panel: ElementStyle {
				color: Color::rgb(15, 15, 25),
				border: Border::NONE, // No border for background
				text_color: None
			},
			title_label: ElementStyle {
				color: Color::rgb(30, 30, 45),
				border: Border::rgbf(60, 70, 110, 0.008),
				text_color: Color::rgb(180, 200, 220).o(),
			},
			best_button: ElementStyle {
				color: Color::rgb(50, 70, 110),
				border: Border::rgbf(80, 110, 160, 0.005),
				text_color: Color::rgb(180, 200, 220).o(),
			},
			worst_button: ElementStyle {
				color: Color::rgb(120, 40, 40),
				border: Border::rgbf(160, 60, 60, 0.005),
				text_color: Color::rgb(255, 180, 180).o(),
			},
			okay_button: ElementStyle {
				color: Color::rgb(50, 255, 70),
				border: Border::rgbf(30, 200, 50, 0.005),
				text_color: Color::rgb(25, 100, 30).o(),
			},
			deny_button: ElementStyle {
				color: Color::rgb(255,40,80),
				border: Border::rgbf(200,35,65, 0.005),
				text_color: Color::rgb(100,15,30).o(),
			},
			buttons: VariantStyles {
				basic: ElementStyle {
					color: Color::rgb(40, 50, 80),
					border: Border::rgbf(70, 90, 130, 0.005),
					text_color: Color::rgb(180, 200, 220).o(),
				},
				nice: ElementStyle {
					color: Color::rgb(40, 40, 60),  // Dark gray-blue
					border: Border::rgbf(70, 90, 120, 0.005),
					text_color: Color::rgb(150, 170, 200).o(), // Light blue-gray
				},
				bad: ElementStyle {
					color: Color::rgb(120, 40, 40),
					border: Border::rgbf(160, 60, 60, 0.005),
					text_color: Color::rgb(255, 180, 180).o(),
				},
				extra: Some(ElementStyle {
					color: Color::rgb(40, 40, 60),
					border: Border::rgbf(70, 90, 120, 0.005),
					text_color: Color::rgb(150, 170, 200).o(),
				}),
			},
			panels: VariantStyles {
				basic: ElementStyle {
					color: Color::rgb(25, 25, 40),
					border: Border::rgbf(60, 70, 100, 0.01),
					text_color: None,
				},
				nice: ElementStyle {
					color: Color::rgb(30, 30, 45),
					border: Border::rgbf(80, 100, 140, 0.008),
					text_color: None,
				},
				bad: ElementStyle {
					color: Color::rgb(20, 20, 35),
					border: Border::rgbf(60, 80, 120, 0.01),
					text_color: None,
				},
				extra: None,
			},
			labels: VariantStyles {
				basic: ElementStyle {
					color: Color::rgb(30, 30, 45),
					border: Border::NONE, // no background for this one, because the color is not rendered, without border it is just a text with transparent background
					text_color: Color::rgb(180, 200, 220).o(),
				},
				nice: ElementStyle {
					color: Color::rgb(30, 30, 45),
					border: Border::rgbf(80, 100, 140, 0.008),
					text_color: Color::rgb(180, 200, 220).o(),
				},
				bad: ElementStyle {
					color: Color::rgb(30, 30, 45),
					border: Border::rgbf(80, 100, 140, 0.008),
					text_color: Color::rgb(120, 140, 180).o(),
				},
				extra: Some(ElementStyle {
					color: Color::rgb(30, 30, 45),
					border: Border::rgbf(80, 100, 140, 0.008),
					text_color: Color::rgb(180, 200, 220).o(),
				}),
			},
			images: VariantStyles {
				basic: ElementStyle {
					color: Color::rgb(255, 255, 255), // White for no color modification
					border: Border::rgbf(80, 120, 180, 0.008),
					text_color: None,
				},
				nice: ElementStyle {
					color: Color::rgb(255, 255, 255),
					border: Border::rgbf(80, 120, 180, 0.008),
					text_color: None,
				},
				bad: ElementStyle {
					color: Color::rgb(255, 255, 255),
					border: Border::rgbf(80, 120, 180, 0.008),
					text_color: None,
				},
				extra: None,
			},
			checkboxs: VariantStyles {
				basic: ElementStyle {
					color: Color::rgb(40, 50, 80),
					border: Border::rgbf(70, 90, 130, 0.005),
					text_color: Color::rgb(30, 30, 45).o(),
				},
				nice: ElementStyle {
					color: Color::rgb(40, 50, 80),
					border: Border::rgbf(80, 110, 160, 0.005),
					text_color: Color::rgb(30, 30, 45).o(),
				},
				bad: ElementStyle {
					color: Color::rgb(120, 40, 40),
					border: Border::rgbf(160, 60, 60, 0.005),
					text_color: Color::rgb(30, 30, 45).o(),
				},
				extra: None,
			},
			sliders: VariantStyles {
				basic: ElementStyle {
					color: Color::rgb(40, 50, 80),
					border: Border::rgbf(70, 90, 130, 0.005),
					text_color: Color::rgb(180, 200, 220).o(),
				},
				nice: ElementStyle {
					color: Color::rgb(40, 50, 80),
					border: Border::rgbf(80, 110, 160, 0.005),
					text_color: Color::rgb(180, 200, 220).o(),
				},
				bad: ElementStyle {
					color: Color::rgb(120, 40, 40),
					border: Border::rgbf(160, 60, 60, 0.005),
					text_color: Color::rgb(255, 180, 180).o(),
				},
				extra: None,
			},
			inputs: VariantStyles {
				basic: ElementStyle {
					color: Color::rgb(40, 50, 70),
					border: Border::rgbf(80, 100, 140, 0.005),
					text_color: Color::rgb(200, 220, 240).o(),
				},
				nice: ElementStyle {
					color: Color::rgb(40, 50, 70),
					border: Border::rgbf(80, 100, 140, 0.005),
					text_color: Color::rgb(200, 220, 240).o(),
				},
				bad: ElementStyle {
					color: Color::rgb(60, 40, 40),
					border: Border::rgbf(140, 80, 80, 0.005),
					text_color: Color::rgb(255, 200, 200).o(),
				},
				extra: None,
			},
			dividers: VariantStyles {
				basic: ElementStyle {
					color: Color::rgb(220, 240, 255),
					border: Border::NONE, // Dividers typically don't have borders
					text_color: None,
				},
				nice: ElementStyle {
					color: Color::rgb(220, 240, 255),
					border: Border::NONE,
					text_color: None,
				},
				bad: ElementStyle {
					color: Color::rgb(255, 100, 100),
					border: Border::NONE,
					text_color: None,
				},
				extra: None,
			},
			inv: InventoryConfig::default(),
		}
	}
}
