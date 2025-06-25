
use std::cell::RefCell;
use std::fmt;
use std::sync::Arc;

type Callback = Arc<RefCell<dyn FnMut() + 'static>>;

#[derive(Clone)]
pub enum UIElementData {
    Panel,
    Label {
        text: String,
    },
    Button {
        text: String,
        on_click: Option<Callback>,
    },
    InputField {
        text: String,
        placeholder: Option<String>,
    },
    Checkbox {
        label: Option<String>,
        checked: bool,
        on_click: Option<Callback>,
    },
    Image {
        path: String
    },
    Animation {
        frames: Vec<String>,
        current_frame: u32,
        frame_duration: f32,
        elapsed_time: f32,
        looping: bool,
        playing: bool,
        smooth_transition: bool,
        blend_delay: u32,
    },
    Divider,
}

impl Default for UIElementData {
    fn default() -> Self {
        UIElementData::Panel
    }
}
impl fmt::Debug for UIElementData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Panel => f.debug_struct("Panel").finish(),
            Self::Label { text } => f.debug_struct("Label").field("text", text).finish(),
            Self::Button { text, .. } => f.debug_struct("Button").field("text", text).finish(),
            Self::InputField { text, placeholder } => f
                .debug_struct("InputField")
                .field("text", text)
                .field("placeholder", placeholder)
                .finish(),
            Self::Checkbox { label, checked, .. } => f
                .debug_struct("Checkbox")
                .field("label", label)
                .field("checked", checked)
                .finish(),
            Self::Image { path } => f
                .debug_struct("Image")
                .field("path", path)
                .finish(),
            Self::Animation { frames, current_frame, frame_duration, looping, playing, .. } => f
                .debug_struct("Animation")
                .field("frames", frames)
                .field("current_frame", current_frame)
                .field("frame_duration", frame_duration)
                .field("looping", looping)
                .field("playing", playing)
                .finish(),
            Self::Divider => write!(f, "Divider"),
        }
    }
}

pub struct UIElement {
    pub id: usize,
    pub data: UIElementData,
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub color: [f32; 4],
    pub hovered: bool,
    pub z_index: i32,
    pub visible: bool,
    pub border_color: [f32; 4],
    pub border_width: f32,
    pub enabled: bool,
}
impl Default for UIElement {
    fn default() -> Self {
        Self {
            id: 0,
            data: UIElementData::default(),
            position: (0.0,0.0),
            size: (0.0,0.0),
            color: Self::DEFAULT_COLOR,
            hovered: false,
            z_index: 0,
            visible: true,
            border_color: Self::DEFAULT_COLOR,
            border_width: 0.0,
            enabled: true,
        }
    }
}
impl fmt::Debug for UIElement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("UIElement")
            .field("id", &self.id)
            .field("data", &self.data)
            .field("position", &self.position)
            .field("size", &self.size)
            .field("color", &self.color)
            .field("hovered", &self.hovered)
            // Enhanced features
            .field("z_index", &self.z_index)
            .field("visible", &self.visible)
            .field("border_color", &self.border_color)
            .field("border_width", &self.border_width)
            .field("enabled", &self.enabled)
            .finish()
    }
}

