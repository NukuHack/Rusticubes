

// ===== HELPER ENUM =====

#[derive(Clone, Copy)]
pub enum ClickMode {
	Left,      // Take/place entire stack
	Right,    // Take/place one item or half stack
	Middle,  // Take/place max possible stack
}
