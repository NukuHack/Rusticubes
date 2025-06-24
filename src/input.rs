use winit::{event::ElementState, keyboard::KeyCode as Key};

pub struct InputSystem {
    pub previous_mouse: Option<winit::dpi::PhysicalPosition<f64>>,
    pub mouse_button_state: MouseButtonState,
    pub modifier_keys: ModifierKeys,
    pub mouse_captured: bool,
}
impl Default for InputSystem {
    #[inline]
    fn default() -> Self {
        Self {
            previous_mouse: None,
            mouse_button_state: MouseButtonState::default(),
            modifier_keys: ModifierKeys::default(),
            mouse_captured: false,
        }
    }
}

#[derive(Default)]
pub struct MouseButtonState {
    pub left: bool,
    pub right: bool,
}
#[derive(Default)]
pub struct ModifierKeys {
    pub sift: bool,
    pub alt: bool,
    pub ctr: bool,
    pub altgr: bool,
    pub caps: bool,
}
impl ModifierKeys {
    #[inline]
    pub fn set_modify_kes(&mut self, key: winit::keyboard::KeyCode, state: ElementState) {
        if state == ElementState::Pressed {
            match key {
                Key::AltLeft => {
                    self.alt = true;
                }
                Key::ShiftLeft | Key::ShiftRight => {
                    self.sift = true;
                }
                Key::AltRight => {
                    self.altgr = true;
                }
                Key::CapsLock => {
                    self.caps = true;
                }
                Key::ControlLeft | Key::ControlRight => {
                    self.ctr = true;
                }
                _ => {}
            }
        } else {
            match key {
                Key::AltLeft => {
                    self.alt = false;
                }
                Key::ShiftLeft | Key::ShiftRight => {
                    self.sift = false;
                }
                Key::AltRight => {
                    self.altgr = false;
                }
                Key::CapsLock => {
                    self.caps = false;
                }
                Key::ControlLeft | Key::ControlRight => {
                    self.ctr = false;
                }
                _ => {}
            }
        }
    }
}
