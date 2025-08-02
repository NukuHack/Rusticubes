#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct RunningAverage {
	count: u64,
	average: f64,
	inv_count: f64,
	min: f64,
	max: f64,
}

impl std::fmt::Debug for RunningAverage {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		write!( f,
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
		write!( f,
			"⏱️─Benchmark Results:\n\
			├─ Samples: {}\n\
			├─ Average: {}\n\
			├─ Min:  {}\n\
			├─ Max:  {}\n\
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
	if num < 5000.0 {
		format!("{:.3}", num)
	} else if num < 5_000_000.0 {
		format!("{:.3}K", num / 1000.0)
	} else if num < 5_000_000_000.0 {
		format!("{:.3}M", num / 1_000_000.0)
	} else if num < 5_000_000_000_000.0 {
		format!("{:.3}B", num / 1_000_000_000.0)
	} else {
		format!("{:.3}T", num / 1_000_000_000_000.0)
	}
}

#[allow(dead_code)]
impl RunningAverage {
	#[inline] pub const fn new() -> Self {
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

	#[inline] pub const fn avg(&self) -> f64 {
		self.average
	}

	#[inline] pub const fn count(&self) -> u64 {
		self.count
	}

	#[inline] pub const fn sum(&self) -> f64 {
		self.count as f64 * self.average
	}

	#[inline] pub const fn min(&self) -> f64 {
		if self.count == 0 {
			f64::NAN
		} else {
			self.min
		}
	}

	#[inline] pub const fn max(&self) -> f64 {
		if self.count == 0 {
			f64::NAN
		} else {
			self.max
		}
	}

	#[inline] pub const fn clear(&mut self) {
		self.count = 0;
		self.average = 0.0;
		self.inv_count = 0.0;
		self.min = f64::INFINITY;
		self.max = f64::NEG_INFINITY;
	}


	#[inline] pub const fn default() -> Self {
		Self::new()
	}
}
