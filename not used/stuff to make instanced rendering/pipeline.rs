/// Struct holding both render pipelines and their associated shaders
#[allow(dead_code, unused)]
pub struct Pipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    shader: wgpu::ShaderModule,
}

impl Pipeline {
    /// Creates both render pipelines with proper configuration
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        layout: &wgpu::PipelineLayout,
    ) -> Self {
        let shader = create_main_shader(device);
        let render_pipeline = create_main_pipeline(device, layout, &main_shader, config.format);
        Self {
            shader,
            render_pipeline,
        }
    }
}

/// Creates the main shader module for texturing
fn create_main_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Main Texture Shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::from(include_str!(
            "texture_shader.wgsl"
        ))),
    })
}
/// Creates the main render pipeline for textured geometry
fn create_main_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Main Render Pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[
                super::geometry::Vertex::desc(),
                super::geometry::InstanceRaw::desc(),
            ],
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


// --- Pipeline Configuration Functions ---

/// Creates default primitive state configuration
fn default_primitive_state() -> wgpu::PrimitiveState {
    wgpu::PrimitiveState {
        // Use strip topology if possible (reduces vertex processing)
        topology: wgpu::PrimitiveTopology::TriangleList, // Keep if indexed geometry needs it
        // Conservative culling - verify your mesh winding
        cull_mode: Some(wgpu::Face::Back),
        // Enable conservative rasterization if supported
        conservative: false,
        // Keep other defaults
        ..Default::default()
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
        // Use StoreOp::Discard if depth isn't reused
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store, // Change to Discard if not needed later
            }),
            stencil_ops: None,
        }),
        // Reduce color attachment clears if possible
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: color_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(BACKGROUND_COLOR),
                store: wgpu::StoreOp::Store,
            },
        })],
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
    let output = current_state.surface().get_current_texture()?;
    let view = output.texture.create_view(&Default::default());

    let mut encoder = current_state
        .device()
        .create_command_encoder(&Default::default());

    let start = std::time::Instant::now();
    // 3D Render Pass
    {
        let depth_view = &current_state.texture_manager().depth_texture.view;
        let mut rpass = begin_3d_render_pass(&mut encoder, &view, depth_view);

        // Shared bind groups for both pipelines
        let bind_groups = &[
            &current_state.texture_manager().bind_group,
            &current_state.camera_system.bind_group,
        ];
        // Set bind groups
        // Draw main geometry
        draw_with_pipeline(
            &mut rpass,
            &current_state.pipeline.render_pipeline,
            bind_groups,
            &[
                &current_state.geometry_buffer().vertex_buffer,
                &current_state.instance_manager().borrow().instance_buffer,
            ],
            &current_state.geometry_buffer().index_buffer,
            current_state.geometry_buffer().num_indices,
            current_state.instance_manager().borrow().instances.len() as u32,
        );

    }

    //println!("GPU draw took: {:?}", start.elapsed());
    // the rest in not included in the time counting because they are closer to static
    // so it is impossible to make that time go down actually (it is basically 15ms so not much)
    // i hope it is still fine ...
    current_state.queue().submit(Some(encoder.finish()));
    output.present();
    Ok(())
}

/// Draws geometry using a specified pipeline
pub fn draw_with_pipeline(
    rpass: &mut wgpu::RenderPass,
    pipeline: &wgpu::RenderPipeline,
    bind_groups: &[&wgpu::BindGroup],
    vertex_buffers: &[&wgpu::Buffer],
    index_buffer: &wgpu::Buffer,
    num_indices: u32,
    instances: u32,
) {
    rpass.set_pipeline(pipeline);
    // Set bind groups
    bind_groups
        .iter()
        .enumerate()
        .for_each(|(i, g)| rpass.set_bind_group(i as u32, *g, &[]));
    // Batch vertex buffer assignments
    for (i, buffer) in vertex_buffers.iter().enumerate() {
        rpass.set_vertex_buffer(i as u32, buffer.slice(..));
    }
    // Single draw call with instancing
    rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
    rpass.draw_indexed(0..num_indices, 0, 0..instances); // Keep instanced draw
}
