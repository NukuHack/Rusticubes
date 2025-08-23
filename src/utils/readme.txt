
these are stuff that are helping structs
so stuff what would be easily imported (like basic math and stuff) but i decided to make custom structs and functions for them


/// Utility things, like helper Structs
pub mod utils {
	/// Input handling (keyboard/mouse).
	pub mod input;
	/// Math utilities (Noise gen, lerping).
	pub mod math;
	// my custom color struct with quick init
	pub mod color;
	/// String helpers (For compile and runtime strings).
	pub mod string;
	/// Random Number Generator
	pub mod rng;
	/// stuff for glam::Vec3 ... in const
	pub mod vec3;
	/// stuff for glam::Vec2 ... in const
	pub mod vec2;
	/// Cursor state (cursor change and locking).
	pub mod cursor;
	/// Time formatting is a pretty struct.
	pub mod time;
}

