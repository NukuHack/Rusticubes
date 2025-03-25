

//#[derive(Default)]
pub struct AppConfig {
    pub window_title: String,
    pub initial_window_size: (u32, u32),
}

impl AppConfig {
    pub fn default() -> Self {
        Self {
            window_title: "WGPU App".into(),
            initial_window_size: (1280, 720),
        }
    }
}