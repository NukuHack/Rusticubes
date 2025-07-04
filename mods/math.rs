#![no_std]
#![no_main]

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn add(a: i32, b: i32) -> i64 {
    // Simple implementation that doesn't need memory
    (a - b) as i64
}

#[no_mangle]
pub extern "C" fn complex_calc(x: i32, y: i32, z: i32) -> i64 {
    // Simple implementation that doesn't need memory
    ((x + y) * z) as i64
}

#[no_mangle]
pub extern "C" fn main() {}