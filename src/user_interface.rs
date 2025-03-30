#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
    color: [f32; 4],
}

#[allow(
    dead_code,
    unused,
    redundant_imports,
    unused_results,
    unused_features,
    unused_variables,
    unused_mut,
    dead_code,
    unused_unsafe,
    unused_attributes
)]
pub struct UIElement {
    pub position: (f32, f32), // Normalized coordinates (-1.0 to 1.0)
    pub size: (f32, f32),
    pub color: [f32; 4],
    pub hovered: bool,
    pub on_click: Option<Box<dyn FnMut()>>,
}
impl UIElement {
    pub fn default() -> Self {
        Self {
            position: (-0.9, -0.8),      // Centered in the viewport
            size: (0.2, 0.1),            // 20% width and 10% height of the viewport
            color: [0.5, 0.8, 0.1, 1.0], // Default to purple (fully opaque)
            hovered: false,
            on_click: Some(Box::new(|| {
                println!("Button clicked!");
            })),
        }
    }
}

#[allow(
    dead_code,
    unused,
    redundant_imports,
    unused_results,
    unused_features,
    unused_variables,
    unused_mut,
    dead_code,
    unused_unsafe,
    unused_attributes
)]
pub struct UIManager {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
    pub elements: Vec<UIElement>,
    pub num_indices: u32, // Total number of indices across all UI elements
    pub visibility: bool,
}

impl UIManager {
    pub fn update(&mut self, queue: &wgpu::Queue) {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut current_index = 0;

        // Collect vertices and indices for all UI elements
        for element in &self.elements {
            let (x, y) = element.position;
            let (w, h) = element.size;
            let color = element.color;

            // Define the four vertices of the UI element
            let positions = [[x, y], [x + w, y], [x, y + h], [x + w, y + h]];

            // Add vertices to the list
            for &pos in &positions {
                vertices.push(Vertex {
                    position: pos,
                    color,
                });
            }

            // Define indices for the two triangles forming the quad
            indices.extend_from_slice(&[
                current_index + 0,
                current_index + 1,
                current_index + 2,
                current_index + 1,
                current_index + 3,
                current_index + 2,
            ]);
            current_index += 4;
        }

        // Write vertices to the GPU buffer
        let vertex_data = bytemuck::cast_slice(&vertices);
        queue.write_buffer(&self.vertex_buffer, 0, vertex_data);

        // Write indices to the GPU buffer
        let index_data = bytemuck::cast_slice(&indices);
        queue.write_buffer(&self.index_buffer, 0, index_data);

        // Update the total number of indices
        self.num_indices = indices.len() as u32;
    }

    pub fn add_ui_element(&mut self, element: self::UIElement) {
        self.elements.push(element);
    }

    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration) -> Self {
        // UI Pipeline
        let ui_shader: wgpu::ShaderModule =
            device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("UI Shader"),
                source: wgpu::ShaderSource::Wgsl(include_str!("ui_shader.wgsl").into()),
            });

        let ui_pipeline_layout: wgpu::PipelineLayout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("UI Pipeline Layout"),
                bind_group_layouts: &[],
                ..Default::default()
            });

        let ui_pipeline: wgpu::RenderPipeline =
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("UI Pipeline"),
                layout: Some(&ui_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &ui_shader,
                    entry_point: Some("vs_main"),
                    compilation_options: Default::default(),
                    buffers: &[wgpu::VertexBufferLayout {
                        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
                        step_mode: wgpu::VertexStepMode::Vertex,
                        attributes: &[
                            wgpu::VertexAttribute {
                                offset: 0,
                                shader_location: 0,
                                format: wgpu::VertexFormat::Float32x2,
                            },
                            wgpu::VertexAttribute {
                                offset: 8,
                                shader_location: 1,
                                format: wgpu::VertexFormat::Float32x4,
                            },
                        ],
                    }],
                },
                fragment: Some(wgpu::FragmentState {
                    module: &ui_shader,
                    entry_point: Some("fs_main"),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: config.format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                multiview: None,
                cache: None,
            });

        // UI Buffers
        let vertex_buffer: wgpu::Buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI Vertex Buffer"),
            size: 1024 * std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer: wgpu::Buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI Index Buffer"),
            size: 1024 * std::mem::size_of::<u32>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            vertex_buffer,
            index_buffer,
            pipeline: ui_pipeline,
            elements: Vec::new(),
            num_indices: 0, // Initialize to 0
            visibility: true,
        }
    }
}

pub fn handle_ui_hover(
    current_state: &mut super::State,
    position: &winit::dpi::PhysicalPosition<f64>,
) {
    let (x, y) = (position.x as f32, position.y as f32);
    let (w, h) = (
        current_state.size.width as f32,
        current_state.size.height as f32,
    );

    // Convert to UI's normalized coordinates (-1.0 to 1.0)
    let norm_x = (2.0 * x / w) - 1.0;
    let norm_y = (2.0 * (h - y) / h) - 1.0; // Invert Y-axis

    for element in &mut current_state.ui_manager.elements {
        let (pos_x, pos_y) = element.position;
        let (size_w, size_h) = element.size;

        // Calculate element's bounds in normalized coordinates
        let right = pos_x + size_w;
        let top = pos_y + size_h;

        // Check if mouse is inside the element's bounds
        if norm_x >= pos_x && norm_x <= right && norm_y >= pos_y && norm_y <= top {
            element.hovered = true;
            element.color[2] = 0.5; // Green tint on hover
        } else {
            element.hovered = false;
            element.color[2] = 0.9; // Default color
        }
    }
}

pub fn handle_ui_click(current_state: &mut super::State) {
    for element in &mut current_state.ui_manager.elements {
        if element.hovered {
            // run the function
            element.on_click.as_deref_mut().unwrap()();
        }
    }
}
