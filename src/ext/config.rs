
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
#[derive(Debug, Clone)]
pub struct InventoryConfig {
	pub slot_size: f32,
	pub slot_padding: f32,
	pub slot_border_width: f32,
	pub section_spacing: f32,
	pub panel_padding: f32,
	pub colors: InventoryColors,
}

#[derive(Debug, Clone)]
pub struct InventoryColors {
	pub panel_bg: PanelStyle,
	pub inventory_slot: PanelStyle,
	pub hotbar_slot: PanelStyle,
	pub armor_slot: PanelStyle,
	pub storage_slot: PanelStyle,
	pub crafting_slot: PanelStyle,
}

impl Default for InventoryConfig {
	fn default() -> Self {
		Self {
			slot_size: 0.08,
			slot_padding: 0.02,
			slot_border_width: 0.003,
			section_spacing: 0.12,
			panel_padding: 0.05,
			colors: InventoryColors::default(),
		}
	}
}

impl Default for InventoryColors {
	fn default() -> Self {
		let panel_bg = PanelStyle{
			color: Color::rgb(25, 25, 40),
			border: Border::rgbf(60, 60, 90, 0.005)
		};
		let inventory_slot = PanelStyle{
			color: Color::rgb(40, 40, 60),
			border: Border::rgbf(80, 80, 120, 0.005)
		};
		let hotbar_slot = PanelStyle{
			color: Color::rgb(50, 50, 70),
			border: Border::rgbf(90, 90, 130, 0.005)
		};
		let armor_slot = PanelStyle{
			color: Color::rgb(60, 60, 80),
			border: Border::rgbf(100, 100, 140, 0.005)
		};
		let storage_slot = PanelStyle{
			color: Color::rgb(40, 40, 60),
			border: Border::rgbf(80, 80, 120, 0.005)
		};
		let crafting_slot = PanelStyle{
			color: Color::rgb(60, 80, 60),
			border: Border::rgbf(80, 120, 80, 0.005)
		};
		Self {
			panel_bg,
			inventory_slot,
			hotbar_slot,
			armor_slot,
			storage_slot,
			crafting_slot,
		}
	}
}





#[derive(Debug, Clone, Copy)]
pub enum ElementVariant {
	Basic,
	Nice,
	Bad,
	Extra, // the extra is just used for cargo package showing not much else
}
#[derive(Debug, Clone, Copy)]
pub struct TextElementStyle {
	pub color: Color,
	pub border: Border,
	pub text_color: Color,
}
#[derive(Debug, Clone, Copy)]
pub struct PanelStyle {
	pub color: Color,
	pub border: Border,
}
#[derive(Debug, Clone, Copy)]
pub struct ButtonStyle {// button color will be used for multi-state button too
	pub color: Color,
	pub border: Border,
	pub text_color: Color,
}
#[derive(Debug, Clone, Copy)]
pub struct LabelStyle {
	pub color: Color, // label's color is not rendered
	pub border: Border,
	pub text_color: Color,
}
#[derive(Debug, Clone, Copy)]
pub struct ImageStyle {// the image color will be used for animations too because that is just a "changing image" so yeah
	pub color: Color, // image's color is and extra color on the image, most of the times leaving this on white is the best
	pub border: Border,
}
#[derive(Debug, Clone, Copy)]
pub struct CheckboxStyle {
	pub color: Color,
	pub border: Border,
	pub text_color: Color, // only used for optional text
}
#[derive(Debug, Clone, Copy)]
pub struct SliderStyle {
	pub color: Color, // used for the track color
	pub border: Border,
	pub text_color: Color, // used for the actual slider's handle
}
#[derive(Debug, Clone, Copy)]
pub struct InputStyle {
	pub color: Color,
	pub border: Border,
	pub text_color: Color,
}
#[derive(Debug, Clone, Copy)]
pub struct DividerStyle { // currently only used as a makeshift crosshair in game
	pub color: Color,
	pub border: Border,
}

