

mod camera;
mod config;
mod geometry;
mod pipeline;
mod user_interface;

use std::time::Instant;
use std::time::Duration;
use std::iter::Iterator;
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::EventLoop,
    keyboard::KeyCode as Key,
    window::{Window, WindowBuilder},
};
use crate::pipeline::*;

pub struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    previous_frame_time: Instant,
    camera_system: camera::CameraSystem,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    pub window: &'a Window,
    pipeline: pipeline::Pipeline,
    geometry_buffer: geometry::GeometryBuffer,
    texture_manager: geometry::TextureManager,
    instance_manager: geometry::InstanceManager,
    ui_manager: user_interface::UIManager,
}
pub static mut CLOSED:bool = false;

impl<'a> State<'a> {
    async fn new(window: &'a Window) -> Self {
        let size: PhysicalSize<u32> = window.inner_size();
        let instance: wgpu::Instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });
        let surface: wgpu::Surface<'a> = instance.create_surface(window).unwrap();

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

        let instance_manager: geometry::InstanceManager = geometry::InstanceManager::new(&device);

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
            &config,
            cgmath::Point3::new(0.5, 1.0, 2.0),
            cgmath::Rad(-std::f32::consts::FRAC_PI_2),
            cgmath::Rad(-0.3),
            70.0,
            0.1,
            100.0,
            4.0,
            0.4,
        );

        surface.configure(&device, &config);

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

        let mut ui_manager:user_interface::UIManager = user_interface::UIManager::new(&device, &config, &queue);
        let rect_element: user_interface::UIElement = user_interface::UIElement::new(
            (-0.5, -0.5),
            (0.2, 0.1),
            [0.3, 0.6, 0.7],
			String::new(),
            Some(Box::new(|| {
                println!("ff");
            })),
        );
        let text_element: user_interface::UIElement = user_interface::UIElement::new(
            (-0.5, 0.7),
            (0.5, 0.2),
            [1.0, 0.6, 0.7],
            "0123456789<=>?".to_string(),
            Some(Box::new(|| {
                println!("ff");
                //add_custom_instance();
            })),
        );
        ui_manager.add_ui_element(rect_element);
        ui_manager.add_ui_element(text_element);

        Self {
            surface,
            device,
            queue,
            previous_frame_time: Instant::now(),
            camera_system,
            config,
            size,
            window,
            pipeline,
            geometry_buffer,
            texture_manager,
            instance_manager,
            ui_manager,
        }
    }

    pub fn window(&self) -> &Window {
        self.window
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) -> bool{
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.camera_system.projection.resize(new_size.width, new_size.height);
            self.surface.configure(&self.device, &self.config);
            self.texture_manager.depth_texture = geometry::Texture::create_depth_texture(
                &self.device,
                &self.config,
                "depth_texture",
            );
            ()
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
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => self.resize(self.size),
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
                    state: ElementState::Pressed //| ElementState::Released // had to disable released otherwise hiding does not work correctly
                    , .. },..
            } => {
                match key {
                    Key::AltLeft | Key::AltRight => {
                        // Reset mouse to center, should make this not call the event for mousemove ...
                        let window: &winit::window::Window = self.window;
                        let physical_size: winit::dpi::PhysicalSize<u32> = window.inner_size();
                        let x:f64 = (physical_size.width as f64) / 2.0;
                        let y:f64 = (physical_size.height as f64) / 2.0;
                        window.set_cursor_position(winit::dpi::PhysicalPosition::new(x, y))
                            .expect("Set mouse cursor position");
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
                    _ => return false,
                }
            },
            _ => return false
        }
        
    }

    pub fn handle_mouse_input(&mut self, event: &WindowEvent) -> bool {
        match event {
            //TODO : make the ui do stuff
            WindowEvent::MouseInput { button, state, .. } => {
                match (button, *state) {
                    (MouseButton::Left, ElementState::Pressed) => {
                        self.camera_system.mouse_button_state.left = true;
                        user_interface::handle_ui_click(self);
                    }
                    (MouseButton::Left, ElementState::Released) => {
                        self.camera_system.mouse_button_state.left = false;
                    }
                    (MouseButton::Right, ElementState::Pressed) => {
                        self.camera_system.mouse_button_state.right = true;
                    }
                    (MouseButton::Right, ElementState::Released) => {
                        self.camera_system.mouse_button_state.right = false;
                    }
                    _ => (),
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.camera_system.mouse_button_state.right == true {
                    if let Some(prev) = self.camera_system.previous_mouse {
                        let delta_x: f64 = position.x - prev.x;
                        let delta_y: f64 = position.y - prev.y;
                        self.camera_system.controller.process_mouse(delta_x, delta_y);
                    }
                }

                user_interface::handle_ui_hover(self, position);

                self.camera_system.previous_mouse = Some(*position);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                self.camera_system.controller.process_scroll(delta);
            }
            _ => (),
        };
        false
    }
	/*
	pub fn add_custom_instance(){
		// Add a custom instance
		let custom_position = cgmath::Vector3 {
			x: 5.0,
			y: 1.0,
			z: 10.0,
		};
		let custom_rotation = cgmath::Quaternion::from_axis_angle(
			cgmath::Vector3::unit_x(),
			cgmath::Deg(90.0),
		);
		geometry::instance_manager.add_custom_instance(custom_position, custom_rotation, &device);
	}
*/
    pub fn update(&mut self) {
        let current_time: Instant = Instant::now();
        let delta_seconds: Duration = current_time - self.previous_frame_time;
        self.previous_frame_time = current_time;
        self.camera_system.update(&self.queue, delta_seconds);

        if self.ui_manager.visibility {
            self.ui_manager.update(&self.queue);
        }
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        render_all(self)
    }
}

//#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    env_logger::init();
	
    let event_loop: EventLoop<()> = EventLoop::new().unwrap();
    let monitor: winit::monitor::MonitorHandle = event_loop.primary_monitor().expect("No primary monitor found!");
    let monitor_size: winit::dpi::PhysicalSize<u32> = monitor.size(); // Monitor size in physical pixels
	
    let config: config::AppConfig = config::AppConfig::default(monitor_size);
    let window: Window = WindowBuilder::new()
        .with_title(&config.window_title)
        .with_inner_size(config.initial_window_size)
        .with_position(config.initial_window_position)
        .build(&event_loop)
        .unwrap();
    // Set the window to be focused immediately
    window.has_focus();
		
    let mut state: State = State::new(&window).await;
    event_loop.run(move |event, control_flow| {
        if 
        unsafe{
            CLOSED
        } {
            control_flow.exit();
        }

        match &event {
            Event::WindowEvent { event, window_id } if *window_id == state.window().id() => {
                state.handle_events(event);
            }
            _ => {}
        }
    }).expect("Event loop error");
}



pub fn close_app() -> bool{
    unsafe{
        CLOSED = true;
    };
    unsafe{
        CLOSED
    }
}