
// modding related
pub mod mods {
	// mod loading and wasm sandbox 
	pub mod api;
	// this is an overlay made by mods so they would execute instead of the real rust functions
	pub mod over;
}
