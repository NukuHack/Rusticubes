use crate::{
    ext::{audio, config},
    get_string,
    ui::{
        dialog,
        element::{self, UIElement, UIElementData},
        render::{UIRenderer, Vertex},
    },
};
use winit::keyboard::KeyCode as Key;

#[derive(PartialEq, Clone, Copy, Default)]
pub struct UIStateID(u32);

impl UIStateID {
    #[inline] pub fn new(id: u32) -> Self { Self(id) }
}

impl From<&UIState> for UIStateID {
    fn from(state: &UIState) -> Self {
        #[allow(unreachable_patterns)]
        match state {
            UIState::None => UIStateID(0),
            UIState::BootScreen => UIStateID(1),
            UIState::WorldSelection => UIStateID(2),
            UIState::Multiplayer => UIStateID(3),
            UIState::NewWorld => UIStateID(4),
            UIState::Escape => UIStateID(5),
            UIState::InGame => UIStateID(6),
            UIState::Settings(..) => UIStateID(7),
            UIState::Confirm(..) => UIStateID(8),
            UIState::Loading => UIStateID(9),            
            UIState::Error(..) => UIStateID(10),
            UIState::ConnectLocal => UIStateID(11),
            _ => UIStateID(0),
        }
    }
}

impl From<UIStateID> for UIState {
    fn from(id: UIStateID) -> Self {
        #[allow(unreachable_patterns)]
        match id.0 {
            0 => UIState::None,
            1 => UIState::BootScreen,
            2 => UIState::WorldSelection,
            3 => UIState::Multiplayer,
            4 => UIState::NewWorld,
            5 => UIState::Escape,
            6 => UIState::InGame,
            7 => UIState::Settings(UIStateID::default()),
            8 => UIState::Confirm(UIStateID::default(), 0),
            9 => UIState::Loading,
            10 => UIState::Error(UIStateID::default(), 0),
            11 => UIState::ConnectLocal,
            _ => UIState::None,
        }
    }
}

#[derive(PartialEq, Clone, Default)]
pub enum UIState {
    #[default] None,
    BootScreen,
    WorldSelection,
    Multiplayer,
    ConnectLocal,
    NewWorld,
    Escape,
    InGame,
    Settings(UIStateID),
    Loading,
    Confirm(UIStateID, u8),
    Error(UIStateID, u8),
}

impl UIState {
    pub fn inner(&self) -> Option<u8> {
        match self {
            UIState::Confirm(_, id) | UIState::Error(_, id) => Some(*id),
            _ => None,
        }
    }
    
    pub fn inner_state(&self) -> UIState {
        match self {
            UIState::Confirm(id, _) | UIState::Error(id, _) => UIState::from(*id),
            UIState::Settings(id) => UIState::from(*id),
            _ => UIState::None,
        }
    }
}

pub fn close_pressed() {
    let state = config::get_state();
    match state.ui_manager.state.clone() {
        UIState::WorldSelection | UIState::Multiplayer => {
            state.ui_manager.state = UIState::BootScreen;
        },
        UIState::BootScreen => config::close_app(),
        UIState::InGame => {
            state.ui_manager.state = UIState::Escape;
            let game_state = config::get_gamestate();
            game_state.player_mut().controller().reset_keyboard();
            *game_state.running() = false;
        },
        UIState::Escape => {
            state.ui_manager.state = UIState::InGame;
            *config::get_gamestate().running() = true;
        },
        UIState::NewWorld => state.ui_manager.state = UIState::WorldSelection,
        UIState::Error(prev_state, dialog_id) | UIState::Confirm(prev_state, dialog_id) => {
            state.ui_manager.dialogs.cancel_dialog(dialog_id);
            state.ui_manager.state = UIState::from(prev_state);
        },
        UIState::Settings(prev_state) => state.ui_manager.state = UIState::from(prev_state),
        UIState::ConnectLocal => state.ui_manager.state = UIState::WorldSelection,
        _ => return,
    }
    state.ui_manager.setup_ui();
}

pub struct UIManager {
    pub state: UIState,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub pipeline: wgpu::RenderPipeline,
    pub elements: Vec<UIElement>,
    pub focused_element: Option<usize>,
    pub visibility: bool,
    pub dialogs: dialog::DialogManager,
    renderer: UIRenderer,
    next_id: usize,
}

