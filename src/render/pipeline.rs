
use crate::block::extra::RENDER_DISTANCE;
use crate::ext::ptr;
use crate::render::meshing::{Vertex, InstanceRaw, VERTICES};
use crate::render::texture;
use crate::get_string;
use crate::State;
use wgpu::{
	util::DeviceExt, util::BufferInitDescriptor, RenderPipeline, Device, SurfaceConfiguration, BindGroupLayout, CompareFunction, CommandEncoderDescriptor,
	ShaderModuleDescriptor, ShaderModule, PipelineLayout, PipelineLayoutDescriptor, StencilState, LoadOp, StoreOp, Operations,
	ShaderSource, TextureFormat, VertexBufferLayout, PrimitiveState, DepthStencilState, DepthBiasState, RenderPassDescriptor,
	RenderPipelineDescriptor, VertexState, FragmentState, ColorTargetState, BlendState, SurfaceError, BufferUsages, RenderPassDepthStencilAttachment,
	ColorWrites, MultisampleState, PrimitiveTopology, Face, FrontFace, PolygonMode, TextureViewDescriptor, RenderPassColorAttachment,
};

/// Struct holding all render pipelines and their associated shaders
#[allow(dead_code)]
pub struct Pipeline {
	// Pipelines
	pub chunk_pipeline: RenderPipeline,
	pub post_pipeline: RenderPipeline,
	pub sky_pipeline: RenderPipeline,
	pub debug_pipeline: RenderPipeline,
}

impl Pipeline {
	/// Creates all render pipelines with proper configuration
	#[inline]
	pub fn new(device: &Device, config: &SurfaceConfiguration, layouts: &[BindGroupLayout]) -> Self {
		// Create shaders
		let shaders = Shaders::new(device);

		let post_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
			label: Some("Post Processing Pipeline Layout"),
			bind_group_layouts: &[&layouts[4]],
			..Default::default()
		});
		let chunk_layout: PipelineLayout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
			label: Some("Chunk Render Pipeline Layout"),
			bind_group_layouts: &[&layouts[0],&layouts[1],&layouts[2]],
			..Default::default()
		});
		let sky_layout: PipelineLayout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
			label: Some("Sky Render Pipeline Layout"),
			bind_group_layouts: &[&layouts[3],&layouts[1]],
			..Default::default()
		});
		// FIXED: Debug layout now includes both line storage buffer AND camera uniform
		let debug_layout: PipelineLayout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
			label: Some("Debug Pipeline Layout"),
			bind_group_layouts: &[&layouts[5], &layouts[1]], // Line buffer + Camera
			..Default::default()
		});

		Self {
			chunk_pipeline: create_chunk_pipeline(device, &chunk_layout, &shaders.chunk, config.format),
			post_pipeline: create_post_pipeline(device, &post_layout, &shaders.post, config.format),
			sky_pipeline: create_sky_pipeline(device, &sky_layout, &shaders.sky, config.format),
			debug_pipeline: create_debug_pipeline(device, &debug_layout, &shaders.debug, config.format),
		}
	}
}

/// Helper struct for organizing shader creation
struct Shaders {
	pub chunk: ShaderModule,
	pub post: ShaderModule,
	pub sky: ShaderModule,
	pub debug: ShaderModule,
}

impl Shaders {
	#[inline]
	fn new(device: &Device) -> Self {
		// Load shader sources first
		let chunk_shader = get_string!("chunk_shader.wgsl");
		let fxaa_shader = get_string!("fxaa.wgsl");
		let sky_shader = get_string!("sky_shader.wgsl");
		let debug_shader = get_string!("debug_shader.wgsl");

		Self {
			chunk: create_shader(device, "Chunk Shader", &chunk_shader),
			post: create_shader(device, "Post Processing Shader", &fxaa_shader),
			sky: create_shader(device, "Sky Shader", &sky_shader),
			debug: create_shader(device, "Debug Shader", &debug_shader),
		}
	}
}

/// Creates a shader module with the given label and source
#[inline]
fn create_shader(device: &Device, label: &str, source: &str) -> ShaderModule {
	device.create_shader_module(ShaderModuleDescriptor {
		label: Some(label),
		source: ShaderSource::Wgsl(std::borrow::Cow::from(source)),
	})
}

