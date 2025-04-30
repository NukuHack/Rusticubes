use wgpu;

/// Struct holding all render pipelines and their associated shaders
#[allow(dead_code)]
pub struct Pipeline {
    // Public pipelines
    pub inside_pipeline: wgpu::RenderPipeline,
    pub chunk_pipeline: wgpu::RenderPipeline,
    pub post_pipeline: wgpu::RenderPipeline,
    pub sky_pipeline: wgpu::RenderPipeline,

    // Private shaders
    inside_shader: wgpu::ShaderModule,
    chunk_shader: wgpu::ShaderModule,
    post_shader: wgpu::ShaderModule,
    sky_shader: wgpu::ShaderModule,
}

impl Pipeline {
    /// Creates all render pipelines with proper configuration
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        layout: &wgpu::PipelineLayout,
    ) -> Self {
        // Create shaders
        let shaders = Shaders::new(device);

        // Create pipelines
        Self {
            inside_pipeline: create_inside_pipeline(device, layout, &shaders.inside, config.format),
            chunk_pipeline: create_chunk_pipeline(device, layout, &shaders.chunk, config.format),
            post_pipeline: create_post_pipeline(device, &shaders.post, config.format),
            sky_pipeline: create_sky_pipeline(device, &shaders.sky, config.format),

            // Store shaders
            inside_shader: shaders.inside,
            chunk_shader: shaders.chunk,
            post_shader: shaders.post,
            sky_shader: shaders.sky,
        }
    }
}

/// Helper struct for organizing shader creation
struct Shaders {
    inside: wgpu::ShaderModule,
    chunk: wgpu::ShaderModule,
    post: wgpu::ShaderModule,
    sky: wgpu::ShaderModule,
}

impl Shaders {
    fn new(device: &wgpu::Device) -> Self {
        Self {
            inside: create_shader(device, "Inside Solid Color Shader", INSIDE_SHADER),
            chunk: create_shader(device, "Chunk Shader", TEXTURE_SHADER),
            post: create_shader(device, "Post Processing Shader", include_str!("fxaa.wgsl")),
            sky: create_shader(device, "Sky Shader", include_str!("sky_shader.wgsl")),
        }
    }
}

/// Creates a shader module with the given label and source
fn create_shader(device: &wgpu::Device, label: &str, source: &str) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::from(source)),
    })
}

// Shader source combinations
const TEXTURE_SHADER: &str = concat!(
    include_str!("chunk_shader.wgsl"),
    "\n",
    include_str!("texture_shader.wgsl")
);

const INSIDE_SHADER: &str = concat!(
    include_str!("chunk_shader.wgsl"),
    "\n",
    include_str!("inside_shader.wgsl")
);

/// Common pipeline creation helper
fn create_base_pipeline(
    device: &wgpu::Device,
    layout: Option<&wgpu::PipelineLayout>,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    buffers: &[wgpu::VertexBufferLayout],
    depth_stencil: Option<wgpu::DepthStencilState>,
    primitive: wgpu::PrimitiveState,
    label: &str,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout,
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers,
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
        primitive,
        depth_stencil,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

fn create_chunk_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    create_base_pipeline(
        device,
        Some(layout),
        shader,
        format,
        &[super::geometry::Vertex::desc()],
        Some(depth_stencil_state(true)),
        default_primitive_state(),
        "Chunk Render Pipeline",
    )
}

fn create_inside_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    create_base_pipeline(
        device,
        Some(layout),
        shader,
        format,
        &[super::geometry::Vertex::desc()],
        Some(inside_depth_stencil()),
        inside_primitive_state(),
        "Inside Render Pipeline",
    )
}

fn create_post_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    // Create bind group layout
    let post_bind_group_layout = create_post_bind_group_layout(device);

    // Create pipeline layout
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Post Processing Pipeline Layout"),
        bind_group_layouts: &[&post_bind_group_layout],
        push_constant_ranges: &[],
    });

    create_base_pipeline(
        device,
        Some(&layout),
        shader,
        format,
        &[],
        None,
        default_primitive_state(),
        "Post Processing Pipeline",
    )
}

fn create_post_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
    })
}

fn create_sky_pipeline(
    device: &wgpu::Device,
    shader: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Sky Render Pipeline"),
        layout: None,
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: Some(wgpu::DepthStencilState {
            format: super::geometry::Texture::DEPTH_FORMAT,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
    })
}

// --- Pipeline Configuration ---

