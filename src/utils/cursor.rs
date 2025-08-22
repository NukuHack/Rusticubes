
use winit::window::CursorGrabMode;
use crate::ui::manager;
use crate::ext::ptr;

impl<'a> crate::State<'a> {
	#[inline] pub fn center_mouse(&self) {
		if !self.window().has_focus() { return } // Don't try to center if window not focused

		let size: &winit::dpi::PhysicalSize<u32> = self.size();
		let (center_x, center_y) = (size.width as f64 / 2.0, size.height as f64 / 2.0);
	
		if let Err(e) = self.window().set_cursor_position(winit::dpi::PhysicalPosition::new(center_x, center_y)) {
			println!("error: {:?}", e);
		}
	}

	#[inline]
	pub fn toggle_mouse_capture(&mut self) {
		if self.is_world_running && !self.input_system.is_mouse_captured() {
			// if not in game ofc do not process
			if !ptr::get_gamestate().is_running() || !matches!(ptr::get_state().ui_manager.state, manager::UIState::InGame)
				{ return }
			self.input_system.set_mouse_captured(true);
			self.window().set_cursor_visible(false);
			self.window().set_cursor_grab(CursorGrabMode::Confined)
				.or_else(|_| self.window().set_cursor_grab(CursorGrabMode::Locked)).unwrap();
			self.center_mouse();
		} else {
			// if the game is not running release mouse all ways
			self.input_system.set_mouse_captured(false);
			// Show cursor and release
			//self.window().set_cursor_icon(winit::window::CursorIcon::Default);
			self.window().set_cursor_visible(true);
			self.window().set_cursor_grab(CursorGrabMode::None).unwrap();
		}
	}

}




/*

CursorIcon::Default       // Default cursor
CursorIcon::Crosshair     // Crosshair
CursorIcon::Hand          // Hand
CursorIcon::Arrow         // Arrow
CursorIcon::Move          // Move
CursorIcon::Text          // Text (I-beam)
CursorIcon::Wait          // Wait (hourglass)
CursorIcon::Help          // Help (arrow with question mark)
CursorIcon::Progress      // Progress (arrow with hourglass)
CursorIcon::NotAllowed    // Not allowed
CursorIcon::ContextMenu   // Context menu
CursorIcon::Cell          // Cell
CursorIcon::VerticalText  // Vertical text
CursorIcon::Alias         // Alias
CursorIcon::Copy          // Copy
CursorIcon::NoDrop        // No drop
CursorIcon::Grab          // Grab
CursorIcon::Grabbing      // Grabbing
CursorIcon::AllScroll     // All scroll
CursorIcon::ZoomIn        // Zoom in
CursorIcon::ZoomOut       // Zoom out
CursorIcon::EResize       // East resize
CursorIcon::NResize       // North resize


*/
