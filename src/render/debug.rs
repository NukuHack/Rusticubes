
use wgpu::util::DeviceExt;
use glam::Vec3;
use std::mem;

// =============================================
// Vertex Definition
// =============================================

/// A vertex with position, normal
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
pub struct Line {
	pub start: [f32;3],
	pub direction: [f32;3], // this is the ending point relative to the start position
}

impl Line {
	/// Describes the vertex buffer layout for wgpu
	pub const fn desc() -> wgpu::VertexBufferLayout<'static> {
		wgpu::VertexBufferLayout {
			array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
			step_mode: wgpu::VertexStepMode::Vertex,
			attributes: &[
				// Packed all
				wgpu::VertexAttribute {
					offset: 0,
					shader_location: 0,
					format: wgpu::VertexFormat::Float32x3,
				},
				wgpu::VertexAttribute {
					offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress, // 12
					shader_location: 1,
					format: wgpu::VertexFormat::Float32x3,
				},
			],
		}
	}
	pub const fn new(pos: Vec3, line: Vec3) -> Self {
		Self{
			start: [pos.x,pos.y,pos.z],
			direction: [line.x,line.y,line.z]
		}
	}
}
//Line::new(Vec3::new(0_f32,0_f32,0_f32), Vec3::new(100_f32,100_f32,100_f32))

pub struct DebugLines {
	pub line_buffer: LineBuffer,
	pub lines: Vec<Line>,
}

impl DebugLines {
	pub fn default(device: &wgpu::Device) -> Self {
		Self { // will crash
			line_buffer: LineBuffer::new(device, &Vec::new()),
			lines: Vec::new(),
		}
	}

	pub fn new(device: &wgpu::Device, lines: Vec<Line>) -> Self {
		Self {
			line_buffer: LineBuffer::new(device, &lines),
			lines,
		}
	}

	pub fn add_line(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, line: Line) {
		self.lines.push(line);
		self.line_buffer.update(device, queue, &self.lines);
	}
	pub fn clear_lines(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
		self.lines.clear();
		self.line_buffer.update(device, queue, &self.lines);
	}

	pub fn add_line_deferred(&mut self, line: Line) {
		self.lines.push(line);
		// Don't update buffer immediately
	}
	pub fn clear_lines_deferred(&mut self) {
		self.lines.clear();
		// Don't update buffer immediately
	}
	
	pub fn flush_updates(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
		self.line_buffer.update(device, queue, &self.lines);
	}
	
	pub fn render<'a>(&'a self, debug_pass: &mut wgpu::RenderPass<'a>) {
		let line_count = self.lines.len();
		if line_count == 0 {  return; } // Early return for empty lines
		
		debug_pass.set_bind_group(0, &self.line_buffer.bind_group, &[]);
		debug_pass.draw(0..2, 0..line_count as u32);
	}
}

pub struct LineBuffer {
	pub buffer: wgpu::Buffer,
	pub bind_group: wgpu::BindGroup,
	pub bind_group_layout: wgpu::BindGroupLayout,
}

impl LineBuffer {
	pub fn new(device: &wgpu::Device, lines: &[Line]) -> Self {
		// Create buffer
		let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
			label: Some("Line Buffer"),
			contents: bytemuck::cast_slice(lines),
			usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
		});

		// Create bind group layout
		let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
			label: Some("Line Bind Group Layout"),
			entries: &[
				wgpu::BindGroupLayoutEntry {
					binding: 0,
					visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
					ty: wgpu::BindingType::Buffer {
						ty: wgpu::BufferBindingType::Storage { read_only: true },
						has_dynamic_offset: false,
						min_binding_size: None,
					},
					count: None,
				},
			],
		});

		// Create bind group
		let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("Line Bind Group"),
			layout: &bind_group_layout,
			entries: &[
				wgpu::BindGroupEntry {
					binding: 0,
					resource: buffer.as_entire_binding(),
				},
			],
		});

		Self {
			buffer,
			bind_group,
			bind_group_layout,
		}
	}

	pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, lines: &[Line]) {
		let required_size = (lines.len() * std::mem::size_of::<Line>()) as u64;
		
		// Recreate buffer if it's too small
		if self.buffer.size() < required_size {
			self.recreate_buffer(device, queue, lines);
		} else {
			queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(lines));
		}
	}
	
	fn recreate_buffer(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, lines: &[Line]) {
		// Create larger buffer with some headroom
		let buffer_size = ((lines.len() * 2).max(16) * std::mem::size_of::<Line>()) as u64;
		
		self.buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label: Some("Line Buffer"),
			size: buffer_size,
			usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});
		
		// Recreate bind group
		self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
			label: Some("Line Bind Group"),
			layout: &self.bind_group_layout,
			entries: &[wgpu::BindGroupEntry {
				binding: 0,
				resource: self.buffer.as_entire_binding(),
			}],
		});
		
		// Write data
		queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(lines));
	}
}
