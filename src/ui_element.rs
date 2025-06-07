use winit::keyboard::KeyCode as Key;

// Constants for common values
const DEFAULT_ALPHA: f32 = 0.9;
const HOVER_ALPHA: f32 = 0.5;
const MAX_INPUT_LENGTH: usize = 120;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UIElementType {
    Panel,
    Label,
    Button,
    InputField,
    Checkbox,
    Image,
    Divider,
}

#[derive(Default)]
pub struct UIElement {
    pub id: usize,
    pub element_type: UIElementType,
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub color: [f32; 4],
    pub text: Option<String>,
    pub hovered: bool,
    pub is_input: bool,
    pub on_click: Option<Box<dyn FnMut()>>,

    // Enhanced features
    pub z_index: i32,
    pub visible: bool,
    pub border_color: [f32; 4],
    pub border_width: f32,
    pub enabled: bool,
    pub checked: bool, // For checkboxes
}

impl Default for UIElementType {
    fn default() -> Self {
        UIElementType::Panel
    }
}

impl UIElement {
    pub const DEFAULT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, DEFAULT_ALPHA];
    pub const DEFAULT_SIZE: (f32, f32) = (0.2, 0.2);
    pub const DEFAULT_BORDER_COLOR: [f32; 4] = [0.5, 0.5, 0.5, 1.0];

    pub fn new(
        id: usize,
        element_type: UIElementType,
        position: (f32, f32),
        size: (f32, f32),
        color: [f32; 3],
        text: Option<String>,
        on_click: Option<Box<dyn FnMut()>>,
    ) -> Self {
        Self {
            id,
            element_type,
            position,
            size,
            color: [color[0], color[1], color[2], DEFAULT_ALPHA],
            text,
            on_click,
            visible: true,
            enabled: true,
            border_color: Self::DEFAULT_BORDER_COLOR,
            border_width: 0.0,
            z_index: 0,
            ..Default::default()
        }
    }

    pub fn new_button(
        id: usize,
        position: (f32, f32),
        size: (f32, f32),
        color: [f32; 3],
        text: String,
        on_click: Box<dyn FnMut()>,
    ) -> Self {
        Self::new(
            id,
            UIElementType::Button,
            position,
            size,
            color,
            Some(text),
            Some(on_click),
        )
    }

    pub fn new_label(
        id: usize,
        position: (f32, f32),
        size: (f32, f32),
        color: [f32; 3],
        text: String,
    ) -> Self {
        Self::new(
            id,
            UIElementType::Label,
            position,
            size,
            color,
            Some(text),
            None,
        )
    }

    pub fn new_input(
        id: usize,
        position: (f32, f32),
        size: (f32, f32),
        color: [f32; 3],
        placeholder: Option<String>,
    ) -> Self {
        let mut element = Self::new(
            id,
            UIElementType::InputField,
            position,
            size,
            color,
            placeholder,
            None,
        );
        element.is_input = true;
        element.border_width = 0.002;
        element
    }

    pub fn new_checkbox(
        id: usize,
        position: (f32, f32),
        size: (f32, f32),
        label: Option<String>,
        checked: bool,
        on_click: Option<Box<dyn FnMut()>>,
    ) -> Self {
        let mut element = Self::new(
            id,
            UIElementType::Checkbox,
            position,
            size,
            [0.9, 0.9, 0.9],
            label,
            on_click,
        );
        element.checked = checked;
        element.border_width = 0.001;
        element.border_color = [0.3, 0.3, 0.3, 1.0];
        element
    }

    pub fn new_panel(id: usize, position: (f32, f32), size: (f32, f32), color: [f32; 3]) -> Self {
        Self::new(id, UIElementType::Panel, position, size, color, None, None)
    }

    pub fn new_divider(id: usize, position: (f32, f32), size: (f32, f32), color: [f32; 3]) -> Self {
        Self::new(
            id,
            UIElementType::Divider,
            position,
            size,
            color,
            None,
            None,
        )
    }

    // Builder pattern methods for enhanced customization
    pub fn with_border(mut self, color: [f32; 4], width: f32) -> Self {
        self.border_color = color;
        self.border_width = width;
        self
    }

    pub fn with_z_index(mut self, z_index: i32) -> Self {
        self.z_index = z_index;
        self
    }

    pub fn with_visibility(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn get_bounds(&self) -> (f32, f32, f32, f32) {
        let (x, y) = self.position;
        let (w, h) = self.size;
        (x, y, x + w, y + h)
    }

    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        if !self.visible || !self.enabled {
            return false;
        }
        let (min_x, min_y, max_x, max_y) = self.get_bounds();
        x >= min_x && x <= max_x && y >= min_y && y <= max_y
    }

    pub fn update_hover_state(&mut self, is_hovered: bool) {
        self.hovered = is_hovered && self.enabled;
        if self.element_type == UIElementType::Button {
            self.color[3] = if self.hovered && self.enabled {
                HOVER_ALPHA
            } else if !self.enabled {
                DEFAULT_ALPHA * 0.5
            } else {
                DEFAULT_ALPHA
            };
        }
    }

    pub fn toggle_checked(&mut self) {
        if self.element_type == UIElementType::Checkbox {
            self.checked = !self.checked;
        }
    }
}

