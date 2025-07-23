
use crate::ext::settings::Settings;
use crate::network::api;
use crate::game::state::GameState;
use crate::ext::audio;
use crate::State;
use std::sync::atomic::{AtomicBool,AtomicPtr, Ordering};
use std::ptr;

// Replace your static mut variables with these:
pub static WINDOW_PTR: AtomicPtr<winit::window::Window> = AtomicPtr::new(ptr::null_mut());
pub static STATE_PTR: AtomicPtr<State<'static>> = AtomicPtr::new(ptr::null_mut());
pub static CLOSED: AtomicBool = AtomicBool::new(false);
pub static GAMESTATE_PTR: AtomicPtr<GameState> = AtomicPtr::new(ptr::null_mut());
pub static SETTINGS: AtomicPtr<Settings> = AtomicPtr::new(ptr::null_mut());


#[inline] pub fn init_settings() {
	let system = Box::new(Settings::default());
	
	let old_ptr = SETTINGS.swap(Box::into_raw(system), Ordering::AcqRel);
	if !old_ptr.is_null() {
		unsafe { let _ = Box::from_raw(old_ptr); }
	}
}
// Helper function to safely access the Settings pointer
#[inline] pub fn get_settings() -> &'static mut Settings {
	let ptr = SETTINGS.load(Ordering::Acquire);
	if ptr.is_null() {
		panic!("Settings not initialized");
	}
	unsafe { &mut *ptr }
}

// Safe accessor functions
#[inline]
pub fn get_window() -> &'static mut winit::window::Window {
	let ptr = WINDOW_PTR.load(Ordering::Acquire);
	if ptr.is_null() {
		panic!("Window not initialized");
	}
	unsafe { &mut *ptr }
}
#[inline]
pub fn get_state() -> &'static mut State<'static> {
	let ptr = STATE_PTR.load(Ordering::Acquire);
	if ptr.is_null() {
		panic!("State not initialized");
	}
	unsafe { &mut *ptr }
}
#[inline]
pub fn get_gamestate() -> &'static mut GameState {
	let ptr = GAMESTATE_PTR.load(Ordering::Acquire);
	if ptr.is_null() {
		panic!("GameState not initialized");
	}
	unsafe { &mut *ptr }
}
#[inline]
pub fn close_app() {
	CLOSED.store(true, Ordering::Release);
}
#[inline]
pub fn is_closed() -> bool {
	CLOSED.load(Ordering::Acquire)
}

// In your cleanup code (like when closing the app):
#[inline]
pub fn cleanup_resources() {
	// dropping the audio first (if not cleaned up properly it might play after app close)
	audio::stop_all_sounds();
	audio::cleanup_audio();
	// 1. Take ownership of the state pointer (atomically setting it to null)
	let state_ptr = STATE_PTR.swap(ptr::null_mut(), Ordering::AcqRel);
	// 2. If we got a non-null pointer, convert it back to Box to drop it
	if !state_ptr.is_null() {
		unsafe { let _ = Box::from_raw(state_ptr); }; // Drops when goes out of scope
	}
	drop_gamestate();
	api::cleanup_network();
	// 3. Do the same for the window
	let window_ptr = WINDOW_PTR.swap(ptr::null_mut(), Ordering::AcqRel);
	if !window_ptr.is_null() {
		unsafe { let _ = Box::from_raw(window_ptr); }; // Drops when goes out of scope
	}
}
#[inline]
pub fn drop_gamestate() {
	let gamestate_ptr = GAMESTATE_PTR.swap(ptr::null_mut(), Ordering::AcqRel);
	if !gamestate_ptr.is_null() {
		unsafe { let _ = Box::from_raw(gamestate_ptr); }; // Drops when goes out of scope
	}
}

