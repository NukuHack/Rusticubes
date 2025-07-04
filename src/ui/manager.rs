
use crate::ui::element;
use crate::audio;
use crate::config;
use crate::ui::element::{UIElement, UIElementData};
use crate::ui::render::{UIRenderer, Vertex};
use crate::get_string;
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
    //Custom,       // this will be the exeption, aka we do not know the contents at compile time
}

pub struct UIManager {
    pub state: UIState,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
    pub elements: Vec<UIElement>,
    pub focused_element: Option<usize>,
    pub visibility: bool,
    renderer: UIRenderer,
    next_id: usize,
}

impl UIManager {
    pub fn new(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        queue: &wgpu::Queue,
    ) -> Self {
        let renderer = UIRenderer::new(device, queue);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[renderer.bind_group_layout(),renderer.uniform_bind_group_layout()], // Use the renderer's layout
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
                buffers: &[Vertex::desc()], // Your vertex layout
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
            visibility: true,
            renderer,
            next_id: 1,
        }
    }

    #[inline]
    pub fn renderer(&self) -> &UIRenderer {
        &self.renderer
    }
    #[inline]
    pub fn renderer_mut(&mut self) -> &mut UIRenderer {
        &mut self.renderer
    }

    pub fn update(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue) {
        let (vertices, indices) = self.renderer.process_elements(&self.elements);

        if !vertices.is_empty() {
            // Write vertices, overwriting old data (no explicit clear needed)
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        }

        if !indices.is_empty() {
            // Write indices, overwriting old data
            queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));
        }
    }

    pub fn update_anim(&mut self, delta: f32) {
        for element in self.elements.iter_mut() {
            if let UIElementData::Animation{ .. } = element.data {
                element.update_anim(delta);
            }
        }
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
        /*
        match self.state {
            UIState::None => {}
        }
        */
    }

    pub fn handle_key_input(&mut self, key: Key, shift: bool) {
        match key {
            Key::Backspace => self.handle_backspace(),
            Key::Enter => self.handle_enter(),
            Key::Escape => self.blur_current_element(),
            _ => {
                if let Some(c) = element::key_to_char(key, shift) {
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
                        element::handle_backspace(text_mut);
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
                    element::process_text_input(text_mut, c);
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
                    }
                    UIElementData::Checkbox { .. } => {
                        element.toggle_checked();
                        element.trigger_callback();
                    }
                    UIElementData::Button { .. } => {
                        audio::set_sound("click.ogg");
                        element.trigger_callback();
                    }

                    _ => {}
                }
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
        if !self.visibility {
            return;
        }

        self.renderer.render(self,render_pass);

    }

    pub fn next_id(&mut self) -> usize {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}


pub fn close_pressed() {
    match config::get_state().ui_manager.state {
        UIState::WorldSelection => {
            let state = config::get_state();
            state.ui_manager.state = UIState::BootScreen;
            state.ui_manager.setup_ui();
        }
        UIState::BootScreen => {
            config::close_app();
        }
        UIState::InGame => {
            let state = config::get_state();
            state.is_world_running = false;
            state.ui_manager.state = UIState::BootScreen;
            state.ui_manager.setup_ui();

            config::drop_gamestate();
        }
        UIState::Loading => {
            return; // hell nah- exiting while loading it like Bruh
        }
        UIState::None => {
            return; // why ???
        }
        UIState::NewWorld => {
            let ui_manager = &mut config::get_state().ui_manager;
            ui_manager.state = UIState::WorldSelection;
            ui_manager.setup_ui();
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