fn default_primitive_state() -> wgpu::PrimitiveState {
    wgpu::PrimitiveState {
        topology: wgpu::PrimitiveTopology::TriangleList,
        strip_index_format: None,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: Some(wgpu::Face::Back),
        polygon_mode: wgpu::PolygonMode::Fill,
        conservative: false,
        unclipped_depth: false,
        ..Default::default()
    }
}

fn inside_primitive_state() -> wgpu::PrimitiveState {
    wgpu::PrimitiveState {
        front_face: wgpu::FrontFace::Cw,
        ..default_primitive_state()
    }
}

fn depth_stencil_state(write_enabled: bool) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: super::geometry::Texture::DEPTH_FORMAT,
        depth_write_enabled: write_enabled,
        depth_compare: wgpu::CompareFunction::Less,
        stencil: wgpu::StencilState::default(),
        bias: wgpu::DepthBiasState::default(),
    }
}

fn inside_depth_stencil() -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        depth_compare: wgpu::CompareFunction::Less,
        ..depth_stencil_state(false)
    }
}

// --- Render Passes ---

fn begin_3d_render_pass<'a>(
    encoder: &'a mut wgpu::CommandEncoder,
    color_view: &'a wgpu::TextureView,
    depth_view: &'a wgpu::TextureView,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("3D Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: color_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        occlusion_query_set: None,
        timestamp_writes: None,
    })
}

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

// --- Main Rendering Function ---

pub fn render_all(current_state: &mut super::State) -> Result<(), wgpu::SurfaceError> {
    let output = current_state.surface().get_current_texture()?;
    let view = output.texture.create_view(&Default::default());
    let mut encoder = current_state
        .device()
        .create_command_encoder(&Default::default());

    render_sky_pass(&mut encoder, &view, current_state);
    render_3d_pass(&mut encoder, &view, current_state);
    render_post_pass(&mut encoder, &view, current_state);

    if current_state.ui_manager.visibility {
        render_ui_pass(&mut encoder, &view, current_state);
    }

    submit_and_present(current_state, encoder, output)
}

fn render_sky_pass(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    state: &super::State,
) {
    let depth_view = &state.texture_manager().depth_texture.view;
    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("Sky Render Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            view: depth_view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }),
        occlusion_query_set: None,
        timestamp_writes: None,
    });

    rpass.set_pipeline(&state.pipeline.sky_pipeline);
    rpass.draw(0..3, 0..1);
}

fn render_3d_pass(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    state: &super::State,
) {
    let depth_view = &state.texture_manager().depth_texture.view;
    let mut rpass = begin_3d_render_pass(encoder, view, depth_view);

    let bind_groups = [
        &state.texture_manager().bind_group,
        &state.camera_system.bind_group,
    ];

    // Render chunks
    rpass.set_pipeline(&state.pipeline.chunk_pipeline);
    set_bind_groups(&mut rpass, &bind_groups);
    state.data_system.world.render_chunks(&mut rpass);

    // Render inside surfaces
    rpass.set_pipeline(&state.pipeline.inside_pipeline);
    set_bind_groups(&mut rpass, &bind_groups);
    state.data_system.world.render_chunks(&mut rpass);
}

fn render_post_pass(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    state: &super::State,
) {
    let mut post_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("FXAA Pass"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
    });

    post_pass.set_pipeline(&state.pipeline.post_pipeline);
    post_pass.set_bind_group(0, &state.texture_manager().post_processing_bind_group, &[]);
    post_pass.draw(0..3, 0..1);
}

fn render_ui_pass(
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    state: &super::State,
) {
    let mut ui_rpass = begin_ui_render_pass(encoder, view);
    draw_ui(
        &mut ui_rpass,
        &state.ui_manager.pipeline,
        &state.ui_manager.vertex_buffer,
        &state.ui_manager.index_buffer,
        &state.ui_manager.bind_group,
        state.ui_manager.num_indices,
    );
}

fn submit_and_present(
    state: &super::State,
    encoder: wgpu::CommandEncoder,
    output: wgpu::SurfaceTexture,
) -> Result<(), wgpu::SurfaceError> {
    let _submission = state.queue().submit(std::iter::once(encoder.finish()));
    output.present();
    Ok(())
}

fn set_bind_groups(rpass: &mut wgpu::RenderPass, bind_groups: &[&wgpu::BindGroup]) {
    bind_groups.iter().enumerate().for_each(|(i, g)| {
        rpass.set_bind_group(i as u32, *g, &[]);
    });
}

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
