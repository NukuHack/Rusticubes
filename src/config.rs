pub struct AppConfig {
    pub window_title: String,
    pub initial_window_size: (u32, u32),
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window_title: "WGPU App".to_string(),
            initial_window_size: (1280, 720),
        }
    }
}