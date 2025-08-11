here the "ext" refers to extra



// extra things that did not fit anywhere else
pub mod ext {
	// audio manager, in extra thread
	pub mod audio;
	// basic configs
	pub mod config;
	// main settings
	pub mod settings;
	// all the pointers and stuff for globar variables
	pub mod ptr;
	// basic struct used for debugging and profiling
	pub mod stopwatch;
	// memory management mainly focusing on memory clean up
	pub mod memory;
}
