use super::ui_element;
use super::ui_element::{UIElement, Vertex};
use super::ui_render::UIRenderer;
use crate::get_string;
use crate::ui_element::UIElementData;
use winit::keyboard::KeyCode as Key;

#[derive(Default, PartialEq)]
pub enum UIState {
    // need at least one more for error / popup handling
    BootScreen,     // Initial boot screen
    WorldSelection, // World selection screen
    InGame,         // Normal game UI
    Loading,        // Loading screen
    NewWorld,       // Make a new world
    #[default]
    None, // Baiscally not yet initialized
}

pub struct UIManager {
    pub state: UIState,
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
            source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::from(get_string!("ui_shader.wgsl"))),
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
            state: UIState::default(),
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

    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        let (vertices, indices) = self
            .renderer
            .process_elements(device, queue, &self.elements);

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
            // Check if we're removing the focused element
            if let Some(focused_pos) = self.focused_element {
                if focused_pos == pos {
                    self.focused_element = None;
                } else if focused_pos > pos {
                    self.focused_element = Some(focused_pos - 1);
                }
            }

            self.elements.remove(pos);
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
        self.next_id = 1;
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
        self.blur_current_element();
    }

    pub fn blur_current_element(&mut self) {
        self.focused_element = None;
    }

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

        // Find topmost clickable element
        for (index, element) in self.elements.iter_mut().enumerate().rev() {
            if element.visible && element.enabled && element.contains_point(norm_x, norm_y) {
                match &element.data {
                    UIElementData::InputField { .. } => {
                        self.focused_element = Some(index); // Store the vector index
                                                            //println!("Focused element at index: {}", index);
                    }
                    UIElementData::Checkbox { .. } => {
                        element.toggle_checked();
                    }
                    _ => {}
                }
                element.trigger_click();
                return true;
            }
        }
        false
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

    fn next_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

impl UIManager {
    pub fn setup_ui(&mut self) {
        self.clear_elements();

        let bg_panel =
            UIElement::new_panel(self.next_id(), (-1.0, -1.0), (2.0, 2.0), [0.08, 0.08, 0.12])
                .with_z_index(-5);

        match self.state {
            UIState::None => {
                self.state = UIState::BootScreen;
                self.setup_ui();
            }
            UIState::BootScreen => {
                self.add_element(bg_panel);
                self.setup_boot_screen_ui();
            }
            UIState::WorldSelection => {
                self.add_element(bg_panel);
                self.setup_world_selection_ui();
            }
            UIState::Loading => {
                self.add_element(bg_panel);
                self.setup_loading_screen_ui();
            }
            UIState::NewWorld => {
                self.add_element(bg_panel);
                self.setup_new_world_ui();
            }
            UIState::InGame => {
                self.setup_in_game_ui();
            }
        }
    }

    fn setup_boot_screen_ui(&mut self) {
        // Title with shadow effect
        let title = UIElement::new_label(
            self.next_id(),
            (-0.4, 0.3),
            (0.8, 0.2),
            [1.0, 1.0, 1.0],
            "Rusticubes".to_string(),
        )
        .with_border([0.9, 0.9, 0.9, 0.9], 0.005)
        .with_z_index(10);
        self.add_element(title);

        // Button container panel
        let button_panel =
            UIElement::new_panel(self.next_id(), (-0.35, -0.2), (0.7, 0.5), [0.15, 0.15, 0.2])
                .with_border([0.3, 0.3, 0.4, 1.0], 0.005)
                .with_z_index(4);
        self.add_element(button_panel);

        // Start button with hover effects
        let start_button = UIElement::new_button(
            self.next_id(),
            (-0.15, 0.0),
            (0.3, 0.1),
            [0.2, 0.5, 0.8],
            "Start".to_string(),
            || {
                let state = super::config::get_state();
                state.ui_manager.state = UIState::WorldSelection;
                state.ui_manager.setup_ui();
            },
        )
        .with_border([0.3, 0.6, 0.9, 1.0], 0.005)
        .with_z_index(5);
        self.add_element(start_button);

        // Exit button with hover effects
        let exit_button = UIElement::new_button(
            self.next_id(),
            (-0.15, -0.15),
            (0.3, 0.1),
            [0.8, 0.2, 0.2],
            "Exit".to_string(),
            || {
                close_pressed();
            },
        )
        .with_border([0.9, 0.3, 0.3, 1.0], 0.005)
        .with_z_index(5);
        self.add_element(exit_button);

        // Version label at bottom
        let version = UIElement::new_label(
            self.next_id(),
            (0.7, -0.95),
            (0.2, 0.05),
            [0.7, 0.7, 0.7],
            format!("v{}", env!("CARGO_PKG_VERSION")),
        )
        .with_border([0.5, 0.5, 0.5, 0.5], 0.002)
        .with_z_index(10);
        self.add_element(version);
    }

