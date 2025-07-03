#[no_mangle]
pub extern "C" fn main() {
    let greeting = "Hello from mod_two!";
    
    // Call log function - needs to be unsafe because it's an external FFI call
    unsafe {
        log(
            greeting.as_ptr() as i32, 
            greeting.len() as i32
        );
    }
}

// Import the functions from the host environment
#[link(wasm_import_module = "env")]
extern "C" {
    fn log(ptr: i32, len: i32);
    fn alloc(size: i32) -> i32;
    fn dealloc(ptr: i32, size: i32);
}

// If you want to provide your own implementations (remove the extern declarations above if you use these)
// #[no_mangle]
// pub extern "C" fn alloc(size: i32) -> i32 {
//     let mut buf = Vec::with_capacity(size as usize);
//     let ptr = buf.as_mut_ptr() as i32;
//     std::mem::forget(buf);
//     ptr
// }

// #[no_mangle]
// pub extern "C" fn dealloc(ptr: i32, size: i32) {
//     unsafe {
//         let _ = Vec::from_raw_parts(ptr as *mut u8, 0, size as usize);
//     }
// }