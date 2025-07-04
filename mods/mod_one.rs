// ================================
// mod_one.rs
// ================================

// Import the functions from the host environment
#[link(wasm_import_module = "env")]
extern "C" {
    fn log(ptr: i32, len: i32);
    //fn alloc(size: i32) -> i32;
    //fn dealloc(ptr: i32, size: i32);
}
/*
// only used with the "no-std" 
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
*/

#[no_mangle]
pub extern "C" fn greet(name_ptr: i32, name_len: i32) -> i64 {
    let name = unsafe {
        let slice = std::slice::from_raw_parts(name_ptr as *const u8, name_len as usize);
        str::from_utf8(slice).unwrap_or("invalid utf-8")
    };
    
    let greeting = String::from("Hello, ") + name + "!";
    let greeting_bytes = greeting.into_bytes();
    let ptr = greeting_bytes.as_ptr() as i32;
    let len = greeting_bytes.len() as i32;
    core::mem::forget(greeting_bytes);
    ((ptr as i64) << 32) | (len as i64)
}

#[no_mangle]
pub extern "C" fn main() {
    let greeting = "Hello from mod_one!";
    
    unsafe {
        log(
            greeting.as_ptr() as i32, 
            greeting.len() as i32
        );
    }
}

#[no_mangle]
pub extern "C" fn alloc(size: i32) -> i32 {
    let mut buf: Vec<u8> = Vec::with_capacity(size as usize);  // Explicit u8 type here
    let ptr = buf.as_mut_ptr() as usize as i32;
    core::mem::forget(buf);
    ptr
}

#[no_mangle]
pub extern "C" fn dealloc(ptr: i32, len: i32) {
    unsafe {
        let _ = Vec::from_raw_parts(ptr as *mut u8, len as usize, len as usize);
    }
}