impl UIElement {
    // Constants
    const DEFAULT_ALPHA: f32 = 0.9;
    const HOVER_ALPHA: f32 = 0.5;
    const DEFAULT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, Self::DEFAULT_ALPHA];
    
    // Element creation
    pub fn new(id: usize, element_type: UIElementData) -> Self {
        Self {
            id,
            data: element_type,
            color: Self::DEFAULT_COLOR,
            ..Default::default()
        }
    }
    
    pub fn panel(id: usize) -> Self {
        Self::new(id, UIElementData::Panel)
    }
    
    pub fn label(id: usize, text: impl Into<String>) -> Self {
        Self::new(id, UIElementData::Label { text: text.into() })
    }
    
    pub fn button(id: usize, text: impl Into<String>) -> Self {
        Self::new(id, UIElementData::Button {
            text: text.into(),
            on_click: None,
        })
    }
    
    pub fn input(id: usize) -> Self {
        Self::new(id, UIElementData::InputField {
            text: String::new(),
            placeholder: None,
        })
    }
    
    pub fn checkbox(id: usize) -> Self {
        Self::new(id, UIElementData::Checkbox {
            label: None,
            checked: false,
            on_click: None,
        })
    }
    
    pub fn image(id: usize, path: impl Into<String>) -> Self {
        Self::new(id, UIElementData::Image { path: path.into() })
    }
    
    pub fn animation(id: usize, frames: Vec<String>) -> Self {
        Self::new(id, UIElementData::Animation {
            frames,
            current_frame: 0,
            frame_duration: 1.0,
            elapsed_time: 0.0,
            looping: true,
            playing: true,
            smooth_transition: false,
            blend_delay: 20,
        })
    }
    
    pub fn divider(id: usize) -> Self {
        Self::new(id, UIElementData::Divider)
    }
    
    // Builder methods for configuration
    pub fn with_position(mut self, x: f32, y: f32) -> Self {
        self.position = (x, y);
        self
    }
    
    pub fn with_size(mut self, width: f32, height: f32) -> Self {
        self.size = (width, height);
        self
    }
    
    pub fn with_color(mut self, r: f32, g: f32, b: f32) -> Self {
        self.color = [r, g, b, self.color[3]];
        self
    }
    
    pub fn with_alpha(mut self, alpha: f32) -> Self {
        self.color[3] = alpha;
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
    
    pub fn with_border(mut self, (r, g, b, a):(f32,f32,f32,f32), width: f32) -> Self {
        self.border_color = [r, g, b, a];
        self.border_width = width;
        self
    }
    
    // Element-specific configuration
    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        if let Some(text_field) = self.get_text_mut() {
            *text_field = text.into();
        }
        self
    }
    
    pub fn with_placeholder(mut self, placeholder: impl Into<String>) -> Self {
        if let UIElementData::InputField { placeholder: p, .. } = &mut self.data {
            *p = Some(placeholder.into());
        }
        self
    }
    
    pub fn with_checked(mut self, checked: bool) -> Self {
        if let UIElementData::Checkbox { checked: c, .. } = &mut self.data {
            *c = checked;
        }
        self
    }
    
    pub fn with_callback<F: FnMut() + 'static>(mut self, callback: F) -> Self {
        match &mut self.data {
            UIElementData::Button { on_click, .. } => {
                *on_click = Some(Arc::new(RefCell::new(callback)));
            }
            UIElementData::Checkbox { on_click, .. } => {
                *on_click = Some(Arc::new(RefCell::new(callback)));
            }
            _ => {}
        }
        self
    }
    
    // Animation-specific methods
    pub fn with_animation_frames(mut self, frames: Vec<String>) -> Self {
        if let UIElementData::Animation { frames: f, .. } = &mut self.data {
            *f = frames;
        }
        self
    }
    
    pub fn with_animation_duration(mut self, duration: f32) -> Self {
        if let UIElementData::Animation { frame_duration, .. } = &mut self.data {
            *frame_duration = duration;
        }
        self
    }
    
    pub fn with_looping(mut self, looping: bool) -> Self {
        if let UIElementData::Animation { looping: l, .. } = &mut self.data {
            *l = looping;
        }
        self
    }
    
    pub fn with_smooth_transition(mut self, smooth: bool) -> Self {
        if let UIElementData::Animation { smooth_transition, .. } = &mut self.data {
            *smooth_transition = smooth;
        }
        self
    }
    
    pub fn with_blend_delay(mut self, delay: u32) -> Self {
        if let UIElementData::Animation { blend_delay, .. } = &mut self.data {
            *blend_delay = delay;
        }
        self
    }
    
    // Utility methods (unchanged from original)
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
        if matches!(self.data, UIElementData::Button { .. }) {
            self.color[3] = if self.hovered && self.enabled {
                Self::HOVER_ALPHA
            } else if !self.enabled {
                Self::DEFAULT_ALPHA * 0.5
            } else {
                Self::DEFAULT_ALPHA
            };
        }
    }
    
    pub fn toggle_checked(&mut self) {
        if let UIElementData::Checkbox { checked, .. } = &mut self.data {
            *checked = !*checked;
        }
    }
    
    pub fn get_text(&self) -> Option<&str> {
        match &self.data {
            UIElementData::Label { text } => Some(text),
            UIElementData::Button { text, .. } => Some(text),
            UIElementData::InputField { text, .. } => Some(text),
            UIElementData::Checkbox { label, .. } => label.as_deref(),
            _ => None,
        }
    }
    
    pub fn get_text_mut(&mut self) -> Option<&mut String> {
        match &mut self.data {
            UIElementData::Label { text } => Some(text),
            UIElementData::Button { text, .. } => Some(text),
            UIElementData::InputField { text, .. } => Some(text),
            UIElementData::Checkbox { label, .. } => label.as_mut(),
            _ => None,
        }
    }
    
    pub fn is_input(&self) -> bool {
        matches!(self.data, UIElementData::InputField { .. })
    }
    
    pub fn is_checked(&self) -> Option<bool> {
        if let UIElementData::Checkbox { checked, .. } = &self.data {
            Some(*checked)
        } else {
            None
        }
    }
    
    pub fn trigger_click(&mut self) {
        match &mut self.data {
            UIElementData::Button { on_click, .. } => {
                if let Some(callback) = on_click {
                    let cb = callback.clone();
                    cb.borrow_mut()();
                }
            }
            UIElementData::Checkbox { on_click, .. } => {
                if let Some(callback) = on_click {
                    let cb = callback.clone();
                    cb.borrow_mut()();
                }
            }
            _ => {}
        }
    }
    
    // Animation control methods
    pub fn play(&mut self) {
        if let UIElementData::Animation { playing, .. } = &mut self.data {
            *playing = true;
        }
    }
    
    pub fn pause(&mut self) {
        if let UIElementData::Animation { playing, .. } = &mut self.data {
            *playing = false;
        }
    }
    
    pub fn reset(&mut self) {
        if let UIElementData::Animation { current_frame, elapsed_time, .. } = &mut self.data {
            *current_frame = 0;
            *elapsed_time = 0.0;
        }
    }
    
    pub fn update_anim(&mut self, delta_time: f32) {
        if let UIElementData::Animation {
            frames,
            current_frame,
            frame_duration,
            elapsed_time,
            looping,
            playing,
            ..
        } = &mut self.data
        {
            if !*playing || frames.is_empty() {
                return;
            }

            *elapsed_time += delta_time;

            while *elapsed_time >= *frame_duration {
                *elapsed_time -= *frame_duration;
                *current_frame += 1;

                if *current_frame >= frames.len() as u32 {
                    if *looping {
                        *current_frame = 0;
                    } else {
                        *current_frame = frames.len() as u32 - 1;
                        *playing = false;
                        break;
                    }
                }
            }
        }
    }
}