    fn setup_world_selection_ui(&mut self) {
        // Title with decorative border
        let title = UIElement::new_label(
            self.next_id(),
            (-0.4, 0.6),
            (0.8, 0.15),
            [1.0, 1.0, 1.0],
            "Select World".to_string(),
        )
        .with_border([0.7, 0.7, 0.8, 1.0], 0.005)
        .with_z_index(10);
        self.add_element(title);

        // World list container
        let list_panel =
            UIElement::new_panel(self.next_id(), (-0.5, -0.4), (1.0, 1.0), [0.15, 0.15, 0.2])
                .with_border([0.25, 0.25, 0.35, 1.0], 0.01)
                .with_z_index(5);
        self.add_element(list_panel);

        // New World button
        let new_w_button = UIElement::new_button(
            self.next_id(),
            (-0.3, 0.4),
            (0.6, 0.1),
            [0.3, 0.4, 0.6],
            "Create New World".to_string(),
            || {
                let state = super::config::get_state();
                state.ui_manager.state = UIState::NewWorld;
                state.ui_manager.setup_ui();
            },
        )
        .with_border([0.4, 0.5, 0.7, 1.0], 0.005)
        .with_z_index(10);
        self.add_element(new_w_button);

        let worlds = match super::file_manager::get_world_names() {
            Ok(worlds) => worlds,
            Err(e) => {
                eprintln!("Error loading world names: {}", e);
                Vec::new()
            }
        };

        // Add world buttons
        for (i, name) in worlds.iter().enumerate() {
            let y_pos = 0.2 - (i as f32 * 0.12);
            let world_name = name.clone();

            let world_button = UIElement::new_button(
                self.next_id(),
                (-0.4, y_pos),
                (0.8, 0.1),
                [0.25, 0.25, 0.4],
                name.clone(),
                move || {
                    let state = super::config::get_state();
                    state.ui_manager.state = UIState::Loading;
                    state.ui_manager.setup_ui();

                    println!("Loading world: {}", world_name);
                    super::start_world(&world_name);
                    state.ui_manager.state = UIState::InGame;
                    state.ui_manager.setup_ui();
                },
            )
            .with_border([0.35, 0.35, 0.5, 1.0], 0.003)
            .with_z_index(10);
            self.add_element(world_button);
        }

        // Back button with consistent styling
        let back_button = UIElement::new_button(
            self.next_id(),
            (-0.1, -0.8),
            (0.2, 0.08),
            [0.5, 0.5, 0.5],
            "Back".to_string(),
            || {
                let state = super::config::get_state();
                state.ui_manager.state = UIState::BootScreen;
                state.ui_manager.setup_ui();
            },
        )
        .with_border([0.6, 0.6, 0.6, 1.0], 0.005)
        .with_z_index(10);
        self.add_element(back_button);
    }

