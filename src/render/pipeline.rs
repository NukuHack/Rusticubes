
use crate::ext::ptr;
use wgpu::util::DeviceExt;
use crate::State;
use crate::render::meshing::{Vertex, InstanceRaw, VERTICES};
use crate::render::texture;
use crate::get_string;
use wgpu;

/// Struct holding all render pipelines and their associated shaders
#[allow(dead_code)]
pub struct Pipeline {
	// Pipelines
	pub chunk_pipeline: wgpu::RenderPipeline,
	pub post_pipeline: wgpu::RenderPipeline,
	pub sky_pipeline: wgpu::RenderPipeline,
}

impl Pipeline {
	/// Creates all render pipelines with proper configuration
	#[inline]
	pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, layouts: &[&wgpu::BindGroupLayout]) -> Self {
/*layouts = [
	&texture_manager.bind_group_layout(),
	&camera_system.bind_group_layout(),
	&chunk_bind_group_layout,
	&skybox_bind_group_layout
	&post_bind_group_layout
];*/
		// Create shaders
		let shaders = Shaders::new(device);

		let post_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("Post Processing Pipeline Layout"),
			bind_group_layouts: &[layouts[4]],
			push_constant_ranges: &[],
		});
		let chunk_layout: wgpu::PipelineLayout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("Chunk Render Pipeline Layout"),
			bind_group_layouts: &[layouts[0],layouts[1],layouts[2]],
			..Default::default()
		});
		let sky_layout: wgpu::PipelineLayout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			label: Some("Sky Render Pipeline Layout"),
			bind_group_layouts: &[layouts[3],layouts[1]],
			..Default::default()
		});

		Self {
			chunk_pipeline: create_chunk_pipeline(device, &chunk_layout, &shaders.chunk, config.format),
			post_pipeline: create_post_pipeline(device, &post_layout, &shaders.post, config.format),
			sky_pipeline: create_sky_pipeline(device, &sky_layout, &shaders.sky, config.format),
		}
	}
}

/// Helper struct for organizing shader creation
struct Shaders {
	pub chunk: wgpu::ShaderModule,
	pub post: wgpu::ShaderModule,
	pub sky: wgpu::ShaderModule,
}

impl Shaders {
	#[inline]
	fn new(device: &wgpu::Device) -> Self {
		// Load shader sources first
		let chunk_shader = get_string!("chunk_shader.wgsl");
		let fxaa_shader = get_string!("fxaa.wgsl");
		let sky_shader = get_string!("sky_shader.wgsl");

		Self {
			chunk: create_shader(device, "Chunk Shader", &chunk_shader),
			post: create_shader(device, "Post Processing Shader", &fxaa_shader),
			sky: create_shader(device, "Sky Shader", &sky_shader),
		}
	}
}

/// Creates a shader module with the given label and source
#[inline]
fn create_shader(device: &wgpu::Device, label: &str, source: &str) -> wgpu::ShaderModule {
	device.create_shader_module(wgpu::ShaderModuleDescriptor {
		label: Some(label),
		source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::from(source)),
	})
}

/// Common pipeline creation helper
#[inline]
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
#[inline]
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
		&[Vertex::desc(), InstanceRaw::desc()],
		Some(depth_stencil_state()),
		default_primitive_state(),
		"Chunk Render Pipeline",
	)
}
#[inline]
fn create_post_pipeline(
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
		&[],
		None,
		default_primitive_state(),
		"Post Processing Pipeline",
	)
}
#[inline]
fn create_sky_pipeline(
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
		&[],
		None,
		default_primitive_state(),
		"Sky Render Pipeline",
	)
}


// --- Pipeline Configuration ---
#[inline]
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
#[inline]
fn depth_stencil_state() -> wgpu::DepthStencilState {
	wgpu::DepthStencilState {
		format: texture::DEPTH_FORMAT,
		depth_write_enabled: true,
		depth_compare: wgpu::CompareFunction::Less,
		stencil: wgpu::StencilState::default(),
		bias: wgpu::DepthBiasState::default(),
	}
}

// --- Optimized Render Passes ---
#[inline]
pub fn render_all(current_state: &mut State) -> Result<(), wgpu::SurfaceError> {
	let output = current_state.surface().get_current_texture()?;
	let view = output
		.texture
		.create_view(&wgpu::TextureViewDescriptor::default());
	let mut encoder =
		current_state
			.device()
			.create_command_encoder(&wgpu::CommandEncoderDescriptor {
				label: Some("Render Encoder"),
			});

	// Reusable render pass descriptors
	let binding = current_state.texture_manager().depth_texture().create_view(&wgpu::TextureViewDescriptor::default());
	{
		let mut sky_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("Sky Render Pass"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: &view,
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
		sky_pass.set_pipeline(&current_state.pipeline().sky_pipeline);
		
		sky_pass.set_bind_group(0, &current_state.skybox().bind_group, &[]);
		
		//sky_pass.set_bind_group(0, current_state.texture_manager().bind_group(), &[]);
		sky_pass.set_bind_group(1, current_state.camera_system().bind_group(), &[]);
		sky_pass.draw(0..36, 0..1);
	}

	// 3D pass
	if current_state.is_world_running {
		let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("3D Render Pass"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: &view,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Load,
					store: wgpu::StoreOp::Store,
				},
			})],
			depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
				view: &binding,
				depth_ops: Some(wgpu::Operations {
					load: wgpu::LoadOp::Clear(1.0),
					store: wgpu::StoreOp::Store,
				}),
				stencil_ops: None,
			}),
			occlusion_query_set: None,
			timestamp_writes: None,
		});

		// Render chunks
		rpass.set_pipeline(&current_state.pipeline().chunk_pipeline);
		rpass.set_bind_group(0, current_state.texture_manager().bind_group(), &[]);
		rpass.set_bind_group(1, current_state.camera_system().bind_group(), &[]);
		{
			// Create vertex buffer
	        let vertex_buffer = current_state.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
	            label: Some("Vertex Buffer"), contents: bytemuck::cast_slice(&VERTICES), usage: wgpu::BufferUsages::VERTEX });
	        rpass.set_vertex_buffer(0, vertex_buffer.slice(..));

			//let indices = vec![0, 1, 2, 2, 3, 0]; // Two triangles forming a quad
			/*let index_buffer = current_state.device().create_buffer_init(&wgpu::util::BufferInitDescriptor {
				label: Some("Index Buffer"),
				contents: bytemuck::cast_slice(&indices),
				usage: wgpu::BufferUsages::INDEX,
			});*/
			//rpass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
		}
		ptr::get_gamestate().world().render_chunks(&mut rpass);
	}

	// Post processing pass 
	{
		let mut post_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("FXAA Pass"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: &view,
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

		post_pass.set_pipeline(&current_state.pipeline().post_pipeline);
		post_pass.set_bind_group(0,current_state.texture_manager().post_processing_bind_group(),&[],);
		post_pass.draw(0..3, 0..1);
	}

	// UI pass 
	{
		let mut ui_rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("UI Render Pass"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: &view,
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

		current_state.ui_manager.render(&mut ui_rpass);
	}

	// Submit commands
	current_state.queue()
		.submit(std::iter::once(encoder.finish()));
	output.present();

	Ok(())
}
