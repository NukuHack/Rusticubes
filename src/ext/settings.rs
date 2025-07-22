
use crate::ext::config::InvLayout;
use crate::ext::config::UITheme;
use crate::ext::config::InvConfig;
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;


static SETTINGS: AtomicPtr<Settings> = AtomicPtr::new(ptr::null_mut());

// Helper function to safely access the Settings pointer
#[inline] pub fn get_ptr() -> Option<&'static mut Settings> {
	let system_ptr = SETTINGS.load(Ordering::Acquire);
	if system_ptr.is_null() {
		None
	} else {
		unsafe { Some(&mut *system_ptr) }
	}
}
#[inline] pub fn init_settings() {
	let system = Box::new(Settings::default());
	
	let old_ptr = SETTINGS.swap(Box::into_raw(system), Ordering::AcqRel);
	if !old_ptr.is_null() {
		unsafe { let _ = Box::from_raw(old_ptr); }
	}
}

/// I implement manual default for this even if it is useless
/// because i want to eliminate edge cases as much as possible (like getting a transparent UI or speed of 0 for audio ...)

pub struct Settings {
	window_config: WindowConfig,
	ui_theme: UITheme,
	inv_config: InvConfig,
	inv_layout: InvLayout,


	musid_settings: MusiConfig,
}
impl Settings {
	#[inline] pub const fn default() -> Self {
		Self{
			window_config: WindowConfig::default(),
			ui_theme: UITheme::default(),
			inv_config: InvConfig::default(),
			inv_layout: InvLayout::default(),

			musid_settings: MusiConfig::default(),
		}
	}
}

/// mainly everything goes from 1 = normal and more is more, less is less
pub struct MusiConfig {
	bg_speed: f32,
	foreground_speed: f32,
	main_speed: f32,
	use_random: bool, // sometimes i heard that randomizing pitch and speed from 0.9 to 1.1 is nicer to the ear than playing the same sound repeatedly
	// so this is the config for that (setting the random_value to 0.1 will give the mentioned results)
	random_value: f32,

	bg_volume: f32,
	foreground_volume: f32,
	main_volume: f32,

	bg_music: &'static str,
}

impl MusiConfig {
	#[inline] pub const fn default() -> Self {
		Self {
			bg_speed: 1.,
			foreground_speed: 1.,
			main_speed: 1.,
			use_random: true,
			random_value: 0.1,

			bg_volume: 1.,
			foreground_volume: 1.,
			main_volume: 1.,

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