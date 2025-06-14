use super::ui_element;
use super::ui_element::{UIElement, Vertex};
use super::ui_render::UIRenderer;
use winit::keyboard::KeyCode as Key;

pub struct UIManager {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
    pub elements: Vec<UIElement>,
    pub focused_element: Option<usize>,
    pub num_indices: u32,
    pub visibility: bool,
    pub renderer: UIRenderer,
    next_id: usize,
}

impl UIManager {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        queue: &wgpu::Queue,
    ) -> Self {
        let renderer = UIRenderer::new(device, queue);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
            ],
            label: Some("font_bind_group_layout"),
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[&bind_group_layout],
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("UI Shader"),
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::from(include_str!(
                "ui_shader.wgsl"
            ))),
        });

        let ui_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("UI Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
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
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: 16,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI Vertex Buffer"),
            size: 2048 * std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("UI Index Buffer"),
            size: 2048 * std::mem::size_of::<u32>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            vertex_buffer,
            index_buffer,
            pipeline: ui_pipeline,
            elements: Vec::new(),
            focused_element: None,
            num_indices: 0,
            visibility: true,
            renderer,
            next_id: 1,
        }
    }

    pub fn update(&mut self, queue: &wgpu::Queue) {
        let (vertices, indices) = self.renderer.process_elements(&self.elements);

        if !vertices.is_empty() {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        }

        if !indices.is_empty() {
            queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));
        }

        self.num_indices = indices.len() as u32;
    }

    // Element management methods
    pub fn add_element(&mut self, mut element: UIElement) -> usize {
        if element.id == 0 {
            element.id = self.next_id;
            self.next_id += 1;
        }
        let id = element.id;
        self.elements.push(element);
        id
    }

    pub fn remove_element(&mut self, id: usize) -> bool {
        if let Some(pos) = self.elements.iter().position(|e| e.id == id) {
            self.elements.remove(pos);

            // Update focused element if necessary
            if let Some(focused_id) = self.focused_element {
                if focused_id == pos {
                    self.focused_element = None;
                } else if focused_id > pos {
                    self.focused_element = Some(focused_id - 1);
                }
            }
            true
        } else {
            false
        }
    }

    pub fn get_element(&self, id: usize) -> Option<&UIElement> {
        self.elements.iter().find(|e| e.id == id)
    }

    pub fn get_element_mut(&mut self, id: usize) -> Option<&mut UIElement> {
        self.elements.iter_mut().find(|e| e.id == id)
    }

    pub fn get_input_text(&self, id: usize) -> Option<&str> {
        self.elements
            .iter()
            .find(|e| e.id == id && e.is_input())
            .and_then(|e| e.get_text())
    }

    pub fn set_element_visibility(&mut self, id: usize, visible: bool) {
        if let Some(element) = self.get_element_mut(id) {
            element.visible = visible;
        }
    }

    pub fn set_element_enabled(&mut self, id: usize, enabled: bool) {
        if let Some(element) = self.get_element_mut(id) {
            element.enabled = enabled;
        }
    }

    pub fn set_element_text(&mut self, id: usize, text: String) {
        if let Some(element) = self.get_element_mut(id) {
            if let Some(text_mut) = element.get_text_mut() {
                *text_mut = text;
            }
        }
    }

    pub fn clear_elements(&mut self) {
        self.elements.clear();
        self.focused_element = None;
    }

    // Input handling methods
    pub fn process_text_input(&mut self, c: char) {
        if let Some(focused_idx) = self.focused_element {
            if let Some(element) = self.elements.get_mut(focused_idx) {
                if !element.is_input() || !element.enabled {
                    return;
                }

                if let Some(text_mut) = element.get_text_mut() {
                    ui_element::process_text_input(text_mut, c);
                }
            }
        }
    }

    pub fn handle_backspace(&mut self) {
        if let Some(focused_idx) = self.focused_element {
            if let Some(element) = self.elements.get_mut(focused_idx) {
                if element.is_input() && element.enabled {
                    if let Some(text_mut) = element.get_text_mut() {
                        ui_element::handle_backspace(text_mut);
                    }
                }
            }
        }
    }

    pub fn handle_enter(&mut self) {
        self.focused_element = None;
    }

    pub fn handle_key_input(&mut self, key: Key, shift: bool) {
        match key {
            Key::Backspace => self.handle_backspace(),
            Key::Enter => self.handle_enter(),
            Key::Escape => self.blur_current_element(),
            _ => {
                if let Some(c) = ui_element::key_to_char(key, shift) {
                    self.process_text_input(c);
                }
            }
        }
    }

    pub fn blur_current_element(&mut self) {
        self.focused_element = None;
    }

    pub fn toggle_visibility(&mut self) {
        self.visibility = !self.visibility;
    }

    // Mouse interaction methods
    pub fn handle_hover(&mut self, norm_x: f32, norm_y: f32) {
        for element in &mut self.elements {
            let is_hovered = element.contains_point(norm_x, norm_y);
            element.update_hover_state(is_hovered);
        }
    }

    pub fn handle_click(&mut self, norm_x: f32, norm_y: f32) -> bool {
        self.focused_element = None;
        let mut handled = false;

        // Process elements in reverse z-order (highest z-index first)
        let mut sorted_indices: Vec<usize> = (0..self.elements.len()).collect();
        sorted_indices.sort_by_key(|&i| std::cmp::Reverse(self.elements[i].z_index));

        for &index in &sorted_indices {
            let element = &mut self.elements[index];

            if element.contains_point(norm_x, norm_y) && element.visible && element.enabled {
                match &element.data {
                    super::ui_element::UIElementData::InputField { .. } => {
                        self.focused_element = Some(index);
                        handled = true;
                        break;
                    }
                    super::ui_element::UIElementData::Checkbox { .. } => {
                        element.toggle_checked();
                        element.trigger_click();
                        handled = true;
                        break;
                    }
                    super::ui_element::UIElementData::Button { .. } => {
                        element.trigger_click();
                        handled = true;
                        break;
                    }
                    _ => {
                        element.trigger_click();
                        handled = true;
                        break;
                    }
                }
            }
        }

        handled
    }

    // Utility methods
    pub fn is_any_element_hovered(&self) -> bool {
        self.elements
            .iter()
            .any(|e| e.hovered && e.visible && e.enabled)
    }

    pub fn get_focused_element(&self) -> Option<&UIElement> {
        self.focused_element.and_then(|idx| self.elements.get(idx))
    }

    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if !self.visibility || self.num_indices == 0 {
            return;
        }

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.renderer.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
    }
}

