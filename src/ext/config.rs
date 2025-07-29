
use crate::Vec3;
use crate::ui::inventory::AreaType;
use crate::ext::color::{Color, Border};


// note that these are currently offsets from real pos, might change them to actual pos later on
// by defaukt everything refers to "in game" values and not in inventory ones
pub struct InvLayout {
	pub hotbar: (f32,f32),
	pub armor: (f32,f32),
	pub inv: (f32,f32),
}
impl InvLayout {
	#[inline] pub const fn default() -> Self {
		Self {
			hotbar: (0.,-0.8),
			armor: (0.,0.),
			inv: (0.,0.),
		}
	}
}


// Camera configuration
#[derive(Debug, Clone, Copy)]
pub struct CameraConfig {
	pub offset: Vec3,
	pub rotation: Vec3,
	pub fovy: f32,
	pub znear: f32,
	pub zfar: f32,
	pub speed: f32,
	pub sensitivity: f32,
	pub run_multiplier: f32,
	pub smoothness: f32,
	pub min_fov: f32,
	pub max_fov: f32,
}

impl CameraConfig {
	#[inline] pub const fn new(offset: Vec3) -> Self {
		Self {
			offset,
			rotation: Vec3::ZERO, // Looking along negative X axis
			fovy: std::f32::consts::FRAC_PI_2, // 90 degrees in radians
			znear: 0.01,
			zfar: 500.0,
			speed: 20.0,
			sensitivity: 0.4,
			run_multiplier: 2.5,
			smoothness: 5.0,
			min_fov: std::f32::consts::FRAC_PI_6 / 2f32, // 15 degrees
			max_fov: std::f32::consts::FRAC_PI_2 * 1.8, // 162 degrees
		}
	}

	#[inline] pub const fn default() -> Self { Self::new(Vec3::ZERO) }
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
pub struct ElementStyle {
	pub color: Color,
	pub border: Border,
	pub text_color: Option<Color>,
}
impl ElementStyle {
	#[inline] pub const fn text_color(&self) -> Color {
		if let Some(color) = self.text_color { color } else { self.color }
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
	#[inline] pub fn extra(&self) -> ElementStyle {
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
}

#[derive(Debug, Clone)]
pub struct InvConfig {
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
impl InvConfig {
	#[inline] pub const fn get_style(&self, area_type : AreaType) -> &ElementStyle {
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


	#[inline] pub const fn default() -> Self {
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

impl UITheme {
	#[inline] pub const fn default() -> Self {		
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
				extra: Some(ElementStyle {
					color: Color::rgb(255, 100, 100),
					border: Border::NONE,
					text_color: None,
				}),
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
					border: Border::NONE,
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
		}
	}
}