/// Common pipeline creation helper
#[inline]
fn create_base_pipeline(
	device: &Device,
	layout: Option<&PipelineLayout>,
	shader: &ShaderModule,
	format: TextureFormat,
	buffers: &[VertexBufferLayout],
	depth_stencil: Option<DepthStencilState>,
	primitive: PrimitiveState,
	label: &str,
) -> RenderPipeline {
	device.create_render_pipeline(&RenderPipelineDescriptor {
		label: Some(label),
		layout,
		vertex: VertexState {
			module: shader,
			entry_point: Some("vs_main"),
			compilation_options: Default::default(),
			buffers,
		},
		fragment: Some(FragmentState {
			module: shader,
			entry_point: Some("fs_main"),
			compilation_options: Default::default(),
			targets: &[Some(ColorTargetState {
				format,
				blend: Some(BlendState::ALPHA_BLENDING),
				write_mask: ColorWrites::ALL,
			})],
		}),
		primitive,
		depth_stencil,
		multisample: MultisampleState::default(),
		multiview: None,
		cache: None,
	})
}
#[inline]
fn create_chunk_pipeline(
	device: &Device,
	layout: &PipelineLayout,
	shader: &ShaderModule,
	format: TextureFormat,
) -> RenderPipeline {
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
fn create_debug_pipeline(
	device: &Device,
	layout: &PipelineLayout,
	shader: &ShaderModule,
	format: TextureFormat,
) -> RenderPipeline {
	create_base_pipeline(
		device,
		Some(layout),
		shader,
		format,
		&[], // FIXED: No vertex buffers needed - we generate vertices in shader
		Some(depth_stencil_state()),
		PrimitiveState {
			topology: PrimitiveTopology::LineList,
			strip_index_format: None,
			front_face: FrontFace::Ccw,
			cull_mode: None, // No culling for lines
			polygon_mode: PolygonMode::Fill, // CHANGED: Use Fill, not Line
			conservative: false,
			unclipped_depth: false,
			..Default::default()
		},
		"Debug Render Pipeline",
	)
}
#[inline]
fn create_post_pipeline(
	device: &Device,
	layout: &PipelineLayout,
	shader: &ShaderModule,
	format: TextureFormat,
) -> RenderPipeline {
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
	device: &Device,
	layout: &PipelineLayout,
	shader: &ShaderModule,
	format: TextureFormat,
) -> RenderPipeline {
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
fn default_primitive_state() -> PrimitiveState {
	PrimitiveState {
		topology: PrimitiveTopology::TriangleList,
		strip_index_format: None,
		front_face: FrontFace::Ccw,
		cull_mode: Some(Face::Back),
		polygon_mode: PolygonMode::Fill,
		conservative: false,
		unclipped_depth: false,
		..Default::default()
	}
}
#[inline]
fn depth_stencil_state() -> DepthStencilState {
	DepthStencilState {
		format: texture::DEPTH_FORMAT,
		depth_write_enabled: true,
		depth_compare: CompareFunction::Less,
		stencil: StencilState::default(),
		bias: DepthBiasState::default(),
	}
}

// --- Optimized Render Passes ---
#[inline]
pub fn render_all(current_state: &mut State) -> Result<(), SurfaceError> {
	let output = current_state.surface().get_current_texture()?;
	let view = output.texture.create_view(&TextureViewDescriptor::default());
	let mut encoder = current_state.device().create_command_encoder(&CommandEncoderDescriptor { label: Some("Render Encoder") });

	// 3D pass
	if current_state.is_world_running {
		// Reusable render pass descriptors
		let game_state = ptr::get_gamestate();
		let binding = current_state.texture_manager().depth_texture().create_view(&TextureViewDescriptor::default());
		{
			let mut sky_pass = encoder.begin_render_pass(&RenderPassDescriptor {
				label: Some("Sky Render Pass"),
				color_attachments: &[Some(RenderPassColorAttachment {
					view: &view,
					resolve_target: None,
					ops: Operations {
						load: LoadOp::Load,
						store: StoreOp::Store,
					},
				})],
				depth_stencil_attachment: None,
				occlusion_query_set: None,
				timestamp_writes: None,
			});
			sky_pass.set_pipeline(&current_state.pipeline().sky_pipeline);
			
			sky_pass.set_bind_group(0, &current_state.skybox().bind_group, &[]);
			
			//sky_pass.set_bind_group(0, current_state.texture_manager().bind_group(), &[]);
			sky_pass.set_bind_group(1, game_state.player().camera_system().bind_group(), &[]);
			sky_pass.draw(0..36, 0..1); // 36 = 6 (side) * 6 (2 triangles)
		}
		{
			let mut rpass = encoder.begin_render_pass(&RenderPassDescriptor {
				label: Some("3D Render Pass"),
				color_attachments: &[Some(RenderPassColorAttachment {
					view: &view,
					resolve_target: None,
					ops: Operations {
						load: LoadOp::Load,
						store: StoreOp::Store,
					},
				})],
				depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
					view: &binding,
					depth_ops: Some(Operations {
						load: LoadOp::Clear(1.0),
						store: StoreOp::Store,
					}),
					stencil_ops: None,
				}),
				occlusion_query_set: None,
				timestamp_writes: None,
			});

			// Render chunks
			rpass.set_pipeline(&current_state.pipeline().chunk_pipeline);
			rpass.set_bind_group(0, current_state.texture_manager().bind_group(), &[]);
			let cam_sys = game_state.player().camera_system();
			rpass.set_bind_group(1, cam_sys.bind_group(), &[]);
			{
				// Create vertex buffer
				let vertex_buffer = current_state.device().create_buffer_init(&BufferInitDescriptor {
					label: Some("Vertex Buffer"), contents: bytemuck::cast_slice(&VERTICES), usage: BufferUsages::VERTEX });
				rpass.set_vertex_buffer(0, vertex_buffer.slice(..));
			}
			ptr::get_gamestate().world().render_chunks_with_culling(&mut rpass, cam_sys, RENDER_DISTANCE);
		}
		{
			let game_state = ptr::get_gamestate();
			
			// Only create debug render pass if there are lines to render
			if game_state.debug().lines.len() > 0 {
				let mut debug_pass = encoder.begin_render_pass(&RenderPassDescriptor {
					label: Some("Debug Render Pass"),
					color_attachments: &[Some(RenderPassColorAttachment {
						view: &view,
						resolve_target: None,
						ops: Operations {
							load: LoadOp::Load,
							store: StoreOp::Store,
						},
					})],
					depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
						view: &binding,
						depth_ops: Some(Operations {
							load: LoadOp::Load,
							store: StoreOp::Store,
						}),
						stencil_ops: None,
					}),
					occlusion_query_set: None,
					timestamp_writes: None,
				});
				debug_pass.set_pipeline(&current_state.pipeline().debug_pipeline);
				debug_pass.set_bind_group(1, game_state.player().camera_system().bind_group(), &[]);
				game_state.debug().render(&mut debug_pass);
			}
		}
	}

	// Post processing pass 
	{
		let mut post_pass = encoder.begin_render_pass(&RenderPassDescriptor {
			label: Some("FXAA Pass"),
			color_attachments: &[Some(RenderPassColorAttachment {
				view: &view,
				resolve_target: None,
				ops: Operations {
					load: LoadOp::Load,
					store: StoreOp::Store,
				},
			})],
			depth_stencil_attachment: None,
			occlusion_query_set: None,
			timestamp_writes: None,
		});

		post_pass.set_pipeline(&current_state.pipeline().post_pipeline);
		post_pass.set_bind_group(0,current_state.texture_manager().post_bind_group(),&[],);
		post_pass.draw(0..3, 0..1);
	}

	// UI pass 
	{
		let mut ui_rpass = encoder.begin_render_pass(&RenderPassDescriptor {
			label: Some("UI Render Pass"),
			color_attachments: &[Some(RenderPassColorAttachment {
				view: &view,
				resolve_target: None,
				ops: Operations {
					load: LoadOp::Load,
					store: StoreOp::Store,
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
