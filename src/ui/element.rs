
use std::{cell::RefCell, fmt, sync::Arc };

type Callback = Arc<RefCell<dyn FnMut() + 'static>>;

pub trait Textlike: Into<String> {}
impl<T> Textlike for T where T: Into<String> {}


#[derive(Clone)]
pub enum UIElementData {
    Panel,
    Divider,
    Label { text: String, text_color: Option<[u8; 4]> },
    Button { text: String, text_color: Option<[u8; 4]>, on_click: Option<Callback> },
    MultiStateButton { states: Vec<String>, current_state: usize, on_click: Option<Callback>, },
    InputField { text: String, text_color: Option<[u8; 4]>, placeholder: Option<String> },
    Checkbox { label: Option<String>, text_color: Option<[u8; 4]>, checked: bool, on_click: Option<Callback> },
    Image { path: String },
    Animation {
        frames: Vec<String>, current_frame: u32, frame_duration: f32, elapsed_time: f32,
        looping: bool, playing: bool, smooth_transition: bool, blend_delay: u32,
    },
    Slider {
        min_value: f32, max_value: f32, current_value: f32,
        step: Option<f32>, on_change: Option<Callback>, //vertical: bool,
    },
}

impl Default for UIElementData {
    fn default() -> Self { UIElementData::Panel }
}

impl fmt::Debug for UIElementData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Panel => write!(f, "Panel"),
            Self::Label { text, .. } => f.debug_struct("Label").field("text", text).finish(),
            Self::Button { text, .. } => f.debug_struct("Button").field("text", text).finish(),
            Self::InputField { text, placeholder, .. } => f
                .debug_struct("InputField")
                .field("text", text)
                .field("placeholder", placeholder)
                .finish(),
            Self::Checkbox { label, checked, .. } => f
                .debug_struct("Checkbox")
                .field("label", label)
                .field("checked", checked)
                .finish(),
            Self::Image { path } => f.debug_struct("Image").field("path", path).finish(),
            Self::Animation { frames, current_frame, frame_duration, looping, playing, .. } => f
                .debug_struct("Animation")
                .field("frames", frames)
                .field("current_frame", current_frame)
                .field("frame_duration", frame_duration)
                .field("looping", looping)
                .field("playing", playing)
                .finish(),
            Self::Divider => write!(f, "Divider"),
            Self::MultiStateButton { states, current_state, .. } => f
                .debug_struct("MultiStateButton")
                .field("states: ", &states.join("|"))
                .field("current_state", current_state)
                .finish(),
            Self::Slider { min_value, max_value, current_value, .. } => f
                .debug_struct("Slider")
                .field("min_value", min_value)
                .field("max_value", max_value)
                .field("current_value", current_value)
                .finish(),
        }
    }
}

#[derive(Clone)]
pub struct UIElement {
    pub id: usize,
    pub data: UIElementData,
    pub position: (f32, f32),
    pub size: (f32, f32),
    pub color: [u8; 4],
    pub hovered: bool,
    pub z_index: i32,
    pub visible: bool,
    pub border_color: [u8; 4],
    pub border_width: f32,
    pub enabled: bool,
}

impl Default for UIElement {
    fn default() -> Self {
        Self {
            id: 0,
            data: UIElementData::default(),
            position: (0.0, 0.0),
            size: (0.0, 0.0),
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
            .field("visible", &self.visible)
            .field("enabled", &self.enabled)
            .finish()
    }
}

impl UIElement {
    const DEFAULT_ALPHA: u8 = 255;
    const HOVER_ALPHA: u8 = 124;
    const DEFAULT_COLOR: [u8; 4] = [255, 255, 255, Self::DEFAULT_ALPHA];
    

