

/// Block and Chunk related
pub mod block {
	/// main block and chunk struct and basic fn
	pub mod main;
	/// chunk and block coords struct and all fn
	pub mod math;
	/// block & chunk interaction and world modification
	pub mod extra;
}
/// Debug, test related
#[cfg(test)]
pub mod debug {
	pub mod network;
	pub mod world;
	pub mod metadata;
	pub mod json_serial;
	pub mod serialize_item;
	pub mod physics;
}
// Extra things that did not fit anywhere else
pub mod ext {
	// audio manager, in extra thread
	pub mod audio;
	// basic configs
	pub mod config;
	// main settings
	pub mod settings;
	// all the pointers and stuff for globar variables
	pub mod ptr;
	// basic struct used for debugging and profiling
	pub mod stopwatch;
	// memory management mainly focusing on memory clean up
	pub mod memory;
}
/// Handles file system operations, including loading resources, reading from disk, and serialization.
pub mod fs {
	/// Compiled resources embedded in the binary.
	pub mod rs;
	/// File system operations (reading/writing to disk).
	pub mod fs;
	/// Custom JSON parser (alternative to `serde_json`).
	pub mod json;
	/// Binary serialization utilities.
	pub mod binary;
}
// Game related, instance related
pub mod game {
	// main camera and player impl.
	pub mod player;
	// game-state with seed and stuff
	pub mod state;
}
/// Item + Inventory and related stuffs
pub mod item {
	// Item and Itemstack body, inventory basics
	pub mod items;
	/// Item related things what will not change at runtime
	pub mod item_lut;
	/// Binary serialization and de-serialization for items
	pub mod item_binary;
	/// Json de-serialization for items
	pub mod item_json;
	/// Basic corner-stone of the item system
	pub mod material;
	/// Main inventory implementation, and Item grid impl.
	pub mod inventory;
	/// the rendering part of the inventory and the items
	pub mod ui_inventory;
	/// the recipes, for now only for crafting
	pub mod recipes;
}
// Modding related
pub mod mods {
	// mod loading and wasm sandbox 
	pub mod api;
	// this is an overlay made by mods so they would execute instead of the real rust functions
	pub mod over;
}
// Network related
pub mod network {
	// the networking system
	pub mod api;
	// the networking system
	pub mod discovery;
	// the networking system and extra utilities for basic stuff
	pub mod types;
}
/// Physics stuff like gravity ...
pub mod physic {
	/// Axis-aligned bounding boxes.
	pub mod aabb;
	/// Physics bodies.
	pub mod body;
}
// Rendering and related
pub mod render {
	pub mod meshing;
	pub mod texture;
	pub mod pipeline;
	pub mod world;
	pub mod skybox;
}
// Ui and related
pub mod ui {
	pub mod element;
	pub mod render;
	pub mod manager;
	pub mod settings;
	pub mod setup;
	pub mod dialog;
	pub mod events;
	pub mod text;
}
/// Utility things, like helper Structs
pub mod utils {
	/// Input handling (keyboard/mouse).
	pub mod input;
	/// Math utilities (Noise gen, lerping).
	pub mod math;
	// my custom color struct with quick init
	pub mod color;
	/// String helpers (For compile and runtime strings).
	pub mod string;
	/// Cursor state (cursor change and locking).
	pub mod cursor;
	/// Time formatting is a pretty struct.
	pub mod time;
}
// World related, tiny bit rendering and game related
pub mod world {
	pub mod main;
	pub mod manager;
	pub mod serialize;
	pub mod handler;
	pub mod threading;
}
/// Main event handler (focused on the user input)
mod event_handler;

use crate::ext::ptr;
use crate::game::player;
use std::sync::atomic::Ordering;
use glam::Vec3;
use std::iter::Iterator;
use winit::{
	event::Event,
	window::Window
};

pub struct State<'a> {
	window: &'a Window,
	render_context: RenderContext<'a>,
	previous_frame_time: std::time::Instant,
	input_system: utils::input::InputSystem,
	pipeline: render::pipeline::Pipeline,
	ui_manager: ui::manager::UIManager,
	texture_manager: render::texture::TextureManager,
	is_world_running: bool,
}

