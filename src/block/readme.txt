this is a folder about the Block related things, 
it also contains the block storage aka Chunks and some helping functions like math related stuff to convert between the position and other needed variable types


/// block and chunk related
pub mod block {
	/// main block and chunk struct and basic fn
	pub mod main;
	/// chunk and block coords struct and all fn
	pub mod math;
	/// block & chunk interaction and world modification
	pub mod extra;
}