// Utility functions for mouse coordinate conversion and UI setup
pub fn convert_mouse_position(
    window_size: (u32, u32),
    mouse_pos: &winit::dpi::PhysicalPosition<f64>,
) -> (f32, f32) {
    let x = mouse_pos.x as f32;
    let y = mouse_pos.y as f32;
    let width = window_size.0 as f32;
    let height = window_size.1 as f32;

    ((2.0 * x / width) - 1.0, (2.0 * (height - y) / height) - 1.0)
}

pub fn handle_ui_hover(
    ui_manager: &mut UIManager,
    window_size: (u32, u32),
    mouse_pos: &winit::dpi::PhysicalPosition<f64>,
) {
    let (norm_x, norm_y) = convert_mouse_position(window_size, mouse_pos);
    ui_manager.handle_hover(norm_x, norm_y);
}

pub fn handle_ui_click(
    ui_manager: &mut UIManager,
    window_size: (u32, u32),
    mouse_pos: &winit::dpi::PhysicalPosition<f64>,
) -> bool {
    let (norm_x, norm_y) = convert_mouse_position(window_size, mouse_pos);
    ui_manager.handle_click(norm_x, norm_y)
}

// Example UI setup function
pub fn setup_ui(ui_manager: &mut UIManager) {
    // Add a button
    let button = UIElement::new_button(
        1,
        (0.45, 0.4),
        (0.5, 0.25),
        [0.7, 0.3, 0.3],
        "Clean World".to_string(),
        || {
            println!("Clean world button clicked!");
            super::cube_extra::add_full_world();
        },
    )
    .with_border([0.8, 0.4, 0.4, 1.0], 0.005);
    ui_manager.add_element(button);

    // Add a close button
    let close_button = UIElement::new_button(
        2,
        (0.6, -0.7),
        (0.2, 0.1),
        [1.0, 0.2, 0.1],
        "Close".to_string(),
        || {
            println!("Close button clicked!");
            super::close_app();
        },
    )
    .with_border([0.2, 0.2, 0.2, 1.0], 0.003);
    ui_manager.add_element(close_button);

    // Add some dividers/panels
    let crosshair_v = UIElement::new_divider(5, (0.0, -0.02), (0.02, 0.06), [0.1, 0.1, 0.1]);
    let crosshair_h = UIElement::new_divider(6, (-0.02, 0.0), (0.06, 0.02), [0.1, 0.1, 0.1]);

    ui_manager.add_element(crosshair_v);
    ui_manager.add_element(crosshair_h);
}