pub struct RenderContext<'a> {
	surface: wgpu::Surface<'a>,
	device: wgpu::Device,
	queue: wgpu::Queue,
	surface_config: wgpu::SurfaceConfiguration,
	size: winit::dpi::PhysicalSize<u32>,
	layouts: Box<[wgpu::BindGroupLayout]>,
	skybox: render::skybox::Skybox,
}

impl<'a> State<'a> {
	#[inline]
	async fn new(window: &'a Window) -> Self {
		let size: winit::dpi::PhysicalSize<u32> = window.inner_size();
		let instance: wgpu::Instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
			#[cfg(not(target_arch = "wasm32"))]
			backends: wgpu::Backends::PRIMARY,
			#[cfg(target_arch = "wasm32")]
			backends: wgpu::Backends::GL,
			..Default::default()
		});
		let surface: wgpu::Surface = instance.create_surface(window).unwrap();
		let adapter: wgpu::Adapter = instance
			.enumerate_adapters(wgpu::Backends::all())
			.into_iter()
			.filter(|adapter| adapter.is_surface_supported(&surface))
			.next()
			.expect("No suitable GPU adapter found");

		println!("Bind-group limits: {:?} if smaller than 4 it will crash", adapter.limits().max_bind_groups);
		println!("Max texture array layers: {} if smaller than 256 it will crash", adapter.limits().max_texture_array_layers);
		let required_limits = wgpu::Limits {
			max_texture_array_layers: 256,
			max_bind_groups: 4,
			..wgpu::Limits::default()
		};

		let (device, queue): (wgpu::Device, wgpu::Queue) = adapter
			.request_device(
				&wgpu::DeviceDescriptor {
					required_features: wgpu::Features::SHADER_INT64,
					required_limits,
					..Default::default()
				},
				None,
			)
			.await.unwrap();

		let surface_caps: wgpu::SurfaceCapabilities = surface.get_capabilities(&adapter);
		let surface_format: wgpu::TextureFormat = surface_caps.formats.iter()
			.copied()
			.find(|f| f.is_srgb())
			.unwrap_or(surface_caps.formats[0]);
		let present_mode: wgpu::PresentMode = surface_caps.present_modes.iter().copied()
			.find(|mode| *mode == wgpu::PresentMode::Fifo)
			.unwrap_or(surface_caps.present_modes[0]);

		let surface_config = wgpu::SurfaceConfiguration {
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
			format: surface_format,
			width: size.width,
			height: size.height,
			present_mode,
			alpha_mode: surface_caps.alpha_modes[0],
			view_formats: vec![],
			desired_maximum_frame_latency: 2,
		};

		surface.configure(&device, &surface_config);

		let layouts = make_layout(&device);
		/*
		&texture_bind_group_layout,
		&camera_bind_group_layout,
		&chunk_bind_group_layout,
		&skybox_bind_group_layout,
		&post_bind_group_layout,
		*/

		let texture_manager = render::texture::TextureManager::new(&device, &queue, &surface_config, &layouts[0], &layouts[4]);

		let pipeline = render::pipeline::Pipeline::new(&device, &surface_config, &layouts);

		let mut ui_manager = ui::manager::UIManager::new(&device, &surface_config, &queue);
		ui_manager.setup_ui();

		let skybox = render::skybox::Skybox::new(&device, &queue, &layouts[3],"basic_skybox.jpg").expect("basic skybox should work");
		
		let render_context: RenderContext = RenderContext{
			surface,
			device,
			queue,
			surface_config,
			size,
			layouts,
			skybox,
		};

		Self {
			window,
			render_context,
			previous_frame_time: std::time::Instant::now(),
			input_system: utils::input::InputSystem::default(),
			pipeline,
			texture_manager,
			ui_manager,
			is_world_running: false,
		}
	}
	#[inline]
	pub fn window(&self) -> &Window {
		self.window
	}
	#[inline]
	pub fn surface(&self) -> &wgpu::Surface<'a> {
		&self.render_context.surface
	}
	#[inline]
	pub fn device(&self) -> &wgpu::Device {
		&self.render_context.device
	}
	#[inline]
	pub fn queue(&self) -> &wgpu::Queue {
		&self.render_context.queue
	}
	#[inline]
	pub fn surface_config(&self) -> &wgpu::SurfaceConfiguration {
		&self.render_context.surface_config
	}
	pub fn size(&self) -> &winit::dpi::PhysicalSize<u32> {
		&self.render_context.size
	}
	#[inline]
	pub fn previous_frame_time(&self) -> &std::time::Instant {
		&self.previous_frame_time
	}
	#[inline]
	pub fn pipeline(&self) -> &render::pipeline::Pipeline {
		&self.pipeline
	}
	#[inline]
	pub fn texture_manager(&self) -> &render::texture::TextureManager {
		&self.texture_manager
	}
	#[inline]
	pub fn ui_manager(&self) -> &ui::manager::UIManager {
		&self.ui_manager
	}
	#[inline]
	pub fn skybox(&self) -> &render::skybox::Skybox {
		&self.render_context.skybox
	}




	#[inline]
	pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) -> bool {
		if new_size.width <= 0 || new_size.height <= 0 { return false; }

		self.render_context.size = new_size;
		self.render_context.surface_config.width = new_size.width;
		self.render_context.surface_config.height = new_size.height;
		if self.is_world_running {
			ptr::get_gamestate().player_mut().resize(new_size);
		}
		// Clone the values to avoid holding borrows
		self.render_context.surface.configure(self.device(), self.surface_config());
		*self.texture_manager.depth_texture_mut() = render::texture::create_depth_texture(self.device(), self.surface_config(),"depth_texture");
		
		true
	}
	#[inline]
	pub fn update(&mut self) {
		let current_time: std::time::Instant = std::time::Instant::now();
		let delta_seconds: f32 = (current_time - self.previous_frame_time).as_secs_f32();
		self.previous_frame_time = current_time;
		network::api::update_network(); // theoretically it should run in other thread so calling it each frame should not be a problem ...
		
		if self.is_world_running {
			let movement_delta = {
				let player = &mut ext::ptr::get_gamestate().player_mut();
				player.update(delta_seconds, self.queue())
			};

			// Update both player and camera positions in one operation
			{
				let player = &mut ext::ptr::get_gamestate().player_mut();
				player.append_position(movement_delta);
			}
		}
		if self.ui_manager.visibility {
			self.ui_manager.update(&self.render_context.device, &self.render_context.queue, delta_seconds);
		}
	}
	#[inline]
	pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
		render::pipeline::render_all(self)
	}
}

