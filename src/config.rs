
use winit::dpi::{PhysicalPosition,PhysicalSize};

//#[derive(Default)]
pub struct AppConfig {
    pub window_title: String,
    pub initial_window_size: PhysicalSize<f32>,
    pub initial_window_position: PhysicalPosition<f32>,
}

impl AppConfig {
    pub fn default(size: PhysicalSize<u32>) -> Self {
        let width:f32 = 1280.0; let height:f32 = 720.0;
        let x:f32 = (size.width as f32 - width) / 2.0;
        let y:f32 = (size.height as f32 - height) / 2.0;
        Self {
            window_title: "WGPU App".into(),
            initial_window_size: PhysicalSize::new(width, height),
            initial_window_position: PhysicalPosition::new(x,y),
        }
    }
}