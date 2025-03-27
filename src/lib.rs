
mod texture;
mod camera;
mod config;
mod geometry;
mod pipeline;
mod instances;
mod cube;

use std::{
    iter::Iterator,
    time::Instant,
};
use wgpu::{
    Adapter,
    PresentMode,
};
use winit::{
    dpi::PhysicalSize,
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowBuilder},
};

use crate::pipeline::*;



struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    previous_frame_time: Instant,
    camera_system: camera::CameraSystem,
    camera_controller: camera::CameraController,
    config: wgpu::SurfaceConfiguration,
    size: PhysicalSize<u32>,
    window: &'a Window,
    pipeline: pipeline::Pipeline,
    geometry_buffer: geometry::GeometryBuffer,
    texture_manager: texture::TextureManager,
    instance_manager: instances::InstanceManager,
}


impl<'a> State<'a> {
    async fn new(window: &'a Window) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY,
            #[cfg(target_arch = "wasm32")]
            backends: wgpu::Backends::GL,
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        // Find the first adapter that supports the surface
        let adapter:Adapter = instance
            .enumerate_adapters(wgpu::Backends::all())
            .into_iter() // Convert Vec to iterator
            .filter(|adapter| adapter.is_surface_supported(&surface))
            .next()
            .expect("No suitable GPU adapter found");


        let (device, queue) = adapter
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

        let instance_manager = instances::InstanceManager::new(&device);

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        let present_mode = surface_caps.present_modes.iter().copied()
            .find(|mode| *mode == PresentMode::Fifo)
            .unwrap_or(surface_caps.present_modes[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let camera = camera::Camera::new(
            (0.0, 1.0, 2.0).into(),
            (0.0, 0.0, 0.0).into(),
            cgmath::Vector3::unit_y(),
            config.width as f32 / config.height as f32,
            45.0,
            0.1,
            100.0,
        );
        let camera_system = camera::CameraSystem::new(&device, &config, camera);
        let camera_controller = camera::CameraController::new(2.0, 1.0);

        surface.configure(&device, &config);
        let texture_manager = texture::TextureManager::new(&device, &queue, &config);
        let geometry_buffer = geometry::GeometryBuffer::new(&device, &geometry::INDICES, &geometry::VERTICES, &geometry::TEXTURE_COORDS);

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &texture_manager.bind_group_layout,
                &camera_system.bind_group_layout,
            ],
            ..Default::default()
        });

        let pipeline = pipeline::Pipeline::new(&device, &config, &render_pipeline_layout);

        Self {
            surface,
            device,
            queue,
            previous_frame_time: Instant::now(),
            camera_system,
            camera_controller,
            config,
            size,
            window,
            pipeline,
            geometry_buffer,
            texture_manager,
            instance_manager,
        }
    }

    pub fn window(&self) -> &Window {
        self.window
    }

    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.texture_manager.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.camera_controller.process_events(event)
    }

    pub fn update(&mut self) {
        let current_time:Instant = Instant::now();
        let delta_seconds:f32 = (current_time - self.previous_frame_time).as_secs_f32();
        self.previous_frame_time = current_time;

        self.camera_controller
            .update_camera(&mut self.camera_system.camera, delta_seconds);

        self.camera_system.update(&mut self.queue);
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        render_all(self)
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let config: config::AppConfig = config::AppConfig::default();

    let window = WindowBuilder::new()
        .with_title(&config.window_title)
        .with_inner_size(PhysicalSize::new(
            config.initial_window_size.0,
            config.initial_window_size.1,
        ))
        .build(&event_loop)
        .unwrap();

    let mut state = State::new(&window).await;

    event_loop.run(move |event, control_flow| {
        match &event {
            Event::WindowEvent { event, window_id } if *window_id == state.window().id() => {
                if !state.input(event) {
                    match event {
                        WindowEvent::CloseRequested |
                        WindowEvent::KeyboardInput {
                            event: KeyEvent {
                                physical_key: PhysicalKey::Code(KeyCode::Escape),
                                state: ElementState::Pressed,
                                ..
                            },
                            ..
                        } => control_flow.exit(),
                        WindowEvent::Resized(physical_size) => state.resize(*physical_size),
                        WindowEvent::RedrawRequested => {
                            state.window().request_redraw();
                            state.update();
                            match state.render() {
                                Ok(_) => (),
                                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => state.resize(state.size),
                                Err(wgpu::SurfaceError::OutOfMemory | wgpu::SurfaceError::Other) => {
                                    log::error!("Surface error");
                                    control_flow.exit();
                                },
                                Err(wgpu::SurfaceError::Timeout) => log::warn!("Surface timeout"),
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }).expect("Event loop error");
}


