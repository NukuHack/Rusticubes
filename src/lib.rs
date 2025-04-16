

mod camera;
mod config;
mod geometry;
mod pipeline;
mod user_interface;

use std::iter::Iterator;
use winit::{
    event::{ElementState, Event, MouseButton, WindowEvent, KeyEvent},
    keyboard::KeyCode as Key,
};


pub struct State<'a> {
    window: &'a winit::window::Window,
    render_context: RenderContext<'a>,
    previous_frame_time: std::time::Instant,
    camera_system: camera::CameraSystem,
    input_system: InputSubsystem,
    pipeline: pipeline::Pipeline,
    data_system: DataSubsystem,
    ui_manager: user_interface::UIManager,
}
pub struct RenderContext<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
}

pub struct InputSubsystem {
    previous_mouse: Option<winit::dpi::PhysicalPosition<f64>>,
    mouse_button_state: MouseButtonState,
}
impl InputSubsystem{
    pub fn default() -> Self{
        Self{
            previous_mouse: None,
            mouse_button_state: MouseButtonState::default(),
        }
    }
}

#[derive(Default)]
pub struct MouseButtonState {
    pub left: bool,
    pub right: bool,
}

pub struct DataSubsystem {
    geometry_buffer: geometry::GeometryBuffer,
    texture_manager: geometry::TextureManager,
    instance_manager: std::cell::RefCell<geometry::InstanceManager>, // Now wrapped in RefCell
}

impl<'a> State<'a> {
    async fn new(window: &'a winit::window::Window) -> Self {
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

        let (device, queue): (wgpu::Device, wgpu::Queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: if cfg!(target_arch = "wasm32") {
                        wgpu::Limits::downlevel_webgl2_defaults()
                    } else {
                        wgpu::Limits::default()
                    },
                    ..Default::default()
                },
                None,
            )
            .await
            .unwrap();

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

        // Create camera system with advanced controls
        let camera_system: camera::CameraSystem = camera::CameraSystem::new(
            &device,
            &size,
            cgmath::Point3::new(0.5, 1.8, 2.0), // by default the camera is 1.8 meters tall
        );

        surface.configure(&device, &config);

        // Constants for geometry creation
        const NUM_INSTANCES: u32 = 3;
        const SPACE_BETWEEN: f32 = 1.0;
        let instance_manager = std::cell::RefCell::new(geometry::InstanceManager::new(
            &device,
            &queue,
            NUM_INSTANCES,
            SPACE_BETWEEN,
            false,
        ));

        let texture_manager: geometry::TextureManager = geometry::TextureManager::new(&device, &queue, &config);
        let cube: geometry::Cube = geometry::Cube::default();
        let geometry_buffer: geometry::GeometryBuffer = geometry::CubeBuffer::new(
            &device,
            &cube,
        );