//#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
#[inline]
pub async fn run() {
	let event_loop: winit::event_loop::EventLoop<()> = winit::event_loop::EventLoop::new().unwrap();
	let monitor: winit::monitor::MonitorHandle = event_loop.primary_monitor().expect("No primary monitor found!");
	let monitor_size: winit::dpi::PhysicalSize<u32> = monitor.size(); // Monitor size in physical pixels

	ext::ptr::init_settings();
	let settings = ext::ptr::get_settings();
	settings.remake_window_config(monitor_size);

	// Initialize once at startup
	ext::audio::init_audio().expect("Failed to initialize audio");
	ext::audio::set_bg(settings.music_settings.bg_music);

	
	let config = &settings.window_config;
	let window_raw: Window = winit::window::WindowBuilder::new()
		.with_title(&*config.window_title())
		.with_inner_size(*config.window_size())
		.with_min_inner_size(*config.min_window_size())
		.with_position(*config.window_position())
		.with_window_icon(fs::rs::load_main_icon())
		.with_theme(*config.theme())
		.with_active(true)
		.build(&event_loop)
		.unwrap();

	// Set the window to be focused immediately
	window_raw.focus_window();
	window_raw.set_visible(true);

	// Store the window pointer
	ext::ptr::WINDOW_PTR.store(Box::into_raw(Box::new(window_raw)), Ordering::Release);
	let window_ref = ext::ptr::get_window();
		
	let state = State::new(window_ref).await;

	// Store the state pointer
	ext::ptr::STATE_PTR.store(Box::into_raw(Box::new(state)), Ordering::Release);

	#[cfg(debug_assertions)] {
		if let Err(e) = mods::api::main() {
			println!("⚠Error modding: {}", e);
		}
		if let Err(e) = mods::over::main() {
			println!("💥Error mod function override: {}", e);
		}
	}

	// Post-init cleanup
	ext::memory::light_trim();
	ext::memory::hard_clean(Some(ext::ptr::get_state().device()));

	event_loop.run(move |event, control_flow| {
		if ext::ptr::is_closed() {
			ext::ptr::cleanup_resources();
			control_flow.exit();
			return;
		}
		let state = ext::ptr::get_state();
		let Event::WindowEvent { event, window_id } = &event else { return; };
		//let Event::DeviceEvent { event, device_id } = &event else { return; };

		if *window_id != state.window().id() { return; };

		state.handle_events(event);

		if state.is_world_running {
			block::extra::update_full_world();
		}
	}).expect("Event loop error");
}