    // Element creation
    pub fn new(id: usize, element_type: UIElementData) -> Self {
        Self { id, data: element_type, color: Self::DEFAULT_COLOR, ..Default::default() }
    }
    pub fn panel(id: usize) -> Self { Self::new(id, UIElementData::Panel) }
    pub fn label<T: Textlike>(id: usize, text: T) -> Self {
        Self::new(id, UIElementData::Label { text: text.into(), text_color: None })
    }
    pub fn button<T: Textlike>(id: usize, text: T) -> Self {
        Self::new(id, UIElementData::Button { text: text.into(), text_color: None, on_click: None })
    }
    pub fn input(id: usize) -> Self {
        Self::new(id, UIElementData::InputField { text: String::new(), text_color: None, placeholder: None })
    }
    pub fn checkbox(id: usize) -> Self {
        Self::new(id, UIElementData::Checkbox { label: None, text_color: None, checked: false, on_click: None })
    }
    pub fn image<T: Textlike>(id: usize, path: T) -> Self {
        Self::new(id, UIElementData::Image { path: path.into() })
    }
    pub fn animation<T: Textlike>(id: usize, frames: Vec<T>) -> Self {
        Self::new(id, UIElementData::Animation {
            frames : frames.into_iter().map(Into::into).collect(),
            current_frame: 0, frame_duration: 1.0, elapsed_time: 0.0,
            looping: true, playing: true, smooth_transition: false, blend_delay: 20,
        })
    }
    pub fn divider(id: usize) -> Self { Self::new(id, UIElementData::Divider) }
    pub fn multi_state_button<T: Textlike>(id: usize, states: Vec<T>) -> Self {
        Self::new(id, UIElementData::MultiStateButton {
            states: states.into_iter().map(Into::into).collect(),
            current_state: 0,on_click: None,
        })
    }
    pub fn slider(id: usize, min_value: f32, max_value: f32) -> Self {
        Self::new(id, UIElementData::Slider {
            min_value, max_value, //vertical: false,
            current_value: min_value, 
            step: None,on_change: None, 
        })
    }
    
    
    // Builder methods
    pub fn with_position(mut self, x: f32, y: f32) -> Self { self.position = (x, y); self }
    pub fn with_size(mut self, width: f32, height: f32) -> Self { self.size = (width, height); self }
    pub fn with_color(mut self, r: u8, g: u8, b: u8) -> Self { self.color = [r, g, b, self.color[3]]; self }
    pub fn with_alpha(mut self, alpha: u8) -> Self { self.color[3] = alpha; self }
    pub fn with_z_index(mut self, z_index: i32) -> Self { self.z_index = z_index; self }
    pub fn with_visibility(mut self, visible: bool) -> Self { self.visible = visible; self }
    pub fn with_enabled(mut self, enabled: bool) -> Self { self.enabled = enabled; self }
    pub fn with_border(mut self, color: (u8, u8, u8, u8), width: f32) -> Self {
        self.border_color = [color.0, color.1, color.2, color.3];
        self.border_width = width;
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
            UIElementData::MultiStateButton { on_click, .. } => {
                *on_click = Some(Arc::new(RefCell::new(callback)));
            }
            UIElementData::Slider { on_change, .. } => {
                *on_change = Some(Arc::new(RefCell::new(callback)));
            }
            _ => {}
        }
        self
    }
            
    // Utility methods
    pub fn get_bounds(&self) -> (f32, f32, f32, f32) {
        let (x, y) = self.position;
        let (w, h) = self.size;
        (x, y, x + w, y + h)
    }
    pub fn contains_point(&self, x: f32, y: f32) -> bool {
        if !self.visible || !self.enabled { return false; }
        let (min_x, min_y, max_x, max_y) = self.get_bounds();
        x >= min_x && x <= max_x && y >= min_y && y <= max_y
    }
    pub fn is_input(&self) -> bool { matches!(self.data, UIElementData::InputField { .. }) }
    pub fn update_hover_state(&mut self, is_hovered: bool) {
        self.hovered = is_hovered && self.enabled;
        if matches!(self.data, UIElementData::Button { .. }) {
            self.color[3] = if self.hovered && self.enabled {
                Self::HOVER_ALPHA
            } else if !self.enabled {
                Self::DEFAULT_ALPHA / 2
            } else {
                Self::DEFAULT_ALPHA
            };
        }
    }
    pub fn get_text(&self) -> Option<&str> {
        match &self.data {
            UIElementData::Label { text, .. } |
            UIElementData::Button { text, .. } |
            UIElementData::InputField { text, .. } => Some(text),
            UIElementData::Checkbox { label, .. } => label.as_deref(),
            _ => None,
        }
    }
    pub fn get_text_mut(&mut self) -> Option<&mut String> {
        match &mut self.data {
            UIElementData::Label { text, .. } |
            UIElementData::Button { text, .. } |
            UIElementData::InputField { text, .. } => Some(text),
            UIElementData::Checkbox { label, .. } => label.as_mut(),
            _ => None,
        }
    }    
    pub fn trigger_callback(&mut self) {
        let callback = match &mut self.data {
            UIElementData::Button { on_click, .. } => on_click.clone(),
            UIElementData::Checkbox { on_click, .. } => on_click.clone(),
            UIElementData::Slider { on_change, .. } => on_change.clone(),
            UIElementData::MultiStateButton { on_click, .. } => {
                // Call next_state before cloning the callback
                let cb = on_click.clone();
                self.next_state();
                cb
            }
            _ => None,
        };
        if let Some(cb) = callback {
            cb.borrow_mut()();
        }
    }
    
}


impl UIElement {