        let render_pipeline_layout: wgpu::PipelineLayout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &texture_manager.bind_group_layout,
                &camera_system.bind_group_layout,
            ],
            ..Default::default()
        });

        let pipeline: pipeline::Pipeline = pipeline::Pipeline::new(&device, &config, &render_pipeline_layout);

        let ui_manager:user_interface::UIManager = user_interface::UIManager::new(&device, &config, &queue);
        
        let data_system: DataSubsystem = DataSubsystem{
            geometry_buffer,
            texture_manager,
            instance_manager,
        };
        let render_context: RenderContext = RenderContext{
            surface,
            device,
            queue,
            config,
            size,
        };

        Self {
            window,
            render_context,
            previous_frame_time: std::time::Instant::now(),
            camera_system,
            input_system: InputSubsystem::default(),
            pipeline,
            data_system,
            ui_manager,
        }
    }

    pub fn window(&self) -> &winit::window::Window {
        self.window
    }
    pub fn surface(&self) -> &wgpu::Surface<'a> {
        &self.render_context.surface
    }
    pub fn device(&self) -> &wgpu::Device {
        &self.render_context.device
    }
    pub fn queue(&self) -> &wgpu::Queue {
        &self.render_context.queue
    }
    pub fn config(&self) -> &wgpu::SurfaceConfiguration {
        &self.render_context.config
    }
    pub fn size(&self) -> &winit::dpi::PhysicalSize<u32> {
        &self.render_context.size
    }
    pub fn previous_frame_time(&self) -> &std::time::Instant {
        &self.previous_frame_time
    }
    pub fn camera_system(&self) -> &camera::CameraSystem {
        &self.camera_system
    }
    pub fn pipeline(&self) -> &pipeline::Pipeline {
        &self.pipeline
    }
    pub fn geometry_buffer(&self) -> &geometry::GeometryBuffer {
        &self.data_system.geometry_buffer
    }
    pub fn texture_manager(&self) -> &geometry::TextureManager {
        &self.data_system.texture_manager
    }
    pub fn instance_manager(&self) -> &std::cell::RefCell<geometry::InstanceManager> {
        &self.data_system.instance_manager
    }
    pub fn ui_manager(&self) -> &user_interface::UIManager {
        &self.ui_manager
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) -> bool{
        if new_size.width > 0 && new_size.height > 0 {
            self.render_context.size = new_size;
            self.render_context.config.width = new_size.width;
            self.render_context.config.height = new_size.height;
            self.camera_system.projection.resize(new_size.width, new_size.height);
            self.render_context.surface.configure(self.device(), self.config());
            self.data_system.texture_manager.depth_texture = geometry::Texture::create_depth_texture(
                self.device(),
                self.config(),
                "depth_texture",
            );
            return true;
        }
        false
    }
    pub fn handle_events(&mut self,event: &WindowEvent) -> bool{
        match event {
            WindowEvent::CloseRequested => close_app(),
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
                        log::error!("Surface error");
                        close_app()
                    }
                    Err(wgpu::SurfaceError::Timeout) => {
                        log::warn!("Surface timeout");
                        true
                    },
                }
            },
            WindowEvent::KeyboardInput { .. } => self.handle_key_input(event),
            _ => self.handle_mouse_input(event)
        }
    }
    pub fn handle_key_input(&mut self, event: &WindowEvent) -> bool {
        if let WindowEvent::KeyboardInput {
            event: KeyEvent {
                physical_key: winit::keyboard::PhysicalKey::Code(physical_key), // Extract the KeyCode
                state,..
            },..
        } = event
        {
            // `key_code` is of type `KeyCode` (e.g., KeyCode::W)
            // `state` is of type `ElementState` (Pressed or Released)
            self.camera_system.controller.process_keyboard(&physical_key, &state);
        }

        match event {
            WindowEvent::KeyboardInput {
                event: KeyEvent {
                    physical_key: winit::keyboard::PhysicalKey::Code(key),
                    state: ElementState::Pressed // ElementState::Released // had to disable released otherwise hiding does not work correctly
                    , .. },..
            } => {
                match key {
                    Key::AltLeft | Key::AltRight => {
                        self.center_mouse();
                        return true;
                    },
                    Key::Escape => {
                        close_app();
                        return true;
                    },
                    Key::F1 => {
                        self.ui_manager.visibility=!self.ui_manager.visibility;
                        return true;
                    },
                    Key::KeyR => {
                        if let Some(prev) = self.input_system.previous_mouse {
                            geometry::rem_raycasted_cube(prev);
                        };
                        return true;
                    },
                    Key::KeyF => {
                        geometry::add_def_cube();
                        return true;
                    },
                    Key::KeyG => {
                        geometry::rem_last_cube();
                        return true;
                    },
                    _ => return false,
                }
            },
            _ => return false
        }
        
    }

    pub fn center_mouse(&self) {
        // Reset mouse to center
        let size: &winit::dpi::PhysicalSize<u32> = self.size();
        let x:f64 = (size.width as f64) / 2.0;
        let y:f64 = (size.height as f64) / 2.0;
        self.window().set_cursor_position(winit::dpi::PhysicalPosition::new(x, y))
            .expect("Set mouse cursor position");
    }

    pub fn handle_mouse_input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::MouseInput { button, state, .. } => {
                match (button, *state) {
                    (MouseButton::Left, ElementState::Pressed) => {
                        self.input_system.mouse_button_state.left = true;
                        user_interface::handle_ui_click(self);
                        return true;
                    }
                    (MouseButton::Left, ElementState::Released) => {
                        self.input_system.mouse_button_state.left = false;
                        return true;
                    }
                    (MouseButton::Right, ElementState::Pressed) => {
                        self.input_system.mouse_button_state.right = true;
                        return true;
                    }
                    (MouseButton::Right, ElementState::Released) => {
                        self.input_system.mouse_button_state.right = false;
                        return true;
                    }
                    _ => (),
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.input_system.mouse_button_state.right == true {
                    if let Some(prev) = self.input_system.previous_mouse {
                        let delta_x: f32 = (position.x - prev.x) as f32;
                        let delta_y: f32 = (position.y - prev.y) as f32;
                        self.camera_system.controller.process_mouse(delta_x, delta_y);
                    }
                }

                user_interface::handle_ui_hover(self, position);
                self.input_system.previous_mouse = Some(*position);
                return true;
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_system.controller.process_scroll(delta);
                return true;
            }
            _ => (),
        };
        false
    }

    pub fn update(&mut self) {
        let current_time: std::time::Instant = std::time::Instant::now();
        let delta_seconds: f32 = (current_time - self.previous_frame_time).as_secs_f32();
        self.previous_frame_time = current_time;
        self.camera_system.update(&self.render_context.queue, delta_seconds);

        if self.ui_manager.visibility {
            self.ui_manager.update(&self.render_context.queue);
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        pipeline::render_all(self)
    }
}

