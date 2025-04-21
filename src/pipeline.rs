/// Struct holding both render pipelines and their associated shaders
#[allow(dead_code, unused)]
pub struct Pipeline {
    pub inside_pipeline: wgpu::RenderPipeline,
    pub chunk_pipeline: wgpu::RenderPipeline,
    inside_shader: wgpu::ShaderModule,
    chunk_shader: wgpu::ShaderModule,
}

impl Pipeline {
    /// Creates both render pipelines with proper configuration
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        layout: &wgpu::PipelineLayout,
    ) -> Self {
        let inside_shader = create_inside_shader(device);
        let chunk_shader = create_chunk_shader(device);
        let inside_pipeline = create_inside_pipeline(device, layout, &inside_shader, config.format);
        let chunk_pipeline = create_chunk_pipeline(device, layout, &chunk_shader, config.format);
        Self {
            inside_pipeline,
            chunk_pipeline,
            inside_shader,
            chunk_shader,
        }
    }
}

/// Creates the main shader module for texturing
fn create_chunk_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Chunk Shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::from(include_str!("chunk_shader.wgsl"))),
    })
}

/// Creates the inside shader module for solid color rendering
fn create_inside_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Inside Solid Color Shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::from(include_str!(
            "inside_shader.wgsl"
        ))),
    })
}

fn create_chunk_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Chunk Render Pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[super::geometry::Vertex::desc()], // Only vertex buffer, no instance buffer
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: default_primitive_state(),
        depth_stencil: Some(depth_stencil_state(true)),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

/// Creates the inside render pipeline for solid color rendering
fn create_inside_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    inside_shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Inside Render Pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: inside_shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[
                // Reference the const + static instance layout
                super::geometry::Vertex::desc(),
            ],
        },
        fragment: Some(wgpu::FragmentState {
            module: inside_shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: inside_primitive_state(),
        depth_stencil: Some(inside_depth_stencil()),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

// --- Pipeline Configuration Functions ---

/// Creates default primitive state configuration
fn default_primitive_state() -> wgpu::PrimitiveState {
    wgpu::PrimitiveState {
        // Use strip topology if possible (reduces vertex processing)
        topology: wgpu::PrimitiveTopology::TriangleList, // Keep if indexed geometry needs it
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        // Conservative culling - verify your mesh winding
        cull_mode: Some(wgpu::Face::Back),
        polygon_mode: wgpu::PolygonMode::Fill,
        // Enable conservative rasterization if supported
        conservative: false,
        unclipped_depth: false,
        // Keep other defaults
        ..Default::default()
    }
}

/// Creates primitive state for inside pipeline (front face culling)
fn inside_primitive_state() -> wgpu::PrimitiveState {
    wgpu::PrimitiveState {
        front_face: wgpu::FrontFace::Cw,
        ..default_primitive_state()
    }
}

/// Creates depth state for main pipeline
fn depth_stencil_state(write_enabled: bool) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: super::geometry::Texture::DEPTH_FORMAT,
        // Disable depth write for opaque objects after first pass
        depth_write_enabled: write_enabled,
        // Use LessEqual for early depth test
        depth_compare: wgpu::CompareFunction::LessEqual,
        // Disable stencil if unused
        stencil: wgpu::StencilState::default(),
        // Disable depth bias
        bias: wgpu::DepthBiasState::default(),
    }
}

/// Creates depth state for inside pipeline (reverse depth)
fn inside_depth_stencil() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        depth_compare: wgpu::CompareFunction::Less, // Render behind existing geometry
        ..depth_stencil_state(false)
    }
}

// --- Render Functions ---

/// Background color constant
const BACKGROUND_COLOR: wgpu::Color = wgpu::Color {
    r: 0.1,
    g: 0.2,
    b: 0.3,
    a: 1.0,
};

/// Begins the 3D render pass with depth buffer
fn begin_3d_render_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    color_view: &'a wgpu::TextureView,
    depth_view: &'a wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("3D Render Pass"),
        // Reduce color attachment clears if possible
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: color_view,
            resolve_target: None,
            ops: wgpu::Operations {
                // Consider using LoadOp::Clear only when necessary
                load: wgpu::LoadOp::Clear(BACKGROUND_COLOR),
                store: wgpu::StoreOp::Store,
            },
        })],
        // Use StoreOp::Discard if depth isn't reused
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth_view,
            depth_ops: Some(wgpu::Operations {
                // Use LoadOp::Load if you're doing multipass rendering
                load: wgpu::LoadOp::Clear(1.0),
                // Use StoreOp::Discard if depth buffer isn't needed next frame
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        // Enable occlusion culling if available
        occlusion_query_set: None,
        timestamp_writes: None,
    })
}

/// Begins the UI render pass without depth buffer
fn begin_ui_render_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    view: &'a wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("UI Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        ..Default::default()
    })
}

/// Main rendering function
pub fn render_all(current_state: &mut super::State) -> Result<(), wgpu::SurfaceError> {
    //let start = std::time::Instant::now();
    let output = current_state.surface().get_current_texture()?;
    let view = output.texture.create_view(&Default::default());

    let mut encoder = current_state
        .device()
        .create_command_encoder(&Default::default());

    // 3D Render Pass
    {
        let depth_view = &current_state.texture_manager().depth_texture.view;
        let mut rpass = begin_3d_render_pass(&mut encoder, &view, depth_view);

        // Shared bind groups for both pipelines
        let bind_groups = &[
            &current_state.texture_manager().bind_group,
            &current_state.camera_system.bind_group,
        ];

        // Draw inside surfaces
        {
            rpass.set_pipeline(&current_state.pipeline.inside_pipeline);
            // Draw chunks using chunk pipeline
            // Set bind groups
            bind_groups
                .iter()
                .enumerate()
                .for_each(|(i, g)| rpass.set_bind_group(i as u32, *g, &[]));
            current_state.data_system.world.render_chunks(&mut rpass);
        }

        {
            rpass.set_pipeline(&current_state.pipeline.chunk_pipeline);
            // Draw chunks using chunk pipeline
            // Set bind groups
            bind_groups
                .iter()
                .enumerate()
                .for_each(|(i, g)| rpass.set_bind_group(i as u32, *g, &[]));
            current_state.data_system.world.render_chunks(&mut rpass);
        }
    }

    // UI Render Pass
    if current_state.ui_manager.visibility {
        let mut ui_rpass = begin_ui_render_pass(&mut encoder, &view);
        draw_ui(
            &mut ui_rpass,
            &current_state.ui_manager.pipeline,
            &current_state.ui_manager.vertex_buffer,
            &current_state.ui_manager.index_buffer,
            &current_state.ui_manager.bind_group,
            current_state.ui_manager.num_indices,
        );
    }

    //println!("Framerate: {:?}", start.elapsed());
    // the rest in not included in the time counting because they are closer to static
    // so it is impossible to make that time go down actually (it is basically 15ms so not much)
    // i hope it is still fine ...

    let _submission = current_state
        .queue()
        .submit(std::iter::once(encoder.finish()));
    output.present();
    Ok(())
}

/// Draws UI elements
pub fn draw_ui(
    rpass: &mut wgpu::RenderPass,
    pipeline: &wgpu::RenderPipeline,
    vertex_buffer: &wgpu::Buffer,
    index_buffer: &wgpu::Buffer,
    bind_group: &wgpu::BindGroup,
    num_indices: u32,
) {
    rpass.set_pipeline(pipeline);
    rpass.set_bind_group(0, bind_group, &[]);
    rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
    rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
    rpass.draw_indexed(0..num_indices, 0, 0..1);
}
