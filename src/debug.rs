
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct RunningAverage {
    count: u64,
    average: f64, // Cache the reciprocal of count to avoid division in hot path
    inv_count: f64,
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
    pub fn average(&self) -> f64 {
        self.average
    }

    #[inline]
    pub fn count(&self) -> u64 {
        self.count
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