impl std::fmt::Debug for UIElement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UIElement")
            .field("id", &self.id)
            .field("element_type", &self.element_type)
            .field("position", &self.position)
            .field("size", &self.size)
            .field("color", &self.color)
            .field("text", &self.text)
            .field("hovered", &self.hovered)
            .field("visible", &self.visible)
            .field("enabled", &self.enabled)
            .field("z_index", &self.z_index)
            .field("has_on_click", &self.on_click.is_some())
            .field("border_width", &self.border_width)
            .field("checked", &self.checked)
            .finish()
    }
}

// Input handling utilities
pub fn key_to_char(key: Key, shift: bool) -> Option<char> {
    match key {
        Key::KeyA => Some(if shift { 'A' } else { 'a' }),
        Key::KeyB => Some(if shift { 'B' } else { 'b' }),
        Key::KeyC => Some(if shift { 'C' } else { 'c' }),
        Key::KeyD => Some(if shift { 'D' } else { 'd' }),
        Key::KeyE => Some(if shift { 'E' } else { 'e' }),
        Key::KeyF => Some(if shift { 'F' } else { 'f' }),
        Key::KeyG => Some(if shift { 'G' } else { 'g' }),
        Key::KeyH => Some(if shift { 'H' } else { 'h' }),
        Key::KeyI => Some(if shift { 'I' } else { 'i' }),
        Key::KeyJ => Some(if shift { 'J' } else { 'j' }),
        Key::KeyK => Some(if shift { 'K' } else { 'k' }),
        Key::KeyL => Some(if shift { 'L' } else { 'l' }),
        Key::KeyM => Some(if shift { 'M' } else { 'm' }),
        Key::KeyN => Some(if shift { 'N' } else { 'n' }),
        Key::KeyO => Some(if shift { 'O' } else { 'o' }),
        Key::KeyP => Some(if shift { 'P' } else { 'p' }),
        Key::KeyQ => Some(if shift { 'Q' } else { 'q' }),
        Key::KeyR => Some(if shift { 'R' } else { 'r' }),
        Key::KeyS => Some(if shift { 'S' } else { 's' }),
        Key::KeyT => Some(if shift { 'T' } else { 't' }),
        Key::KeyU => Some(if shift { 'U' } else { 'u' }),
        Key::KeyV => Some(if shift { 'V' } else { 'v' }),
        Key::KeyW => Some(if shift { 'W' } else { 'w' }),
        Key::KeyX => Some(if shift { 'X' } else { 'x' }),
        Key::KeyY => Some(if shift { 'Y' } else { 'y' }),
        Key::KeyZ => Some(if shift { 'Z' } else { 'z' }),
        Key::Digit0 => Some(if shift { ')' } else { '0' }),
        Key::Digit1 => Some(if shift { '!' } else { '1' }),
        Key::Digit2 => Some(if shift { '@' } else { '2' }),
        Key::Digit3 => Some(if shift { '#' } else { '3' }),
        Key::Digit4 => Some(if shift { '$' } else { '4' }),
        Key::Digit5 => Some(if shift { '%' } else { '5' }),
        Key::Digit6 => Some(if shift { '^' } else { '6' }),
        Key::Digit7 => Some(if shift { '&' } else { '7' }),
        Key::Digit8 => Some(if shift { '*' } else { '8' }),
        Key::Digit9 => Some(if shift { '(' } else { '9' }),
        Key::Space => Some(' '),
        Key::Minus => Some(if shift { '_' } else { '-' }),
        Key::Equal => Some(if shift { '+' } else { '=' }),
        Key::BracketLeft => Some(if shift { '{' } else { '[' }),
        Key::BracketRight => Some(if shift { '}' } else { ']' }),
        Key::Backslash => Some(if shift { '|' } else { '\\' }),
        Key::Semicolon => Some(if shift { ':' } else { ';' }),
        Key::Quote => Some(if shift { '"' } else { '\'' }),
        Key::Comma => Some(if shift { '<' } else { ',' }),
        Key::Period => Some(if shift { '>' } else { '.' }),
        Key::Slash => Some(if shift { '?' } else { '/' }),
        _ => None,
    }
}

// Input validation and processing
pub fn process_text_input(text: &mut String, c: char) -> bool {
    if text.len() >= MAX_INPUT_LENGTH || c.is_control() {
        return false;
    }
    text.push(c);
    true
}

pub fn handle_backspace(text: &mut String) -> bool {
    if !text.is_empty() {
        text.pop();
        true
    } else {
        false
    }
}
