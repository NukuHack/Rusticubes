// ================================
// mod_one.rs - Fixed Module One
// ================================

// Import the functions from the host environment
#[link(wasm_import_module = "env")]
extern "C" {
    fn log(ptr: i32, len: i32);
    //fn alloc(size: i32) -> i32;
    //fn dealloc(ptr: i32, size: i32);
}

#[no_mangle]
pub extern "C" fn greet(name_ptr: i32, name_len: i32) -> i64 {
    let name = unsafe {
        std::str::from_utf8(std::slice::from_raw_parts(
            name_ptr as *const u8, 
            name_len as usize
        )).unwrap_or("invalid utf-8")
    };
    let greeting = format!("Hello, {}!", name);
    let greeting_bytes = greeting.into_bytes();
    let ptr = greeting_bytes.as_ptr() as i32;
    let len = greeting_bytes.len() as i32;
    std::mem::forget(greeting_bytes);
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
pub extern "C" fn get_string_len(ptr: i32) -> i32 {
    unsafe {
        let mut len = 0;
        while *((ptr as *const u8).add(len)) != 0 {
            len += 1;
        }
        len as i32
    }
}

#[no_mangle]
pub extern "C" fn alloc(size: i32) -> i32 {
    let mut buf: Vec<u8> = Vec::with_capacity(size as usize);  // Explicit u8 type here
    let ptr = buf.as_mut_ptr() as usize as i32;
    std::mem::forget(buf);
    ptr
}

#[no_mangle]
pub extern "C" fn dealloc(ptr: i32, len: i32) {
    unsafe {
        let _ = Vec::from_raw_parts(ptr as *mut u8, len as usize, len as usize);
    }
}