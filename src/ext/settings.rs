
use crate::ext::config::{InvLayout, UITheme, InvConfig};


/// I implement manual default for this even if it is useless
/// because i want to eliminate edge cases as much as possible (like getting a transparent UI or speed of 0 for audio ...)

pub struct Settings {
	pub window_config: WindowConfig,
	pub ui_theme: UITheme,
	pub inv_config: InvConfig,
	pub inv_layout: InvLayout,
	pub music_settings: MusiConfig,
}
impl Settings {
	#[inline] pub const fn default() -> Self {
		Self{
			window_config: WindowConfig::default(),
			ui_theme: UITheme::default(),
			inv_config: InvConfig::default(),
			inv_layout: InvLayout::default(),

			music_settings: MusiConfig::default(),
		}
	}
	pub fn remake_window_config(&mut self, size: winit::dpi::PhysicalSize<u32>) {
		self.window_config = WindowConfig::new(size);
	}
}

/// mainly everything goes from 1 = normal and more is more, less is less
pub struct MusiConfig {
	pub bg_speed: f32,
	pub fg_speed: f32,
	pub main_speed: f32,
	pub use_random: bool, // sometimes i heard that randomizing pitch and speed from 0.9 to 1.1 is nicer to the ear than playing the same sound repeatedly
	// so this is the config for that (setting the random_value to 0.1 will give the mentioned results)
	pub random_value: f32,

	pub bg_volume: f32,
	pub fg_volume: f32,
	pub main_volume: f32,

	pub bg_music: &'static str,
}

impl MusiConfig {
	#[inline] pub const fn default() -> Self {
		Self {
			bg_speed: 1.,
			fg_speed: 1.,
			main_speed: 1.,
			use_random: true,
			random_value: 0.1,

			bg_volume: 0.5, // Lower volume for background music
			fg_volume: 0.7, // Higher volume for UI sounds
			main_volume: 0.8,

			bg_music: "background_music.ogg",
		}
	}
}


pub struct WindowConfig {
	window_title: &'static str,
	window_size: winit::dpi::PhysicalSize<f32>,
	min_window_size: winit::dpi::PhysicalSize<f32>,
	window_position: winit::dpi::PhysicalPosition<f32>,
	theme: Option<winit::window::Theme>
}

impl WindowConfig {
	#[inline] pub const fn window_title(&self) -> &str { &self.window_title }
	#[inline] pub const fn window_size(&self) -> &winit::dpi::PhysicalSize<f32> { &self.window_size }
	#[inline] pub const fn min_window_size(&self) -> &winit::dpi::PhysicalSize<f32> { &self.min_window_size }
	#[inline] pub const fn window_position(&self) -> &winit::dpi::PhysicalPosition<f32> { &self.window_position }
	#[inline] pub const fn theme(&self) -> &Option<winit::window::Theme> { &self.theme }

	#[inline] pub const fn default() -> Self {
		Self {
			window_title: "Default App",
			window_size: winit::dpi::PhysicalSize::new(1280.0, 720.0),
			min_window_size: winit::dpi::PhysicalSize::new(600.0, 400.0),
			window_position: winit::dpi::PhysicalPosition::new(100.0,100.0),
			theme: Some(winit::window::Theme::Dark),
		}
	}
	#[inline] pub const fn new(size: winit::dpi::PhysicalSize<u32>) -> Self {
		let width:f32 = 1280.0; let height:f32 = 720.0;
		let x:f32 = (size.width as f32 - width) / 2.0;
		let y:f32 = (size.height as f32 - height) / 2.0;
		Self {
			window_title: "Rusticubes",
			window_size: winit::dpi::PhysicalSize::new(width, height),
			min_window_size: winit::dpi::PhysicalSize::new(width/3.0, height/3.0),
			window_position: winit::dpi::PhysicalPosition::new(x,y),
			..Self::default()
		}
	}
}