    fn setup_new_world_ui(&mut self) {
        // Title with decorative border
        let title = UIElement::new_label(
            self.next_id(),
            (-0.5, 0.4),
            (1.0, 0.15),
            [1.0, 1.0, 1.0],
            "Create New World".to_string(),
        )
        .with_border([0.7, 0.7, 0.8, 1.0], 0.005)
        .with_z_index(10);
        self.add_element(title);

        // Form panel
        let form_panel =
            UIElement::new_panel(self.next_id(), (-0.4, -0.3), (0.8, 0.7), [0.15, 0.15, 0.2])
                .with_border([0.25, 0.25, 0.35, 1.0], 0.01)
                .with_z_index(5);
        self.add_element(form_panel);

        // World name label
        let w_name_label = UIElement::new_label(
            self.next_id(),
            (-0.35, 0.1),
            (0.3, 0.08),
            [0.9, 0.9, 0.9],
            "World Name:".to_string(),
        )
        .with_z_index(10);
        self.add_element(w_name_label);

        // World Name input
        let input_id = self.next_id();
        let world_name_input = UIElement::new_input(
            input_id,
            (-0.35, -0.0),
            (0.7, 0.1),
            [0.2, 0.2, 0.3],
            Some("New World".to_string()),
        )
        .with_border([0.4, 0.4, 0.6, 1.0], 0.005)
        .with_z_index(10);
        self.add_element(world_name_input);

        // Generate button
        let gen_button = UIElement::new_button(
            self.next_id(),
            (-0.2, -0.2),
            (0.4, 0.1),
            [0.3, 0.4, 0.6],
            "Create World".to_string(),
            move || {
                let state = super::config::get_state();
                let world_name = state
                    .ui_manager
                    .get_input_text(input_id)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "New World".to_string());

                state.ui_manager.state = UIState::Loading;
                state.ui_manager.setup_ui();

                super::start_world(&world_name);
                state.ui_manager.state = UIState::InGame;
                state.ui_manager.setup_ui();
            },
        )
        .with_border([0.4, 0.5, 0.7, 1.0], 0.005)
        .with_z_index(10);
        self.add_element(gen_button);

