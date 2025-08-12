
use winit::keyboard::ModifiersState;

pub struct InputSystem {
	pub previous_mouse: Option<winit::dpi::PhysicalPosition<f64>>,
	pub mouse_button_state: MouseButtonState,
	pub modifiers: ModifiersState,
	mouse_captured: bool,
}
impl InputSystem {
	#[inline] pub const fn default() -> Self {
		Self {
			previous_mouse: None,
			mouse_button_state: MouseButtonState::default(),
			modifiers: ModifiersState::empty(),
			mouse_captured: false,
		}
	}

	#[inline] pub const fn set_mouse_captured(&mut self, is_captured:bool) {
		self.mouse_captured = is_captured;
	}
	#[inline] pub const fn mouse_captured(&self) -> bool {
		self.mouse_captured
	}
	#[inline] pub const fn clear(&mut self) {
		let is_mouse_captured = self.mouse_captured();
		*self = Self::default();
		self.set_mouse_captured(is_mouse_captured);
	}

    /// Compares two ModifiersState and returns the differences as a new ModifiersState.
    /// Each bit in the returned value represents a modifier that is in a different state
    /// between the two ModifiersState instances.
    pub fn compare(&self, other: &ModifiersState) -> ModifiersState {
        ModifiersState::from_bits(self.modifiers.bits() ^ other.bits()).unwrap_or_default()
    }
}

pub struct MouseButtonState {
	pub left: bool,
	pub right: bool,
	pub middle: bool,
	pub back: bool,
	pub forward: bool,
}
impl MouseButtonState {
	pub const fn default() -> Self {
		Self {
			left : false,
			right : false,
			middle : false,
			back : false,
			forward : false,
		}
	}
}
