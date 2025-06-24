use std::cell::RefCell;
use std::fmt;
use std::sync::Arc;
use winit::keyboard::KeyCode as Key;

// Constants for common values
impl UIElement {
    const DEFAULT_ALPHA: f32 = 0.9;
    const HOVER_ALPHA: f32 = 0.5;
    const DEFAULT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, Self::DEFAULT_ALPHA];
    const DEFAULT_SIZE: (f32, f32) = (0.0, 0.0);
    const DEFAULT_POSITION: (f32, f32) = (0.0, 0.0);
    const MAX_INPUT_LENGTH: usize = 255; // was only used when text was rendered by char now could be removed
}

type Callback = Arc<RefCell<dyn FnMut()>>;


#[derive(Clone)]
pub enum UIElementData {
    Panel,
    Label {
        text: String,
    },
    Button {
        text: String,
        on_click: Callback,
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
        // Store raw path (only name and extension)
        path: String
    },
    Animation {
        frames: Vec<String>,          // Paths to each frame
        current_frame: u32,           // Current frame index
        frame_duration: f32,          // Duration per frame in seconds
        elapsed_time: f32,            // Time since last frame change
        looping: bool,                // Whether animation loops
        playing: bool,                // Whether animation is currently playing
        smooth_transition: bool,      // When making the transition alpha changing
        blend_delay: u32,             // If sm_tr -> how much% of the fr_dur should the frame be Not affected by the blending
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
    // Enhanced features
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
            position: Self::DEFAULT_POSITION,
            size: Self::DEFAULT_SIZE,
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
    pub fn new(
        id: usize,
        data: UIElementData,
        position: (f32, f32),
        size: (f32, f32),
        color: [f32; 3],
    ) -> Self {
        Self {
            id,
            data,
            position,
            size,
            color: [color[0], color[1], color[2], Self::DEFAULT_ALPHA],
            ..Self::default()
        }
    }

    pub fn new_button(
        id: usize,
        position: (f32, f32),
        size: (f32, f32),
        color: [f32; 3],
        text: String,
        on_click: impl FnMut() + 'static,
    ) -> Self {
        Self::new(
            id,
            UIElementData::Button {
                text,
                on_click: Arc::new(RefCell::new(on_click)),
            },
            position,
            size,
            color,
        )
    }

    pub fn new_label(
        id: usize,
        position: (f32, f32),
        size: (f32, f32),
        color: [f32; 3],
        text: String,
    ) -> Self {
        Self::new(id, UIElementData::Label { text }, position, size, color)
    }

    pub fn new_input(
        id: usize,
        position: (f32, f32),
        size: (f32, f32),
        color: [f32; 3],
        placeholder: Option<String>,
    ) -> Self {
        // Initialize text with placeholder if it exists, otherwise empty string
        let text = placeholder.clone().unwrap_or_default();

        Self::new(
            id,
            UIElementData::InputField {
                text,        // Use the initialized text
                placeholder, // Keep the original placeholder
            },
            position,
            size,
            color,
        )
    }

    pub fn new_checkbox(
        id: usize,
        position: (f32, f32),
        size: (f32, f32),
        color: [f32; 3],
        label: Option<String>,
        checked: bool,
        on_click: Option<impl FnMut() + 'static>,
    ) -> Self {
        Self::new(
            id,
            UIElementData::Checkbox {
                label,
                checked,
                on_click: on_click
                    .map(|f| Arc::new(RefCell::new(f)))
                    .map(|arc| arc as Callback),
            },
            position,
            size,
            color,
        )
    }

    pub fn new_panel(id: usize, position: (f32, f32), size: (f32, f32), color: [f32; 3]) -> Self {
        Self::new(id, UIElementData::Panel, position, size, color)
    }

    pub fn new_divider(id: usize, position: (f32, f32), size: (f32, f32), color: [f32; 3]) -> Self {
        Self::new(id, UIElementData::Divider, position, size, color)
    }

    pub fn new_image(
        id: usize,
        position: (f32, f32),
        size: (f32, f32),
        color: [f32; 3],
        path: String,
    ) -> Self {
        Self::new(
            id,
            UIElementData::Image { path },
            position,
            size,
            color
        )
    }


    pub fn new_animation(
        id: usize,
        position: (f32, f32),
        size: (f32, f32),
        color: [f32; 3],
        frames: Vec<String>,
    ) -> Self {
        Self::new(
            id,
            UIElementData::Animation {
                frames,
                current_frame: 0,
                frame_duration: 1f32,
                elapsed_time: 0.0,
                looping: true,
                playing: true,
                smooth_transition: false,
                blend_delay: 20,
            },
            position,
            size,
            color,
        )
    }

    pub fn play_anim(mut self) -> Self {
        if let UIElementData::Animation {
            frames, current_frame, looping, playing, ..
        } = &mut self.data {
            if !*playing && !*looping && *current_frame == frames.len() as u32 -1 {
                *current_frame = 0;
            }
            *playing = !*playing;
        }
        self
    }
    pub fn add_anim_frame(mut self, frame: String) -> Self {
        if let UIElementData::Animation { frames, .. } = &mut self.data {
            frames.push(frame); // Just push directly, no assignment needed
        }
        self
    }
    pub fn anim_smooth(mut self, smooth: bool) -> Self {
        if let UIElementData::Animation { smooth_transition, .. } = &mut self.data {
            *smooth_transition = smooth;
        }
        self
    }
    pub fn with_anim_blend_delay(mut self, delay: u32) -> Self {
        if let UIElementData::Animation { blend_delay, .. } = &mut self.data {
            *blend_delay = delay;
        }
        self
    }
    pub fn loop_anim(mut self) -> Self {
        if let UIElementData::Animation { looping, .. } = &mut self.data {
            *looping = !*looping;
        }
        self
    }
    pub fn set_anim_duration(mut self, delay: f32) -> Self {
        if let UIElementData::Animation { frame_duration, .. } = &mut self.data {
            *frame_duration = delay;
        }
        self
    }
    pub fn set_anim_index(mut self, index: u32) -> Self {
        if let UIElementData::Animation {
            current_frame,
            elapsed_time,
            ..
        } = &mut self.data
        {
            *current_frame = index;
            *elapsed_time = 0.0;
        }
        self
    }
    pub fn reset_anim(&mut self) {
        if let UIElementData::Animation {
            current_frame,
            elapsed_time,
            ..
        } = &mut self.data
        {
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

    pub fn set_border(mut self, border_color: [f32; 4], border_width: f32) -> Self {
        self.border_color = border_color;
        self.border_width = border_width;
        self
    }
    pub fn set_color(mut self, color: [f32; 4]) -> Self {
        // should not use this, cus' alpha
        self.color = color;
        self
    }
    pub fn set_z_index(mut self, z_index: i32) -> Self {
        self.z_index = z_index;
        self
    }
    pub fn set_visibility(mut self, visible: bool) -> Self {
        self.visible = visible;
        self
    }
    pub fn set_enabled(mut self, enabled: bool) -> Self {
        // currently this is basically not processed ... so yeah ...
        self.enabled = enabled;
        self
    }
    pub fn set_pos(mut self, x : f32, y : f32) -> Self {
        self.position = (x, y);
        self
    }
    pub fn set_size(mut self, w : f32, h : f32) -> Self {
        self.size = (w, h);
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
                if let Ok(mut callback) = on_click.try_borrow_mut() {
                    callback();
                }
            }
            UIElementData::Checkbox { on_click, .. } => {
                if let Some(callback) = on_click {
                    if let Ok(mut cb) = callback.try_borrow_mut() {
                        cb();
                    }
                }
            }
            _ => {}
        }
    }
    #[allow(dead_code)] // maybe better than try_borrow
    pub fn trigger_click_clone(&mut self) {
        match &mut self.data {
            UIElementData::Button { on_click, .. } => {
                let callback = on_click.clone();
                callback.borrow_mut()();
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
}

// Input validation and processing (unchanged)
pub fn process_text_input(text: &mut String, c: char) -> bool {
    if text.len() >= UIElement::MAX_INPUT_LENGTH || c.is_control() {
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

// Input handling utilities (unchanged)
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