static mut WINDOW_PTR: *mut winit::window::Window = std::ptr::null_mut();
pub static mut STATE_PTR: *mut State = std::ptr::null_mut();

//#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    env_logger::init();
    
    let event_loop: winit::event_loop::EventLoop<()> = winit::event_loop::EventLoop::new().unwrap();
    let monitor: winit::monitor::MonitorHandle = event_loop.primary_monitor().expect("No primary monitor found!");
    let monitor_size: winit::dpi::PhysicalSize<u32> = monitor.size(); // Monitor size in physical pixels
    
    let config: config::AppConfig = config::AppConfig::default(monitor_size);
    let window_raw: winit::window::Window = winit::window::WindowBuilder::new()
        .with_title(&config.window_title)
        .with_inner_size(config.initial_window_size)
        .with_position(config.initial_window_position)
        .build(&event_loop)
        .unwrap();

    // Set the window to be focused immediately
    window_raw.has_focus();
    unsafe {
        WINDOW_PTR = Box::into_raw(Box::new(window_raw));
    }
    let window: &mut winit::window::Window = unsafe { &mut *WINDOW_PTR };
        
    let mut state_raw: State = State::new(window).await;
    user_interface::setup_ui(&mut state_raw);

    #[allow(unused_unsafe)]
    unsafe {   // Store the pointer in the static variable
        STATE_PTR = Box::into_raw(Box::new(state_raw));
    }

    let state = unsafe{get_state()};

    event_loop.run(move |event, control_flow| {
        if closed() {
            unsafe {
                if !STATE_PTR.is_null() {
                    let _ = Box::from_raw(STATE_PTR); // Drops the State
                    STATE_PTR = std::ptr::null_mut();
                }
            }
            control_flow.exit();
            return;
        }
        match &event {
            Event::WindowEvent { event, window_id } if *window_id == state.window().id() => {
                state.handle_events(event);
            }
            _ => {}
        }
    }).expect("Event loop error");
}


// Add this new unsafe function to retrieve the State
pub unsafe fn get_state() -> &'static mut State<'static> { unsafe {
    if STATE_PTR.is_null() {
        panic!("State not initialized or already dropped");
    }
    &mut *STATE_PTR
}}


pub static mut CLOSED:bool = false;
pub fn close_app() -> bool{
    unsafe{CLOSED = true;};
    true
}
pub fn closed() -> bool{
    unsafe{CLOSED}
}