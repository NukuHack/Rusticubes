
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct RunningAverage {
    count: u64,
    average: f64, // Cache the reciprocal of count to avoid division in hot path
    inv_count: f64,
}

impl std::fmt::Debug for RunningAverage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Average {{  samples: {},  average: {} ,  total: {} }}",
            format_number(self.count() as f64),
            format_number(self.avg()),
            format_number(self.sum())
        )
    }
}
impl std::fmt::Display for RunningAverage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "⏱️─Benchmark Results:\n\
            ├─ Samples: {}\n\
            ├─ Average: {}\n\
            └─ Total: {}",
            format_number(self.count() as f64),
            format_number(self.avg()),
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
    // it can't get more than Trillions because f64 would overflow
}

#[allow(dead_code)]
impl RunningAverage {
    #[inline]
    pub fn new() -> Self {
        RunningAverage {
            count: 0,
            average: 0.0,
            inv_count: 0.0, // Will be set properly on first add
        }
    }

    #[inline]
    pub fn add(&mut self, value: f64) {
        self.count += 1;
        // Precompute reciprocal once
        self.inv_count = 1.0 / self.count as f64;
        // Use FMA (fused multiply-add) when available
        self.average = f64::mul_add(value - self.average, self.inv_count, self.average);
    }

    #[inline]
    pub fn avg(&self) -> f64 {
        self.average
    }

    #[inline]
    pub fn count(&self) -> u64 {
        self.count
    }

    #[inline]
    pub fn sum(&self) -> f64 {
        self.count as f64 * self.average
    }

    #[inline]
    pub fn clear(&mut self) {
        self.count = 0;
        self.average = 0.0;
        self.inv_count = 0.0;
    }
}

impl Default for RunningAverage {
    #[inline]
    fn default() -> Self {
        Self::new()
    }
}
