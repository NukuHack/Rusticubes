// ================================
// mod_two.rs
// ================================
#![no_std]

// Import the functions from the host environment
#[link(wasm_import_module = "env")]
extern "C" {
    fn log(ptr: i32, len: i32);
    //fn alloc(size: i32) -> i32;
    //fn dealloc(ptr: i32, size: i32);
}
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    let msg = "Panic occurred";
    unsafe {
        log(
            msg.as_ptr() as i32,
            msg.len() as i32
        );
    }
    loop {}
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