    // Text-related methods
    pub fn with_text<T: Textlike>(mut self, text: T) -> Self {
        if let Some(text_field) = self.get_text_mut() { *text_field = text.into(); }
        self
    }
    fn set_text_color(&mut self, r: u8, g: u8, b: u8, a: u8) {
        match &mut self.data {
            UIElementData::Label { text_color, .. } |
            UIElementData::Button { text_color, .. } |
            UIElementData::InputField { text_color, .. } |
            UIElementData::Checkbox { text_color, .. } => *text_color = Some([r, g, b, a]),
            _ => {}
        }
    }
    pub fn with_text_color(mut self, r: u8, g: u8, b: u8) -> Self {
        self.set_text_color(r, g, b, 255);
        self
    }
    pub fn with_text_visibility(mut self, a: u8) -> Self {
        let color = self.get_text_color();
        self.set_text_color(color[0], color[1], color[2], a);
        self
    }
    pub fn get_text_color(&self) -> [u8; 4] {
        let text_color = match &self.data {
            UIElementData::Label { text_color, .. } |
            UIElementData::Button { text_color, .. } |
            UIElementData::InputField { text_color, .. } |
            UIElementData::Checkbox { text_color, .. } => *text_color,
            _ => None,
        };
        text_color.unwrap_or(self.color)
    }
    pub fn with_placeholder<T: Textlike>(mut self, placeholder: T) -> Self {
        if let UIElementData::InputField { placeholder: p, .. } = &mut self.data {
            *p = Some(placeholder.into());
        }
        self
    }

    // MultiStateButton-related methods
    pub fn next_state(&mut self) {
        if let UIElementData::MultiStateButton { states, current_state, .. } = &mut self.data {
            *current_state = (*current_state + 1) % states.len();
        }
    }
    pub fn get_current_state(&self) -> Option<usize> {
        if let UIElementData::MultiStateButton { current_state, .. } = &self.data {
            Some(*current_state)
        } else {
            None
        }
    }
    pub fn with_step(mut self, step: f32) -> Self {
        if let UIElementData::Slider { step: s, .. } = &mut self.data {
            *s = Some(step);
        }
        self
    }
    /*pub fn with_vertical(mut self, vertical: bool) -> Self {
        if let UIElementData::Slider { vertical: v, .. } = &mut self.data {
            *v = vertical;
        }
        self
    }*/
    pub fn with_value(mut self, value: f32) -> Self {
        if let UIElementData::Slider { min_value, max_value, current_value, .. } = &mut self.data {
            *current_value = value.clamp(*min_value, *max_value);
        }
        self
    }
    pub fn get_value(&self) -> Option<f32> {
        if let UIElementData::Slider { current_value, .. } = &self.data {
            Some(*current_value)
        } else {
            None
        }
    }
    
    // Checkbox-related methods
    pub fn with_checked(mut self, checked: bool) -> Self {
        if let UIElementData::Checkbox { checked: c, .. } = &mut self.data { *c = checked; }
        self
    }
    pub fn toggle_checked(&mut self) {
        if let UIElementData::Checkbox { checked, .. } = &mut self.data { *checked = !*checked; }
    }
    pub fn is_checked(&self) -> Option<bool> {
        if let UIElementData::Checkbox { checked, .. } = &self.data { Some(*checked) } else { None }
    }

    // Animation-related methods
    pub fn with_animation_frames<T: Textlike>(mut self, frames_new: Vec<T>) -> Self {
        if let UIElementData::Animation { frames, .. } = &mut self.data {
            *frames = frames_new.into_iter().map(Into::into).collect();
        }
        self
    }
    pub fn with_animation_duration(mut self, duration: f32) -> Self {
        if let UIElementData::Animation { frame_duration, .. } = &mut self.data { *frame_duration = duration; }
        self
    }
    pub fn with_looping(mut self, looping: bool) -> Self {
        if let UIElementData::Animation { looping: l, .. } = &mut self.data { *l = looping; }
        self
    }
    pub fn with_smooth_transition(mut self, smooth: bool) -> Self {
        if let UIElementData::Animation { smooth_transition, .. } = &mut self.data { *smooth_transition = smooth; }
        self
    }
    pub fn with_blend_delay(mut self, delay: u32) -> Self {
        if let UIElementData::Animation { blend_delay, .. } = &mut self.data { *blend_delay = delay; }
        self
    }
    pub fn play(&mut self) {
        if let UIElementData::Animation { playing, .. } = &mut self.data { *playing = true; }
    }
    pub fn pause(&mut self) {
        if let UIElementData::Animation { playing, .. } = &mut self.data { *playing = false; }
    }
    pub fn reset(&mut self) {
        if let UIElementData::Animation { current_frame, elapsed_time, .. } = &mut self.data {
            *current_frame = 0; *elapsed_time = 0.0;
        }
    }
    pub fn update_anim(&mut self, delta_time: f32) {
        if let UIElementData::Animation {
            frames, current_frame, frame_duration, elapsed_time, looping, playing, ..
        } = &mut self.data {
            if !*playing || frames.is_empty() { return; }
            
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
