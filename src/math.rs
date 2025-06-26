// might need more of these small files for math and other extra shit 
use std::time::{SystemTime, UNIX_EPOCH};

pub fn random_float(min: f32, max: f32) -> f32 {
    // Seed based on current time (nanoseconds)
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .subsec_nanos();

    // Xorshift algorithm (fast but not cryptographically secure)
    let mut state = seed as u32;
    state ^= state << 13;
    state ^= state >> 17;
    state ^= state << 5;

    // Convert to float in [0, 1) range
    let rand_01 = (state as f32) / (u32::MAX as f32);

    // Scale to [min, max)
    min + rand_01 * (max - min)
}