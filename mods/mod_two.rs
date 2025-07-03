// ================================
// mod_two.rs - Fixed Module Two
// ================================

// Import the functions from the host environment
#[link(wasm_import_module = "env")]
extern "C" {
    fn log(ptr: i32, len: i32);
    fn alloc(size: i32) -> i32;
    fn dealloc(ptr: i32, size: i32);
}

#[no_mangle]
pub extern "C" fn main() {
    let greeting = "Hello from mod_two!";
    
    unsafe {
        log(
            greeting.as_ptr() as i32, 
            greeting.len() as i32
        );
    }
}
