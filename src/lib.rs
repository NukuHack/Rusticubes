
mod texture;
mod camera;
mod config;
mod geometry;
mod pipeline;
mod instances;

use std::{
    iter::Iterator,
    time::{Duration, Instant},
};
use wgpu::{
    Adapter,
    PipelineLayout,
    PresentMode,
    TextureFormat
};
use winit::{
    dpi::PhysicalSize,
    window::Window,
    event::*,
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::WindowBuilder,
};

use crate::texture::*;



struct State<'a> {
    #[allow(unused)]
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    previous_frame_time: Instant,

    camera_system: camera::CameraSystem,
    camera_controller: camera::CameraController,

    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: &'a Window,

    pipeline: pipeline::Pipeline,

    geometry_buffer: geometry::GeometryBuffer,

    texture_manager: texture::TextureManager,

    instance_manager: instances::InstanceManager,
}


const BACKGROUND_COLOR: wgpu::Color = wgpu::Color {
    r: 0.1,
    g: 0.2,
    b: 0.3,
    a: 1.0,
};

impl<'a> State<'a> {
    // Creating some of the wgpu types requires async code
    async fn new(window: &'a Window) -> State<'a> {
        let size:PhysicalSize<u32> = window.inner_size();

        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            #[cfg(not(target_arch = "wasm32"))]
            backends: wgpu::Backends::PRIMARY, // Use the primary backends (Vulkan/DX12/Metal)
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
                    label: None,
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .unwrap();

        let instance_manager: instances::InstanceManager = instances::InstanceManager::new(
            &device,
        );

        // Shader code in this tutorial assumes an sRGB surface texture. Using a different
        // one will result in all the colors coming out darker. If you want to support non
        // sRGB surfaces, you'll need to account for that when drawing to the frame.
        let surface_caps:wgpu::SurfaceCapabilities = surface.get_capabilities(&adapter);
        let surface_format:TextureFormat = surface_caps.formats.iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let present_mode:PresentMode = surface_caps.present_modes.iter().copied()
            .find(|mode| *mode == wgpu::PresentMode::Fifo)
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

        let camera:camera::Camera = camera::Camera::new(
            (0.0, 1.0, 2.0).into(),
            (0.0, 0.0, 0.0).into(),
            cgmath::Vector3::unit_y(),
            config.width as f32 / config.height as f32,
            45.0,
            0.1,
            100.0,
            //yaw: 90,
        );
        let camera_system: camera::CameraSystem = camera::CameraSystem::new(
            &device,
            &config,
            camera,
        );
        let camera_controller: camera::CameraController = camera::CameraController::new(
            2f32,
            1f32
        );

        // After creating `config`, configure the surface
        surface.configure(&device, &config);

        let texture_manager:TextureManager = texture::TextureManager::new(
            &device,
            &queue,
            &config,
            "happy-tree.png"
        );

        let geometry_buffer:geometry::GeometryBuffer = geometry::GeometryBuffer::new(
            &device,
            &geometry::INDICES,
            &geometry::VERTICES,
        );

        //let shader = device.create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        let render_pipeline_layout:PipelineLayout = device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[
                    &texture_manager.bind_group_layout,
                    &camera_system.bind_group_layout,
                ],
                push_constant_ranges: &[],
            }
        );

        let pipeline:pipeline::Pipeline = pipeline::Pipeline::new(
            &device,
            &config,
            &render_pipeline_layout,
        );
        
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
        &self.window
    }

    // impl State
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.texture_manager.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
        }
    }


    fn input(&mut self, event: &WindowEvent) -> bool {
        // passing the event so the movement gets processed
        self.camera_controller.process_events(event)
    }

    // In your main loop or update function:
    fn update(&mut self, _event: &WindowEvent) {
        // Calculate delta time
        let current_time:Instant = Instant::now();
        let delta_time:Duration = current_time - self.previous_frame_time;
        let delta_seconds:f32 = delta_time.as_secs_f32(); // Convert to seconds as f32
        self.previous_frame_time = current_time;

        // Update the camera with delta_time
        self.camera_controller.update_camera(
            &mut self.camera_system.camera,
            delta_seconds,
        );

        // Update the camera system buffer
        self.camera_system.update(&mut self.queue);
    }



    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        // Get the current surface texture
        let output = match self.surface.get_current_texture() {
            Ok(output) => output,
            Err(wgpu::SurfaceError::Lost) => {
                // Reconfigure the surface if it's lost
                self.resize(self.size);
                return Err(wgpu::SurfaceError::Lost);
            }
            Err(e) => return Err(e), // Propagate other errors
        };

        // Create a texture view from the surface texture
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

        // Create a command encoder
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            // Begin a render pass
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[
                    Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(BACKGROUND_COLOR),
                            store: wgpu::StoreOp::Store,
                        },
                    }),
                ],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.texture_manager.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            // Set the render pipeline
            render_pass.set_pipeline(&self.pipeline.render_pipeline);
            render_pass.set_bind_group(0, &self.texture_manager.bind_group, &[]);
            render_pass.set_bind_group(1, &self.camera_system.bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.geometry_buffer.vertex_buffer.slice(..));
            // NEW!
            render_pass.set_vertex_buffer(1, self.instance_manager.instance_buffer.slice(..));
            render_pass.set_index_buffer(self.geometry_buffer.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            // UPDATED!
            render_pass.draw_indexed(0..self.geometry_buffer.num_indices, 0, 0..self.instance_manager.instances.len() as _);
        }

        // Submit the command buffer to the queue
        self.queue.submit(std::iter::once(encoder.finish()));

        // Present the frame
        output.present();

        Ok(())
    }

}




#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub async fn run() {
    // Window setup...
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let config: config::AppConfig = config::AppConfig::default();
    let window:Window = WindowBuilder::new()
        .with_title(&config.window_title)
        .with_inner_size(winit::dpi::PhysicalSize::new(
            config.initial_window_size.0,
            config.initial_window_size.1,
        ))
        .build(&event_loop)
        .unwrap();
    let mut state:State = State::new(&window).await;


    event_loop.run(move |event, control_flow| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => if !state.input(event) { // UPDATED!
                match event {
                    WindowEvent::CloseRequested
                    | WindowEvent::KeyboardInput {
                        event:
                        KeyEvent {
                            state: ElementState::Pressed,
                            physical_key: PhysicalKey::Code(KeyCode::Escape),
                            ..
                        },
                        ..
                    } => control_flow.exit(),
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::RedrawRequested => {
                        // This tells winit that we want another frame after this one
                        state.window().request_redraw();

                        state.update(event);

                        match state.render() {
                            Ok(_) => {}
                            // Reconfigure the surface if it's lost or outdated
                            Err(
                                wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated,
                            ) => state.resize(state.size),
                            // The system is out of memory, we should probably quit
                            Err(
                                wgpu::SurfaceError::OutOfMemory | wgpu::SurfaceError::Other
                            ) => {
                                log::error!("OutOfMemory");
                                control_flow.exit();
                            }
                            // This happens when the frame takes too long to present
                            Err(wgpu::SurfaceError::Timeout) => {
                                log::warn!("Surface timeout")
                            }
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }).expect("Event call function:");
}