fn make_layout(device: &wgpu::Device) -> Box<[wgpu::BindGroupLayout]> {
	let post_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
		label: Some("Post Processing Bind Group Layout"),
		entries: &[
			// Texture
			wgpu::BindGroupLayoutEntry {
				binding: 0,
				visibility: wgpu::ShaderStages::FRAGMENT,
				ty: wgpu::BindingType::Texture {
					sample_type: wgpu::TextureSampleType::Float { filterable: true },
					view_dimension: wgpu::TextureViewDimension::D2,
					multisampled: false,
				},
				count: None,
			},
			// Sampler
			wgpu::BindGroupLayoutEntry {
				binding: 1,
				visibility: wgpu::ShaderStages::FRAGMENT,
				ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
				count: None,
			},
		],
	});
	let texture_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
		label: Some("texture_array_bind_group_layout"),
		entries: &[
			wgpu::BindGroupLayoutEntry {
				binding: 0,
				visibility: wgpu::ShaderStages::FRAGMENT,
				ty: wgpu::BindingType::Texture {
					sample_type: wgpu::TextureSampleType::Float { filterable: true },
					view_dimension: wgpu::TextureViewDimension::D2Array,
					multisampled: false,
				},
				count: None,
			},
			wgpu::BindGroupLayoutEntry {
				binding: 1,
				visibility: wgpu::ShaderStages::FRAGMENT,
				ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
				count: None,
			},
		],
	});

	let chunk_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
		label: Some("chunk_bind_group_layout"),
		entries: &[wgpu::BindGroupLayoutEntry {
			binding: 0,
			visibility: wgpu::ShaderStages::VERTEX,
			ty: wgpu::BindingType::Buffer {
				ty: wgpu::BufferBindingType::Uniform,
				has_dynamic_offset: false,
				min_binding_size: None, // used to be "wgpu::BufferSize::new(16)" -> // stupid gpu has to make it 16 at least ... 8 byte would be enough tho ..
			},
			count: None,
		},],
	});
	let skybox_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
		label: Some("skybox_bind_group_layout"),
		entries: &[
			wgpu::BindGroupLayoutEntry {
				binding: 0,
				visibility: wgpu::ShaderStages::FRAGMENT,
				ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
				count: None,
			},
			wgpu::BindGroupLayoutEntry {
				binding: 1,
				visibility: wgpu::ShaderStages::FRAGMENT,
				ty: wgpu::BindingType::Texture {
					sample_type: wgpu::TextureSampleType::Float { filterable: true },
					view_dimension: wgpu::TextureViewDimension::D2,
					multisampled: false,
				},
				count: None,
			},
		],
	});
	let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
		label: Some("camera_bind_group_layout"),
		entries: &[wgpu::BindGroupLayoutEntry {
			binding: 0,
			visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
			ty: wgpu::BindingType::Buffer {
				ty: wgpu::BufferBindingType::Uniform,
				has_dynamic_offset: false,
				min_binding_size: None,
			},
			count: None,
		}],
	});
	//wgpu::BindGroupLayout
	let layouts = [
		texture_bind_group_layout,
		camera_bind_group_layout,
		chunk_bind_group_layout,
		skybox_bind_group_layout,
		post_bind_group_layout,
	];

	Box::new(layouts)
}
