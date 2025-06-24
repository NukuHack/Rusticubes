
mod camera;
mod player;

mod config;
mod geometry;
mod pipeline;

mod debug;
mod input;
mod math;
mod game_state;

mod resources;
mod audio;

mod cube;
mod cube_math;
mod cube_extra;
mod cube_render;
mod cube_tables;

mod world_manager;
mod world_builder;

mod ui_element;
mod ui_render;
mod ui_manager;
mod ui_setup;


use winit::keyboard::PhysicalKey;
use std::sync::atomic::Ordering;
use glam::Vec3;
use std::iter::Iterator;
use winit::{
    event::{ElementState, Event, MouseButton, WindowEvent, KeyEvent},
    keyboard::KeyCode as Key,
    window::Window
};

pub struct State<'a> {
    window: &'a Window,
    render_context: RenderContext<'a>,
    previous_frame_time: std::time::Instant,
    input_system: input::InputSystem,
    pipeline: pipeline::Pipeline,
    ui_manager: ui_manager::UIManager,
    camera_system: camera::CameraSystem,
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
            .await
            .unwrap();

        //println!("max_texture_array_layers {}",device.limits().max_texture_array_layers);
        // this is 256 so i can load up to 256 textures into one binding of the shader
        //most shader support up to 16 bindings but using too much stuff can make it slow or really V-ram consuming

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

        let cam_config = camera::CameraConfig::new(Vec3::new(0.5, 1.8, 2.0));
        // Create camera system with advanced controls
        let camera_system: camera::CameraSystem = camera::CameraSystem::new(
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
                &texture_manager.bind_group_layout,
                &camera_system.bind_group_layout,
                &chunk_bind_group_layout,

            ],
            ..Default::default()
        });

        let pipeline: pipeline::Pipeline = pipeline::Pipeline::new(&device, &config, &render_pipeline_layout);

        let mut ui_manager:ui_manager::UIManager = ui_manager::UIManager::new(&device, &config, &queue);
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
    pub fn modifiers(&self) -> &winit::keyboard::ModifiersState {
        &self.input_system.modifiers
    }
    #[inline]
    pub fn mouse_states(&self) -> &input::MouseButtonState {
        &self.input_system.mouse_button_state
    }
    #[inline]
    pub fn previous_frame_time(&self) -> &std::time::Instant {
        &self.previous_frame_time
    }
    #[inline]
    pub fn camera_system(&self) -> &camera::CameraSystem {
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
    pub fn ui_manager(&self) -> &ui_manager::UIManager {
        &self.ui_manager
    }
    #[inline]
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) -> bool{
        if new_size.width > 0 && new_size.height > 0 {
            self.render_context.size = new_size;
            self.render_context.config.width = new_size.width;
            self.render_context.config.height = new_size.height;
            self.camera_system.projection.resize(new_size);
            self.render_context.surface.configure(self.device(), self.config());
            self.texture_manager.depth_texture = geometry::Texture::create_depth_texture(
                self.device(),
                self.config(),
                "depth_texture",
            );
            return true
        }
        false
    }
    #[inline]
    pub fn handle_events(&mut self,event: &WindowEvent) -> bool{
        match event {
            WindowEvent::CloseRequested => {config::close_app(); true},
            WindowEvent::Resized(physical_size) => self.resize(*physical_size),
            WindowEvent::RedrawRequested => {
                self.window().request_redraw();
                self.update();
                match self.render() {
                    Ok(_) => true,
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        self.resize(*self.size())
                    },
                    Err(wgpu::SurfaceError::OutOfMemory | wgpu::SurfaceError::Other) => {
                        println!("Surface error");
                        config::close_app(); true
                    }
                    Err(wgpu::SurfaceError::Timeout) => {
                        println!("Surface timeout");
                        true
                    },
                }
            },
            WindowEvent::ModifiersChanged(modifiers) => {
                self.input_system.modifiers = modifiers.state();
                true
            },
            WindowEvent::KeyboardInput { .. } => {
                self.handle_key_input(event);
                true
            },
            _ => {
                self.handle_mouse_input(event);
                true
            }
        }
    }
    #[inline]
    pub fn handle_key_input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    physical_key,
                    state // ElementState::Released or ElementState::Pressed
                    , .. },..
            } => {
                let key = match physical_key {
                    PhysicalKey::Code(code) => *code,
                    _ => {
                        println!("You called a function that can only be called with a keyboard input ... without a keyboard input ... FF"); 
                        return false;
                    },
                };
                // Handle UI input first if there's a focused element
                if let Some(_focused_idx) = self.ui_manager.focused_element {
                    if self.is_world_running {
                        config::get_gamestate().player.controller.reset_keyboard(); // Temporary workaround
                    }
                    
                    if *state == ElementState::Pressed {
                        // Handle special keys for UI
                        self.ui_manager.handle_key_input(key,self.modifiers().shift_key());
                        return true;
                    }
                    return true;
                }

                // Toggle mouse capture when ALT is pressed
                if key == Key::AltLeft || key == Key::AltRight {
                    if *state == ElementState::Pressed {
                        self.toggle_mouse_capture();
                    }
                    return true;
                }

                // Handle game controls if no UI element is focused
                // `key` is of type `KeyCode` (e.g., KeyCode::W)
                // `state` is of type `ElementState` (Pressed or Released)
                if self.is_world_running {
                    config::get_gamestate().player.controller.process_keyboard(&key, &state);
                    match key {
                        Key::KeyF => {
                            if *state == ElementState::Pressed {
                                cube_extra::place_looked_cube();
                                return true
                            }
                            return false;
                        },
                        Key::KeyR => {
                            if *state == ElementState::Pressed {
                                cube_extra::remove_targeted_block();
                                return true
                            }
                            return false;
                        },
                        Key::KeyE => {
                            if *state == ElementState::Pressed {
                                cube_extra::toggle_looked_point();
                                return true
                            }
                            return false;
                        },
                        Key::KeyL => {
                            if *state == ElementState::Pressed {
                                cube_extra::add_full_chunk();
                                return true
                            }
                            return false;
                        },
                        _ => false,
                    };
                }
                match key {
                    Key::AltLeft | Key::AltRight => {
                        self.center_mouse();
                        true
                    },
                    Key::Escape => {
                        if *state == ElementState::Pressed {
                            ui_manager::close_pressed();
                            return true;
                        }
                        false
                    },
                    Key::F1 => {
                        if *state == ElementState::Pressed {
                            self.ui_manager.toggle_visibility();
                            return true
                        }
                        false
                    },
                    Key::F11 => {
                        if *state == ElementState::Pressed {
                            let window = self.window();
                            
                            if window.fullscreen().is_some() {
                                // If already fullscreen, exit fullscreen
                                window.set_fullscreen(None);
                            } else {
                                // Otherwise enter fullscreen
                                let current_monitor = window.current_monitor().unwrap_or_else(|| {
                                    window.available_monitors().next().expect("No monitors available")
                                });
                                
                                window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(Some(current_monitor))));
                            }
                            return true;
                        }
                        false
                    },
                    _ => false,
                }
            },
            _ => false
        }
    }
    #[inline]
    pub fn center_mouse(&self) {
        // Reset mouse to center
        let size: &winit::dpi::PhysicalSize<u32> = self.size();
        let x:f64 = (size.width as f64) / 2.0;
        let y:f64 = (size.height as f64) / 2.0;
        self.window().set_cursor_position(winit::dpi::PhysicalPosition::new(x, y))
            .expect("Set mouse cursor position");
    }
    #[inline]
    pub fn handle_mouse_input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::MouseInput { button, state, .. } => {
                match (button, *state) {
                    (MouseButton::Left, ElementState::Pressed) => {
                        self.input_system.mouse_button_state.left = true;
                        if self.ui_manager.visibility!=false{
                        // Use the stored current mouse position
                        if let Some(current_position) = self.input_system.previous_mouse {
                            ui_manager::handle_ui_click(&mut self.ui_manager, self.render_context.size.into(), &current_position);
                        }
                        }
                        true
                    }
                    (MouseButton::Left, ElementState::Released) => {
                        self.input_system.mouse_button_state.left = false;
                        true
                    }
                    (MouseButton::Right, ElementState::Pressed) => {
                        self.input_system.mouse_button_state.right = true;
                        true
                    }
                    (MouseButton::Right, ElementState::Released) => {
                        self.input_system.mouse_button_state.right = false;
                        true
                    }
                    _ => false,
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.input_system.mouse_captured == true {
                    // Calculate relative movement from center
                    let size = self.size();
                    let center_x = size.width as f64 / 2.0;
                    let center_y = size.height as f64 / 2.0;
                    
                    let delta_x = (position.x - center_x) as f32;
                    let delta_y = (position.y - center_y) as f32;
                    
                    // Process mouse movement for camera control

                    if self.is_world_running {
                        config::get_gamestate().player.controller.process_mouse(delta_x, delta_y);
                    }
                    // Reset cursor to center
                    self.center_mouse();
                    self.input_system.previous_mouse = Some(winit::dpi::PhysicalPosition::new(center_x, center_y));
                    return true;
                } else {
                    // Handle normal mouse movement for UI
                    if self.input_system.mouse_button_state.right {
                        if let Some(prev) = self.input_system.previous_mouse {
                            let delta_x = (position.x - prev.x) as f32;
                            let delta_y = (position.y - prev.y) as f32;
                            if self.is_world_running {
                                config::get_gamestate().player.controller.process_mouse(delta_x, delta_y);
                            }
                        }
                    }
                    
                    // Handle UI hover
                    ui_manager::handle_ui_hover(&mut self.ui_manager, self.render_context.size.into(), position);
                    self.input_system.previous_mouse = Some(*position);
                    return true;
                }

            }
            WindowEvent::MouseWheel { delta, .. } => {
                if self.is_world_running {
                    config::get_gamestate().player.controller.process_scroll(delta);
                }
                true
            }
            _ => false,
        };
        false
    }
    #[inline]
    pub fn update(&mut self) {
        let current_time: std::time::Instant = std::time::Instant::now();
        let delta_seconds: f32 = (current_time - self.previous_frame_time).as_secs_f32();
        self.previous_frame_time = current_time;
        
        if self.is_world_running {
            let movement_delta = config::get_gamestate().player.update(&mut self.camera_system.camera,&mut self.camera_system.projection,delta_seconds);
            config::get_gamestate().player.position += movement_delta;
            self.camera_system.camera.position+= movement_delta;
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
    #[inline]
    pub fn toggle_mouse_capture(&mut self) {
        if !self.input_system.mouse_captured && self.is_world_running {
            self.input_system.mouse_captured = true;
            // Hide cursor and lock to center
            self.window().set_cursor_visible(false);
            self.window().set_cursor_grab(winit::window::CursorGrabMode::Confined)
                .or_else(|_| self.window().set_cursor_grab(winit::window::CursorGrabMode::Locked))
                .unwrap();
            self.center_mouse();
        } else {
            self.input_system.mouse_captured = false;
            // Show cursor and release
            self.window().set_cursor_visible(true);
            self.window().set_cursor_grab(winit::window::CursorGrabMode::None).unwrap();
        }
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
    audio::play_background("background_music.ogg".to_string());
    // Control audio
    // audio::stop_all_sounds();
    // audio::clear_sound_queue();

    
    let config: config::AppConfig = config::AppConfig::new(monitor_size);
    let window_raw: Window = winit::window::WindowBuilder::new()
        .with_title(&config.window_title)
        .with_inner_size(config.initial_window_size)
        .with_min_inner_size(config.min_window_size)
        .with_position(config.initial_window_position)
        .with_window_icon(resources::load_icon_from_bytes())
        .with_theme(config.theme)
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
                    cube_extra::update_full_world();
                }
            }
            _ => {}
        }
    }).expect("Event loop error");
}


