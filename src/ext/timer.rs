use std::time::Instant;

#[derive(Clone, Copy)]
pub struct RunningAverage {
	count: u64,
	average: f64,
	inv_count: f64,
	min: f64,
	max: f64,
}

impl std::fmt::Debug for RunningAverage {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"Average {{ samples: {}, average: {}, min: {}, max: {}, total: {} }}",
			format_number(self.count() as f64),
			format_number(self.avg()),
			format_number(self.min()),
			format_number(self.max()),
			format_number(self.sum())
		)
	}
}

impl std::fmt::Display for RunningAverage {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!(
			f,
			"⏱️ Benchmark Results:\n\
			 ├─ Samples: {}\n\
			 ├─ Average: {}\n\
			 ├─ Min:     {}\n\
			 ├─ Max:     {}\n\
			 └─ Total:   {}",
			format_number(self.count() as f64),
			format_number(self.avg()),
			format_number(self.min()),
			format_number(self.max()),
			format_number(self.sum())
		)
	}
}

pub fn format_number(num: f64) -> String {
	if num < 1000.0 {
		format!("{:.2}", num)
	} else if num < 1_000_000.0 {
		format!("{:.2}K", num / 1000.0)
	} else if num < 1_000_000_000.0 {
		format!("{:.2}M", num / 1_000_000.0)
	} else if num < 1_000_000_000_000.0 {
		format!("{:.2}B", num / 1_000_000_000.0)
	} else {
		format!("{:.2}T", num / 1_000_000_000_000.0)
	}
}

impl RunningAverage {
	#[inline]
	pub const fn new() -> Self {
		RunningAverage {
			count: 0,
			average: 0.0,
			inv_count: 0.0,
			min: f64::INFINITY,
			max: f64::NEG_INFINITY,
		}
	}

	#[inline]
	pub fn add(&mut self, value: f64) {
		self.count += 1;
		self.inv_count = 1.0 / self.count as f64;
		self.average = f64::mul_add(value - self.average, self.inv_count, self.average);
		
		if value < self.min {
			self.min = value;
		}
		if value > self.max {
			self.max = value;
		}
	}

	#[inline]
	pub const fn avg(&self) -> f64 {
		self.average
	}

	#[inline]
	pub const fn count(&self) -> u64 {
		self.count
	}

	#[inline]
	pub const fn sum(&self) -> f64 {
		self.count as f64 * self.average
	}

	#[inline]
	pub const fn min(&self) -> f64 {
		if self.count == 0 {
			f64::NAN
		} else {
			self.min
		}
	}

	#[inline]
	pub const fn max(&self) -> f64 {
		if self.count == 0 {
			f64::NAN
		} else {
			self.max
		}
	}

	#[inline]
	pub fn clear(&mut self) {
		*self = Self::new();
	}
}

impl Default for RunningAverage {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Debug)]
pub struct FPSCounter {
    frame_count: u32,
    last_update: Instant,
    last_reset: Instant,
    fps: f64,
    frame_times: RunningAverage,
    smoothing_factor: f64,
}

impl FPSCounter {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            frame_count: 0,
            last_update: now,
            last_reset: now,
            fps: 0.0,
            frame_times: RunningAverage::new(),
            smoothing_factor: 0.95,
        }
    }
    
    pub fn with_smoothing_factor(smoothing_factor: f64) -> Self {
        Self {
            smoothing_factor: smoothing_factor.clamp(0.0, 1.0),
            ..Self::new()
        }
    }
    
    pub fn update(&mut self) -> f64 {
        let now = Instant::now();
        let delta_time = now.duration_since(self.last_update).as_secs_f64();
        self.last_update = now;
        
        self.frame_count += 1;
        
        // Calculate frame time in milliseconds
        let frame_time_ms = delta_time * 1000.0;
        self.frame_times.add(frame_time_ms);
        
        // Calculate FPS with exponential smoothing for smoother display
        let current_fps = 1.0 / delta_time;
        self.fps = if self.fps == 0.0 {
            current_fps
        } else {
            self.smoothing_factor * self.fps + (1.0 - self.smoothing_factor) * current_fps
        };
        
        // Reset counter every second for accurate average calculation
        if now.duration_since(self.last_reset).as_secs_f64() >= 1.0 {
            self.frame_count = 0;
            self.last_reset = now;
        }
        
        self.fps
    }
	
	pub fn fps(&self) -> f64 {
		self.fps
	}
	
	pub fn frame_time_ms(&self) -> f64 {
		self.frame_times.avg()
	}
	
	pub fn min_frame_time_ms(&self) -> f64 {
		self.frame_times.min()
	}
	
	pub fn max_frame_time_ms(&self) -> f64 {
		self.frame_times.max()
	}
	
	pub fn frame_time_stats(&self) -> &RunningAverage {
		&self.frame_times
	}
	
	pub fn reset(&mut self) {
		*self = Self::new();
	}
}

impl Default for FPSCounter {
	fn default() -> Self {
		Self::new()
	}
}