#[derive(Debug, Clone)]
pub struct VariantStyles<T> {
	pub basic: T,
	pub nice: T,
	pub bad: T,
	pub extra: Option<T>, // Extra is optional since not all elements need it
}

#[derive(Debug, Clone)]
pub struct UITheme {
	pub bg_panel: PanelStyle,
	pub title_label: TextElementStyle,
	pub best_button: ButtonStyle,
	pub worst_button: ButtonStyle,
	
	pub buttons: VariantStyles<ButtonStyle>,
	pub panels: VariantStyles<PanelStyle>,
	pub labels: VariantStyles<LabelStyle>,
	pub images: VariantStyles<ImageStyle>,
	pub checkboxs: VariantStyles<CheckboxStyle>, // i know it should be called checkboxes but this is ... okay
	pub sliders: VariantStyles<SliderStyle>,
	pub inputs: VariantStyles<InputStyle>,
	pub dividers: VariantStyles<DividerStyle>,

	pub inv_config: InventoryConfig,
}

impl Default for UITheme {
	fn default() -> Self {
		// Common colors extracted from the UI setup
		const TITLE_BG: Color = Color::rgb(30, 30, 45);
		const LIGHT_TEXT: Color = Color::rgb(180, 200, 220);
		const BLUE_TEXT: Color = Color::rgb(180, 180, 220);
		const INPUT_BG: Color = Color::rgb(40, 50, 70);
		const INPUT_TEXT: Color = Color::rgb(200, 220, 240);
		// Button colors
		const BUTTON_BASIC: Color = Color::rgb(40, 50, 80);
		const BUTTON_NICE: Color = Color::rgb(50, 70, 110);
		const BUTTON_BAD: Color = Color::rgb(120, 40, 40);
		// Text colors
		const TEXT_BASIC: Color = Color::rgb(180, 200, 220);
		const TEXT_BAD: Color = Color::rgb(255, 180, 180);
		// Border colors
		const BORDER_BASIC: Color = Color::rgb(70, 90, 130);
		const BORDER_NICE: Color = Color::rgb(80, 110, 160);
		const BORDER_BAD: Color = Color::rgb(160, 60, 60);
		const BORDER_TITLE: Color = Color::rgb(80, 100, 140);
		
		Self {
			bg_panel: PanelStyle {
				color: Color::rgb(15, 15, 25),
				border: Border::rgb(0, 0, 0), // No border for background
			},
			title_label: TextElementStyle {
				color: TITLE_BG,
				border: Border::colf(BORDER_TITLE, 0.008),
				text_color: LIGHT_TEXT,
			},
			best_button: ButtonStyle {
				color: BLUE_TEXT*0.8,
				border: Border::colf(BLUE_TEXT*0.8, 0.009),
				text_color: BLUE_TEXT,
			},
			worst_button: ButtonStyle {
				color: TEXT_BAD*0.3,
				border: Border::colf(TEXT_BAD*0.3, 0.009),
				text_color: TEXT_BAD,
			},
			buttons: VariantStyles {
				basic: ButtonStyle {
					color: BUTTON_BASIC,
					border: Border::colf(BORDER_BASIC, 0.005),
					text_color: TEXT_BASIC,
				},
				nice: ButtonStyle {
					color: BUTTON_NICE,
					border: Border::colf(BORDER_NICE, 0.005),
					text_color: TEXT_BASIC,
				},
				bad: ButtonStyle {
					color: BUTTON_BAD,
					border: Border::colf(BORDER_BAD, 0.005),
					text_color: TEXT_BAD,
				},
				extra: Some(ButtonStyle {
					color: Color::rgb(40, 40, 60),
					border: Border::rgbf(70, 90, 120, 0.005),
					text_color: Color::rgb(150, 170, 200),
				}),
			},
			panels: VariantStyles {
				basic: PanelStyle {
					color: Color::rgb(25, 25, 40),
					border: Border::rgbf(60, 70, 100, 0.01),
				},
				nice: PanelStyle {
					color: TITLE_BG,
					border: Border::colf(BORDER_TITLE, 0.008),
				},
				bad: PanelStyle {
					color: Color::rgb(20, 20, 35),
					border: Border::rgbf(60, 80, 120, 0.01),
				},
				extra: None,
			},
			labels: VariantStyles {
				basic: LabelStyle {
					color: TITLE_BG,
					border: Border::rgb(0, 0, 0), // Labels typically don't have borders
					text_color: LIGHT_TEXT,
				},
				nice: LabelStyle {
					color: TITLE_BG,
					border: Border::colf(BORDER_TITLE, 0.008),
					text_color: LIGHT_TEXT,
				},
				bad: LabelStyle {
					color: TITLE_BG,
					border: Border::rgb(0, 0, 0),
					text_color: Color::rgb(120, 140, 180),
				},
				extra: None,
			},
			images: VariantStyles {
				basic: ImageStyle {
					color: Color::rgb(255, 255, 255), // White for no color modification
					border: Border::rgbf(80, 120, 180, 0.008),
				},
				nice: ImageStyle {
					color: Color::rgb(255, 255, 255),
					border: Border::rgbf(80, 120, 180, 0.008),
				},
				bad: ImageStyle {
					color: Color::rgb(255, 255, 255),
					border: Border::rgbf(80, 120, 180, 0.008),
				},
				extra: None,
			},
			checkboxs: VariantStyles {
				basic: CheckboxStyle {
					color: BUTTON_BASIC,
					border: Border::colf(BORDER_BASIC, 0.005),
					text_color: TEXT_BASIC,
				},
				nice: CheckboxStyle {
					color: BUTTON_NICE,
					border: Border::colf(BORDER_NICE, 0.005),
					text_color: TEXT_BASIC,
				},
				bad: CheckboxStyle {
					color: BUTTON_BAD,
					border: Border::colf(BORDER_BAD, 0.005),
					text_color: TEXT_BAD,
				},
				extra: None,
			},
			sliders: VariantStyles {
				basic: SliderStyle {
					color: BUTTON_BASIC, // Track color
					border: Border::colf(BORDER_BASIC, 0.005),
					text_color: TEXT_BASIC, // Handle color
				},
				nice: SliderStyle {
					color: BUTTON_NICE,
					border: Border::colf(BORDER_NICE, 0.005),
					text_color: TEXT_BASIC,
				},
				bad: SliderStyle {
					color: BUTTON_BAD,
					border: Border::colf(BORDER_BAD, 0.005),
					text_color: TEXT_BAD,
				},
				extra: None,
			},
			inputs: VariantStyles {
				basic: InputStyle {
					color: INPUT_BG,
					border: Border::rgbf(80, 100, 140, 0.005),
					text_color: INPUT_TEXT,
				},
				nice: InputStyle {
					color: INPUT_BG,
					border: Border::rgbf(80, 100, 140, 0.005),
					text_color: INPUT_TEXT,
				},
				bad: InputStyle {
					color: Color::rgb(60, 40, 40),
					border: Border::rgbf(140, 80, 80, 0.005),
					text_color: Color::rgb(255, 200, 200),
				},
				extra: None,
			},
			dividers: VariantStyles {
				basic: DividerStyle {
					color: Color::rgb(220, 240, 255),
					border: Border::rgb(0, 0, 0), // Dividers typically don't have borders
				},
				nice: DividerStyle {
					color: Color::rgb(220, 240, 255),
					border: Border::rgb(0, 0, 0),
				},
				bad: DividerStyle {
					color: Color::rgb(255, 100, 100),
					border: Border::rgb(0, 0, 0),
				},
				extra: None,
			},
			inv_config: InventoryConfig::default(),
		}
	}
}