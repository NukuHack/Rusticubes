use std::collections::HashMap;
use std::mem::MaybeUninit;
use std::sync::Once;

static INIT: Once = Once::new();
static mut GLOBAL_MAP: MaybeUninit<HashMap<String, String>> = MaybeUninit::uninit();

pub fn init_global_map() {
    INIT.call_once(|| {
        unsafe {
            GLOBAL_MAP.as_mut_ptr().write(HashMap::new());
        }
    });
}

pub fn get_global_map() -> &'static mut HashMap<String, String> {
    unsafe { &mut *GLOBAL_MAP.as_mut_ptr() }
}



fn main() {
    // Initialize exactly once
    init_global_map();
    
    // Get mutable reference
    let map = get_global_map();
    map.insert("key".to_string(), "value".to_string());
    
    // Later access
    let map = get_global_map();
    println!("Value: {}", map.get("key").unwrap());
}
