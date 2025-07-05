
//mod camera;
mod player;

mod config;
mod geometry;
mod pipeline;

mod debug;
mod memory;
mod input;
mod math;
mod game_state;

mod audio;
mod time;
mod cursor;
mod event_handler;

pub mod ext{
    pub mod mods;
    pub mod mods_over;
    pub mod network;
    pub mod rs; // app-compiled resources
    pub mod fs; // file system - from the disk
}
pub mod world {
    pub mod main;
    pub mod manager;
    pub mod handler;
}
pub mod block {
    pub mod main;
    pub mod math;
    pub mod extra;
    pub mod render;
    pub mod lut;
}
pub mod ui {
    pub mod element;
    pub mod render;
    pub mod manager;
    pub mod setup;
}

/*
// Core game systems
pub mod game {
    pub mod player;
    pub mod state;
}
// Engine systems
pub mod systems {
    pub mod input;
    pub mod time;
    pub mod event_handler;
    pub mod file_manager;
    pub mod audio;
    pub mod resources;
    pub mod cursor;
    pub mod pipeline;
    pub mod modding;
    pub mod modding_override;
}
// Utility modules
pub mod utils {
    pub mod math;
    pub mod geometry;
    pub mod debug;
    pub mod memory;
    pub mod config;
}
*/


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
    input_system: input::InputSystem,
    pipeline: pipeline::Pipeline,
    ui_manager: ui::manager::UIManager,
    camera_system: player::CameraSystem,
    texture_manager: geometry::TextureManager,
    is_world_running: bool,
    save_path: std::path::PathBuf,
}

pub struct RenderContext<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    chunk_bind_group_layout:  wgpu::BindGroupLayout,
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

        let config: wgpu::SurfaceConfiguration = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let cam_config = player::CameraConfig::new(Vec3::new(0.5, 1.8, 2.0));
        // Create camera system with advanced controls
        let camera_system: player::CameraSystem = player::CameraSystem::new(
            &device,
            size,
            cam_config,
        );

        surface.configure(&device, &config);

        let texture_manager: geometry::TextureManager = geometry::TextureManager::new(&device, &queue, &config);

        let chunk_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(16), // stupid gpu has to make it 16 at least ... 8 byte would be enough tho ..
                    },
                    count: None,
                },
            ],
            label: Some("chunk_bind_group_layout"),
        });
        let render_pipeline_layout: wgpu::PipelineLayout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &texture_manager.bind_group_layout(),
                &camera_system.bind_group_layout(),
                &chunk_bind_group_layout,

            ],
            ..Default::default()
        });

        let pipeline: pipeline::Pipeline = pipeline::Pipeline::new(&device, &config, &render_pipeline_layout);

        let mut ui_manager:ui::manager::UIManager = ui::manager::UIManager::new(&device, &config, &queue);
        ui_manager.setup_ui();
        
        let render_context: RenderContext = RenderContext{
            surface,
            device,
            queue,
            config,
            size,
            chunk_bind_group_layout,
        };


        // has to make the error handling better , make the error quit from world
        let _ = config::ensure_save_dir();
        let save_path = config::get_save_path();

        Self {
            window,
            render_context,
            previous_frame_time: std::time::Instant::now(),
            camera_system,
            input_system: input::InputSystem::default(),
            pipeline,
            texture_manager,
            ui_manager,
            is_world_running: false,
            save_path,
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
    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        &self.render_context.config
    }
    pub fn size(&self) -> &winit::dpi::PhysicalSize<u32> {
        &self.render_context.size
    }
    #[inline]
    pub fn previous_frame_time(&self) -> &std::time::Instant {
        &self.previous_frame_time
    }
    #[inline]
    pub fn camera_system(&self) -> &player::CameraSystem {
        &self.camera_system
    }
    #[inline]
    pub fn pipeline(&self) -> &pipeline::Pipeline {
        &self.pipeline
    }
    #[inline]
    pub fn texture_manager(&self) -> &geometry::TextureManager {
        &self.texture_manager
    }
    #[inline]
    pub fn ui_manager(&self) -> &ui::manager::UIManager {
        &self.ui_manager
    }
    #[inline]
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) -> bool {
        if new_size.width > 0 && new_size.height > 0 {
            self.render_context.size = new_size;
            self.render_context.config.width = new_size.width;
            self.render_context.config.height = new_size.height;
            self.camera_system.projection_mut().resize(new_size);
            // Clone the values to avoid holding borrows
            self.render_context.surface.configure(self.device(), self.config());
            *self.texture_manager.depth_texture_mut() = geometry::Texture::create_depth_texture(self.device(), self.config(),"depth_texture");
            
            true
        } else {
            false
        }
    }
    #[inline]
    pub fn update(&mut self) {
        let current_time: std::time::Instant = std::time::Instant::now();
        let delta_seconds: f32 = (current_time - self.previous_frame_time).as_secs_f32();
        self.previous_frame_time = current_time;
        
        if self.is_world_running {
            let movement_delta = {
                let player = &mut config::get_gamestate().player_mut();
                player.update(&mut self.camera_system, delta_seconds)
            };

            // Update both player and camera positions in one operation
            {
                let player = &mut config::get_gamestate().player_mut();
                let camera = self.camera_system.camera_mut();
                player.append_position(movement_delta, camera);
            }

            self.camera_system.update(&self.render_context.queue);
        }
        if self.ui_manager.visibility {
            self.ui_manager.update_anim(delta_seconds);
            self.ui_manager.update(&self.render_context.device,&self.render_context.queue);
        }
    }
    #[inline]
    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        pipeline::render_all(self)
    }
}

