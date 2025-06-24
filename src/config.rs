
use crate::game::GameState;
use crate::State;
use std::sync::atomic::{AtomicBool,AtomicPtr, Ordering};
use std::path::PathBuf;

//#[derive(Default)]
pub struct AppConfig {
    pub window_title: String,
    pub initial_window_size: winit::dpi::PhysicalSize<f32>,
    pub min_window_size: winit::dpi::PhysicalSize<f32>,
    pub initial_window_position: winit::dpi::PhysicalPosition<f32>,
    pub theme: Option<winit::window::Theme>
}

impl Default for AppConfig {
     fn default() -> Self {
        Self {
            window_title: "Default App".into(),
            initial_window_size: winit::dpi::PhysicalSize::new(1280.0, 720.0),
            min_window_size: winit::dpi::PhysicalSize::new(600.0, 400.0),
            initial_window_position: winit::dpi::PhysicalPosition::new(100.0,100.0),
            theme: Some(winit::window::Theme::Dark),
        }
    }
}
impl AppConfig {
    pub fn new(size: winit::dpi::PhysicalSize<u32>) -> Self {
        let width:f32 = 1280.0; let height:f32 = 720.0;
        let x:f32 = (size.width as f32 - width) / 2.0;
        let y:f32 = (size.height as f32 - height) / 2.0;
        Self {
            window_title: "WGPU App".into(),
            initial_window_size: winit::dpi::PhysicalSize::new(width, height),
            min_window_size: winit::dpi::PhysicalSize::new(width/3.0, height/3.0),
            initial_window_position: winit::dpi::PhysicalPosition::new(x,y),
            ..Self::default()
        }
    }
}

pub fn get_save_path() -> PathBuf {
    let mut path = if cfg!(windows) {
        dirs::document_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("My Games")
    } else if cfg!(target_os = "macos") {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library/Application Support")
    } else {
        // Linux and others
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
    };

    path.push("Rusticubes");
    path
}

pub fn ensure_save_dir() -> std::io::Result<PathBuf> {
    let path = get_save_path();
    std::fs::create_dir_all(&path)?;
    Ok(path)
}


// Replace your static mut variables with these:
pub static WINDOW_PTR: AtomicPtr<winit::window::Window> = AtomicPtr::new(std::ptr::null_mut());
pub static STATE_PTR: AtomicPtr<State<'static>> = AtomicPtr::new(std::ptr::null_mut());
pub static CLOSED: AtomicBool = AtomicBool::new(false);
pub static GAMESTATE_PTR: AtomicPtr<GameState> = AtomicPtr::new(std::ptr::null_mut());

// Safe accessor functions
pub fn get_window() -> &'static mut winit::window::Window {
    let ptr = WINDOW_PTR.load(Ordering::Acquire);
    if ptr.is_null() {
        panic!("Window not initialized");
    }
    unsafe { &mut *ptr }
}

pub fn get_state() -> &'static mut State<'static> {
    let ptr = STATE_PTR.load(Ordering::Acquire);
    if ptr.is_null() {
        panic!("State not initialized");
    }
    unsafe { &mut *ptr }
}
pub fn get_gamestate() -> &'static mut GameState {
    let ptr = GAMESTATE_PTR.load(Ordering::Acquire);
    if ptr.is_null() {
        panic!("GameState not initialized");
    }
    unsafe { &mut *ptr }
}

pub fn close_app() {
    CLOSED.store(true, Ordering::Release);
}

pub fn is_closed() -> bool {
    CLOSED.load(Ordering::Acquire)
}

// In your cleanup code (like when closing the app):
pub fn cleanup_resources() {
    // dropping the audio first (if not cleaned up properly it might play after app close)
    super::audio::stop_all_sounds();
    super::audio::cleanup_audio();
    // 1. Take ownership of the state pointer (atomically setting it to null)
    let state_ptr = STATE_PTR.swap(std::ptr::null_mut(), Ordering::AcqRel);
    // 2. If we got a non-null pointer, convert it back to Box to drop it
    if !state_ptr.is_null() {
        unsafe { let _ = Box::from_raw(state_ptr); }; // Drops when goes out of scope
    }
    drop_gamestate();
    // 3. Do the same for the window
    let window_ptr = WINDOW_PTR.swap(std::ptr::null_mut(), Ordering::AcqRel);
    if !window_ptr.is_null() {
        unsafe { let _ = Box::from_raw(window_ptr); }; // Drops when goes out of scope
    }
}

pub fn drop_gamestate() {

    let gamestate_ptr = GAMESTATE_PTR.swap(std::ptr::null_mut(), Ordering::AcqRel);
    if !gamestate_ptr.is_null() {
        unsafe { let _ = Box::from_raw(gamestate_ptr); }; // Drops when goes out of scope
    }

}

