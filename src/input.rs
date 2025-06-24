
use winit::keyboard::ModifiersState;

pub struct InputSystem {
    pub previous_mouse: Option<winit::dpi::PhysicalPosition<f64>>,
    pub mouse_button_state: MouseButtonState,
    pub modifiers: ModifiersState,
    pub mouse_captured: bool,
}
impl Default for InputSystem {
    #[inline]
    fn default() -> Self {
        Self {
            previous_mouse: None,
            mouse_button_state: MouseButtonState::default(),
            modifiers: ModifiersState::empty(),
            mouse_captured: false,
        }
    }
}

#[derive(Default)]
pub struct MouseButtonState {
    pub left: bool,
    pub right: bool,
}