//#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
#[inline]
pub async fn run() {    
    let event_loop: winit::event_loop::EventLoop<()> = winit::event_loop::EventLoop::new().unwrap();
    let monitor: winit::monitor::MonitorHandle = event_loop.primary_monitor().expect("No primary monitor found!");
    let monitor_size: winit::dpi::PhysicalSize<u32> = monitor.size(); // Monitor size in physical pixels


    // Initialize once at startup
    audio::init_audio().expect("Failed to initialize audio");
    audio::set_background("background_music.ogg");
    // Control audio
    // audio::stop_all_sounds();
    // audio::clear_sound_queue();

    
    let config: config::AppConfig = config::AppConfig::new(monitor_size);
    let window_raw: Window = winit::window::WindowBuilder::new()
        .with_title(&*config.window_title())
        .with_inner_size(*config.initial_window_size())
        .with_min_inner_size(*config.min_window_size())
        .with_position(*config.initial_window_position())
        .with_window_icon(ext::rs::load_icon_from_bytes())
        .with_theme(*config.theme())
        .with_active(true)
        .build(&event_loop)
        .unwrap();

    // Set the window to be focused immediately
    window_raw.focus_window();
    window_raw.set_visible(true);

    // Store the window pointer
    config::WINDOW_PTR.store(Box::into_raw(Box::new(window_raw)), Ordering::Release);
    let window_ref = config::get_window();
        
    let state = State::new(window_ref).await;

    // Store the state pointer
    config::STATE_PTR.store(Box::into_raw(Box::new(state)), Ordering::Release);

    match ext::mods::main() {
        Ok(_) => (), // Success case
        Err(e) => println!("⚠Error modding: {}", e),
    }
    match ext::mods_over::main() {
        Ok(_) => (), // Success case
        Err(e) => { println!("💥Error mod function override: {}", e); }
    }

    // Post-init cleanup
    memory::light_trim();
    memory::hard_clean(Some(config::get_state().device()));

    event_loop.run(move |event, control_flow| {
        if config::is_closed() {
            config::cleanup_resources();
            control_flow.exit();
            return;
        }
        match &event {
            Event::WindowEvent { event, window_id } if *window_id == config::get_state().window().id() => {
                config::get_state().handle_events(event);

                if config::get_state().is_world_running {
                    block::extra::update_full_world();
                }
            }
            _ => {}
        }
    }).expect("Event loop error");
}


