this is just the debug and testing stuff 
this entire folder is probably will NOT be inside the --release builds
these are just doe the debug testing and stuff


/// debug, test related
#[cfg(test)]
pub mod debug {
	pub mod network;
	pub mod world;
	pub mod metadata;
	pub mod json_serial;
	pub mod serialize_item;
	pub mod physics;
}
