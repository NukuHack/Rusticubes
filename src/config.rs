pub struct AppConfig {
    pub window_title: String,
    pub initial_window_size: (u32, u32),
    pub clear_color: wgpu::Color,
    pub camera_speed: f32,
    pub camera_sensitivity: f32,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            window_title: "WGPU App".to_string(),
            initial_window_size: (1280, 720),
            clear_color: wgpu::Color::BLACK,
            camera_speed: 1.0,
            camera_sensitivity: 0.05,
        }
    }
}