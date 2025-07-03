use std::mem;

#[no_mangle]
pub extern "C" fn main() {
    let greeting = "Hello from mod_two!";
    
    // Print to console
    unsafe {
        log(
            greeting.as_ptr() as i32, 
            greeting.len() as i32
        );
    }
}

// Changed to match i32 parameters (WASM uses i32 for pointers/lengths)
extern "C" {
    fn log(ptr: i32, len: i32);
}

// Memory management functions
#[no_mangle]
pub extern "C" fn alloc(size: i32) -> i32 {
    let mut buf: Vec<u8> = Vec::with_capacity(size as usize);  // Explicit u8 type here
    let ptr = buf.as_mut_ptr() as usize as i32;
    mem::forget(buf);
    ptr
}

#[no_mangle]
pub extern "C" fn dealloc(ptr: i32, size: i32) {
    unsafe {
        let _ = Vec::from_raw_parts(ptr as *mut u8, size as usize, size as usize);
    }
}