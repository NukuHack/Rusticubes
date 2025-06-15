
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