impl UIManager {
    #[inline]
    pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, queue: &wgpu::Queue) -> Self {
        let renderer = UIRenderer::new(device, queue);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            bind_group_layouts: &[renderer.bind_group_layout(), renderer.uniform_bind_group_layout()],
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
                buffers: &[Vertex::desc()],
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
            depth_stencil : None,
            primitive: wgpu::PrimitiveState::default(),
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
            state: Default::default(),
            vertex_buffer,
            index_buffer,
            pipeline: ui_pipeline,
            elements: Vec::new(),
            focused_element: None,
            visibility: true,
            dialogs: dialog::DialogManager::new(),
            renderer,
            next_id: 1,
        }
    }

    #[inline] pub fn renderer(&self) -> &UIRenderer { &self.renderer }
    #[inline] pub fn renderer_mut(&mut self) -> &mut UIRenderer { &mut self.renderer }
    
    #[inline]
    pub fn update(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue) {
        let (vertices, indices) = self.renderer.process_elements(&self.elements);
        if !vertices.is_empty() {
            queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
        }
        if !indices.is_empty() {
            queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));
        }
    }
    
    #[inline]
    pub fn update_anim(&mut self, delta: f32) {
        self.elements.iter_mut()
            .filter(|e| matches!(e.data, UIElementData::Animation{..}))
            .for_each(|e| e.update_anim(delta));
    }
    
    #[inline]
    pub fn add_element(&mut self, mut element: UIElement) -> usize {
        if element.id == 0 {
            element.id = self.next_id;
            self.next_id += 1;
        }
        self.elements.push(element);
        self.next_id-1
    }
    
    #[inline]
    pub fn remove_element(&mut self, id: usize) -> bool {
        if let Some(pos) = self.elements.iter().position(|e| e.id == id) {
            if let Some(focused_pos) = self.focused_element {
                match focused_pos.cmp(&pos) {
                    std::cmp::Ordering::Equal => self.focused_element = None,
                    std::cmp::Ordering::Greater => self.focused_element = Some(focused_pos - 1),
                    _ => (),
                }
            }
            self.elements.remove(pos);
            true
        } else {
            false
        }
    }
    
    #[inline] pub fn get_element(&self, id: usize) -> Option<&UIElement> { self.elements.iter().find(|e| e.id == id) }
    #[inline] pub fn get_element_mut(&mut self, id: usize) -> Option<&mut UIElement> { self.elements.iter_mut().find(|e| e.id == id) }
    #[inline] pub fn get_element_data(&self, id: usize) -> Option<&str> { self.get_element(id).and_then(|e| e.get_data()) }
    
    #[inline]
    pub fn set_element_visibility(&mut self, id: usize, visible: bool) {
        if let Some(element) = self.get_element_mut(id) {
            element.visible = visible;
        }
    }
    
    #[inline]
    pub fn set_element_enabled(&mut self, id: usize, enabled: bool) {
        if let Some(element) = self.get_element_mut(id) {
            element.enabled = enabled;
        }
    }
    
    #[inline]
    pub fn set_element_text(&mut self, id: usize, text: String) {
        if let Some(element) = self.get_element_mut(id) {
            if let Some(text_mut) = element.get_text_mut() {
                *text_mut = text;
            }
        }
    }
    
    #[inline] pub fn clear_elements(&mut self) { self.elements.clear(); self.focused_element = None; self.next_id = 1; }
    
    #[inline]
    pub fn handle_key_input(&mut self, key: Key, shift: bool) {
        match key {
            Key::Backspace => self.handle_backspace(),
            Key::Enter => self.handle_enter(),
            Key::Escape => self.blur_current_element(),
            _ => if let Some(c) = element::key_to_char(key, shift) {
                self.process_text_input(c);
            },
        }
    }
    
    #[inline]
    pub fn handle_backspace(&mut self) {
        if let Some(element) = self.focused_element.and_then(|idx| self.elements.get_mut(idx)) {
            if element.is_input() && element.enabled {
                if let Some(text_mut) = element.get_text_mut() {
                    element::handle_backspace(text_mut);
                }
            }
        }
    }
    
    #[inline] pub fn handle_enter(&mut self) { self.blur_current_element(); }
    #[inline] pub fn blur_current_element(&mut self) { self.focused_element = None; }
    
    #[inline]
    pub fn process_text_input(&mut self, c: char) {
        if let Some(element) = self.focused_element.and_then(|idx| self.elements.get_mut(idx)) {
            if element.is_input() && element.enabled {
                if let Some(text_mut) = element.get_text_mut() {
                    element::process_text_input(text_mut, c);
                }
            }
        }
    }
    
    #[inline] pub fn toggle_visibility(&mut self) { self.visibility = !self.visibility; }
    #[inline] pub fn is_any_element_hovered(&self) -> bool { self.elements.iter().any(|e| e.hovered && e.visible && e.enabled) }
    #[inline] pub fn get_focused_element(&self) -> Option<&UIElement> { self.focused_element.and_then(|idx| self.elements.get(idx)) }
    #[inline] pub fn next_id(&mut self) -> usize { let id = self.next_id; self.next_id += 1; id }
        
    #[inline]
    pub fn handle_click_press(&mut self, norm_x: f32, norm_y: f32) -> bool {
        self.focused_element = None;
        for (_i, element) in self.elements.iter_mut().enumerate().rev() {
            if element.visible && element.enabled && element.contains_point(norm_x, norm_y) {
                if matches!(element.data, 
                    UIElementData::InputField{..} |
                    UIElementData::Checkbox{..} | 
                    UIElementData::Button{..} |
                    UIElementData::MultiStateButton{..} | 
                    UIElementData::Slider{..}
                ) {
                    self.focused_element = Some(element.id);
                    audio::set_sound("click.ogg");
                    return true;
                }
            }
        }
        false
    }
    #[inline]
    pub fn handle_click_rele(&mut self, norm_x: f32, norm_y: f32) -> bool {
        println!("release index: {:?}", self.focused_element);
        if let Some(element) = match self.focused_element {
            Some(id) => self.get_element_mut(id),
            None => None,
        } {
            if element.visible && element.enabled && element.contains_point(norm_x, norm_y) {
                match &element.data {
                    UIElementData::Checkbox{..} => { element.toggle_checked(); },
                    UIElementData::Button{..} => {},
                    UIElementData::MultiStateButton{..} => { element.next_state(); },
                    UIElementData::Slider{..} => {
                        if let Some(value) = element.calc_value(norm_x, norm_y) {
                            element.set_value(value);
                        }
                    },
                    _ => return false,
                }
                element.trigger_callback();
                //self.focused_element = None;
                return true;
            }
        }
        false
    }
    
    pub fn handle_mouse_move(&mut self, window_size: (u32, u32), mouse_pos: &winit::dpi::PhysicalPosition<f64>) {
        let (norm_x, norm_y) = convert_mouse_position(window_size, mouse_pos);
        if let Some(element) = self.focused_element.and_then(|id| self.get_element_mut(id)) {
            if let UIElementData::Slider{..} = element.data {
                if let Some(value) = element.calc_value(norm_x, norm_y) {
                    audio::set_sound("click.ogg");
                    element.set_value(value);
                    element.trigger_callback();
                }
            }
        }
    }
    
    #[inline]
    pub fn handle_ui_hover(&mut self, window_size: (u32, u32), mouse_pos: &winit::dpi::PhysicalPosition<f64>) {
        let (norm_x, norm_y) = convert_mouse_position(window_size, mouse_pos);

        self.elements.iter_mut()
            .for_each(|e| 
                e.update_hover_state(
                    e.contains_point(norm_x, norm_y)
                    )
                );
    }
    
    #[inline]
    pub fn handle_ui_click(&mut self, window_size: (u32, u32), mouse_pos: &winit::dpi::PhysicalPosition<f64>, pressed: bool) -> bool {
        let (x, y) = convert_mouse_position(window_size, mouse_pos);
        if pressed {
            self.handle_click_press(x, y)
        } else {
            self.handle_click_rele(x, y)
        }
    }
    
    #[inline]
    pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
        if self.visibility {
            self.renderer.render(self, render_pass);
        }
    }
}

#[inline]
pub fn get_element_data_dy_id(id: usize) -> String {
    config::get_state()
        .ui_manager()
        .get_element_data(id)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .unwrap_or("Null")
        .to_string()
}

#[inline]
pub fn convert_mouse_position(window_size: (u32, u32), mouse_pos: &winit::dpi::PhysicalPosition<f64>) -> (f32, f32) {
    let (x, y) = (mouse_pos.x as f32, mouse_pos.y as f32);
    let (width, height) = (window_size.0 as f32, window_size.1 as f32);
    ((2.0 * x / width) - 1.0, (2.0 * (height - y) / height) - 1.0)
}