        // Back button
        let back_button = UIElement::new_button(
            self.next_id(),
            (-0.1, -0.45),
            (0.2, 0.08),
            [0.5, 0.5, 0.5],
            "Back".to_string(),
            || {
                let state = super::config::get_state();
                state.ui_manager.state = UIState::WorldSelection;
                state.ui_manager.setup_ui();
            },
        )
        .with_border([0.6, 0.6, 0.6, 1.0], 0.005)
        .with_z_index(10);
        self.add_element(back_button);
    }

    fn setup_loading_screen_ui(&mut self) {
        // Loading panel
        let loading_panel =
            UIElement::new_panel(self.next_id(), (-0.3, -0.1), (0.6, 0.2), [0.1, 0.1, 0.15])
                .with_border([0.3, 0.3, 0.4, 1.0], 0.01)
                .with_z_index(10);
        self.add_element(loading_panel);

        // Loading text with animation
        let loading_text = UIElement::new_label(
            self.next_id(),
            (-0.25, -0.05),
            (0.5, 0.1),
            [1.0, 1.0, 1.0],
            "Loading...".to_string(),
        )
        .with_z_index(11);
        self.add_element(loading_text);

        // Progress bar background
        let progress_bg = UIElement::new_panel(
            self.next_id(),
            (-0.25, -0.15),
            (0.5, 0.03),
            [0.05, 0.05, 0.1],
        )
        .with_border([0.2, 0.2, 0.3, 1.0], 0.003)
        .with_z_index(11);
        self.add_element(progress_bg);

        // Progress bar (animated)
        let progress_bar = UIElement::new_panel(
            self.next_id(),
            (-0.245, -0.145),
            (0.01, 0.02), // Will be animated
            [0.3, 0.5, 0.8],
        )
        .with_z_index(12);
        self.add_element(progress_bar);
    }

    fn setup_in_game_ui(&mut self) {
        // Side panel for in-game UI
        let side_panel =
            UIElement::new_panel(self.next_id(), (0.4, -0.9), (0.6, 1.8), [0.1, 0.1, 0.15])
                .with_border([0.2, 0.2, 0.3, 1.0], 0.01)
                .with_z_index(5);
        self.add_element(side_panel);

        // Panel title
        let panel_title = UIElement::new_label(
            self.next_id(),
            (0.45, 0.75),
            (0.5, 0.1),
            [1.0, 1.0, 1.0],
            "Game Menu".to_string(),
        )
        .with_border([0.5, 0.5, 0.6, 1.0], 0.003)
        .with_z_index(10);
        self.add_element(panel_title);

        // Clean world button
        let clean_button = UIElement::new_button(
            self.next_id(),
            (0.45, 0.4),
            (0.5, 0.15),
            [0.6, 0.3, 0.3],
            "Clean World".to_string(),
            || {
                println!("Clean world button clicked!");
                super::cube_extra::add_full_world();
            },
        )
        .with_border([0.7, 0.4, 0.4, 1.0], 0.005)
        .with_z_index(10);
        self.add_element(clean_button);

        // Help text
        let help_text_1 = UIElement::new_label(
            self.next_id(),
            (0.5, 0.1),
            (0.4, 0.1),
            [1.0, 1.0, 1.0],
            "Press ALT to lock".to_string(),
        )
        .with_border([0.5, 0.5, 0.6, 1.0], 0.003)
        .with_z_index(7);
        self.add_element(help_text_1);

        let help_text_2 = UIElement::new_label(
            self.next_id(),
            (0.5, -0.05),
            (0.4, 0.1),
            [1.0, 1.0, 1.0],
            "Press L to fill chunk".to_string(),
        )
        .with_border([0.5, 0.5, 0.6, 1.0], 0.003)
        .with_z_index(7);
        self.add_element(help_text_2);

        let help_text_3 = UIElement::new_label(
            self.next_id(),
            (0.5, -0.2),
            (0.4, 0.1),
            [1.0, 1.0, 1.0],
            "Press R to break".to_string(),
        )
        .with_border([0.5, 0.5, 0.6, 1.0], 0.003)
        .with_z_index(7);
        self.add_element(help_text_3);

        let help_text_4 = UIElement::new_label(
            self.next_id(),
            (0.5, -0.35),
            (0.4, 0.1),
            [1.0, 1.0, 1.0],
            "Press F to place".to_string(),
        )
        .with_border([0.5, 0.5, 0.6, 1.0], 0.003)
        .with_z_index(7);
        self.add_element(help_text_4);

        let help_text_5 = UIElement::new_label(
            self.next_id(),
            (0.5, -0.5),
            (0.4, 0.1),
            [1.0, 1.0, 1.0],
            "Press ESC to leave".to_string(),
        )
        .with_border([0.5, 0.5, 0.6, 1.0], 0.003)
        .with_z_index(7);
        self.add_element(help_text_5);

        // Close button
        let close_button = UIElement::new_button(
            self.next_id(),
            (0.55, -0.8),
            (0.3, 0.1),
            [0.8, 0.2, 0.2],
            "Exit Game".to_string(),
            || {
                println!("Close button clicked!");
                close_pressed();
            },
        )
        .with_border([0.9, 0.3, 0.3, 1.0], 0.005)
        .with_z_index(10);
        self.add_element(close_button);

        // Crosshair with better visibility
        let crosshair_v =
            UIElement::new_divider(self.next_id(), (0.0, -0.02), (0.02, 0.06), [1.0, 1.0, 1.0])
                .with_z_index(20);
        let crosshair_h =
            UIElement::new_divider(self.next_id(), (-0.02, 0.0), (0.06, 0.02), [1.0, 1.0, 1.0])
                .with_z_index(20);

        self.add_element(crosshair_v);
        self.add_element(crosshair_h);
        /*
                // Status bar at bottom
                let status_bar =
                    UIElement::new_panel(self.next_id(), (-1.0, -0.96), (1.4, 0.06), [0.1, 0.1, 0.15])
                        .with_border([0.2, 0.2, 0.3, 1.0], 0.005)
                        .with_z_index(10);
                self.add_element(status_bar);

                let coord_text = UIElement::new_label(
                    self.next_id(),
                    (-0.95, -0.93),
                    (0.3, 0.05),
                    [0.8, 0.8, 0.8],
                    "X: 0, Y: 0, Z: 0".to_string(),
                )
                .with_z_index(11);
                self.add_element(coord_text);
        */
    }
}

pub fn close_pressed() {
    match super::config::get_state().ui_manager.state {
        UIState::WorldSelection => {
            super::config::get_state().ui_manager.state = UIState::BootScreen;
            super::config::get_state().ui_manager.setup_ui();
        }
        UIState::BootScreen => {
            super::config::close_app();
        }
        UIState::InGame => {
            super::config::get_state().is_world_running = false;
            super::config::get_state().ui_manager.state = UIState::BootScreen;
            super::config::get_state().ui_manager.setup_ui();

            super::config::drop_gamestate();
        }
        UIState::Loading => {
            return; // hell nah- exiting while loading it like Bruh
        }
        UIState::None => {
            return; // why ???
        }
        UIState::NewWorld => {
            super::config::get_state().ui_manager.state = UIState::WorldSelection;
            super::config::get_state().ui_manager.setup_ui();
        }
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