// Input validation and processing (unchanged)
#[inline]
pub fn process_text_input(text: &mut String, c: char) -> bool {
    if text.len() >= 256 || c.is_control() {
        return false;
    }
    text.push(c);
    true
}
#[inline]
pub fn handle_backspace(text: &mut String) -> bool {
    if !text.is_empty() {
        text.pop();
        true
    } else {
        false
    }
}


use winit::keyboard::KeyCode as Key;

// Input handling utilities (unchanged)
#[inline]
pub fn key_to_char(key: Key, shift: bool) -> Option<char> {
    match key {
            // Alphabet
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
            // Numbers
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
            // Symbols
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
            // Numpad keys (with NumLock on)
        Key::Numpad0 => Some('0'),
        Key::Numpad1 => Some('1'),
        Key::Numpad2 => Some('2'),
        Key::Numpad3 => Some('3'),
        Key::Numpad4 => Some('4'),
        Key::Numpad5 => Some('5'),
        Key::Numpad6 => Some('6'),
        Key::Numpad7 => Some('7'),
        Key::Numpad8 => Some('8'),
        Key::Numpad9 => Some('9'),
        Key::NumpadAdd => Some('+'),
        Key::NumpadSubtract => Some('-'),
        Key::NumpadMultiply => Some('*'),
        Key::NumpadDivide => Some('/'),
        Key::NumpadDecimal => Some('.'),
            // Fallback - undefined
        _ => None,
    }
}
