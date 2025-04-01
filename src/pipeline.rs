use super::geometry;
use std::borrow::Cow;
use wgpu;
#[allow(
    dead_code,
    unused,
    redundant_imports,
    unused_results,
    unused_features,
    unused_variables
)]
/// Struct holding both render pipelines and their associated shaders
pub struct Pipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub inside_pipeline: wgpu::RenderPipeline,
    shader: wgpu::ShaderModule,
    inside_shader: wgpu::ShaderModule,
}

impl Pipeline {
    /// Creates both render pipelines with proper configuration
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        layout: &wgpu::PipelineLayout,
    ) -> Self {
        let main_shader = create_main_shader(device);
        let inside_shader = create_inside_shader(device);

        // Removed unused 'config' parameter from create_main_pipeline
        let render_pipeline = create_main_pipeline(device, layout, &main_shader, config.format);

        let inside_pipeline = create_inside_pipeline(device, layout, &inside_shader, config.format);

        Pipeline {
            render_pipeline,
            inside_pipeline,
            shader: main_shader,
            inside_shader,
        }
    }
}

/// Creates the main shader module for texturing
fn create_main_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Main Texture Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::from(include_str!("texture_shader.wgsl"))),
    })
}

/// Creates the inside shader module for solid color rendering
fn create_inside_shader(device: &wgpu::Device) -> wgpu::ShaderModule {
    device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Inside Solid Color Shader"),
        source: wgpu::ShaderSource::Wgsl(Cow::from(
            r#"
                // Vertex Input Structure
                struct VertexInput {
                    @location(0) position: vec3<f32>,
                };

                // Camera Uniform Structure
                struct CameraUniform {
                    view_proj: mat4x4<f32>,
                };
                @group(1) @binding(0)
                var<uniform> camera: CameraUniform;

                // Instance Data Structure
                struct InstanceInput {
                    @location(5) model_matrix_0: vec4<f32>,
                    @location(6) model_matrix_1: vec4<f32>,
                    @location(7) model_matrix_2: vec4<f32>,
                    @location(8) model_matrix_3: vec4<f32>,
                };

                @vertex
                fn vs_main(
                    vertex: VertexInput,
                    instance: InstanceInput,
                ) -> @builtin(position) vec4<f32> {
                    // Reconstruct model matrix from instance data
                    let model_matrix = mat4x4<f32>(
                        instance.model_matrix_0,
                        instance.model_matrix_1,
                        instance.model_matrix_2 * -1.0, // Invert Y for coordinate system
                        instance.model_matrix_3,
                    );
                    let world_pos = model_matrix * vec4<f32>(vertex.position, 1.0);
                    return camera.view_proj * world_pos;
                }

                @fragment
                fn fs_main() -> @location(0) vec4<f32> {
                    // Solid gray color with transparency
                    return vec4<f32>(0.8, 0.8, 0.8, 0.8);
                }
            "#,
        )),
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
                geometry::Vertex::desc(),
                geometry::TexCoord::desc(),
                geometry::InstanceRaw::desc(),
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
            buffers: &*&[
                // Reference the const + static instance layout
                wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &VERTEX_ATTRIBUTES_POS,
                },
                geometry::InstanceRaw::desc(),
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
        topology: wgpu::PrimitiveTopology::TriangleList,
        front_face: wgpu::FrontFace::Ccw,
        cull_mode: Some(wgpu::Face::Back),
        ..Default::default()
    }
}

// --- Pipeline Configuration Functions ---

/// Vertex attributes for position-only buffers (now static)
const VERTEX_ATTRIBUTES_POS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];

/// Creates primitive state for inside pipeline (front face culling)
fn inside_primitive_state() -> wgpu::PrimitiveState {
    wgpu::PrimitiveState {
        cull_mode: Some(wgpu::Face::Front),
        ..default_primitive_state()
    }
}

/// Creates depth state for main pipeline
fn depth_stencil_state(write_enabled: bool) -> wgpu::DepthStencilState {
    wgpu::DepthStencilState {
        format: geometry::Texture::DEPTH_FORMAT,
        depth_write_enabled: write_enabled,
        depth_compare: wgpu::CompareFunction::LessEqual,
        stencil: Default::default(),
        bias: Default::default(),
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
    let color_attachment = wgpu::RenderPassColorAttachment {
        view: color_view,
        resolve_target: None,
        ops: wgpu::Operations {
            load: wgpu::LoadOp::Clear(BACKGROUND_COLOR),
            store: wgpu::StoreOp::Store,
        },
    };

    let depth_attachment = wgpu::RenderPassDepthStencilAttachment {
        view: depth_view,
        depth_ops: Some(wgpu::Operations {
            load: wgpu::LoadOp::Clear(1.0),
            store: wgpu::StoreOp::Store,
        }),
        stencil_ops: None,
    };

    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("3D Render Pass"),
        color_attachments: &[Some(color_attachment)],
        depth_stencil_attachment: Some(depth_attachment),
        ..Default::default()
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
    let output = current_state.surface.get_current_texture()?;
    let view = output.texture.create_view(&Default::default());

    let mut encoder = current_state
        .device
        .create_command_encoder(&Default::default());

    // 3D Render Pass
    {
        let depth_view = &current_state.texture_manager.depth_texture.view;
        let mut rpass = begin_3d_render_pass(&mut encoder, &view, depth_view);

        // Draw main geometry
        draw_with_pipeline(
            &mut rpass,
            &current_state.pipeline.render_pipeline,
            &[
                &current_state.texture_manager.bind_group,
                &current_state.camera_system.bind_group,
            ],
            &[
                &current_state.geometry_buffer.vertex_buffer,
                &current_state.geometry_buffer.texture_coord_buffer,
                &current_state.instance_manager.instance_buffer,
            ],
            &current_state.geometry_buffer.index_buffer,
            current_state.geometry_buffer.num_indices,
            current_state.instance_manager.instances.len() as u32,
        );

        // Draw inside surfaces
        draw_with_pipeline(
            &mut rpass,
            &current_state.pipeline.inside_pipeline,
            &[
                &current_state.texture_manager.bind_group,
                &current_state.camera_system.bind_group, // Only camera needed here
            ],
            &[
                &current_state.geometry_buffer.vertex_buffer,
                &current_state.instance_manager.instance_buffer,
            ],
            &current_state.geometry_buffer.index_buffer,
            current_state.geometry_buffer.num_indices,
            current_state.instance_manager.instances.len() as u32,
        );
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

    current_state.queue.submit(Some(encoder.finish()));
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
    for (i, group) in bind_groups.iter().enumerate() {
        rpass.set_bind_group(i as u32, *group, &[]);
    }

    // Set vertex buffers
    for (i, buffer) in vertex_buffers.iter().enumerate() {
        rpass.set_vertex_buffer(i as u32, buffer.slice(..));
    }

    // Draw command
    rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
    rpass.draw_indexed(0..num_indices, 0, 0..instances);
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
