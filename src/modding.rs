use wasmtime::{Engine, Linker, Module, Store, Memory, Caller, Instance};
use std::path::Path;
use std::error::Error;
use std::fmt;

// rustc mods/mod_one.rs --target=wasm32-unknown-unknown --crate-type=cdylib -O -o comp_mods/mod_one.wasm

#[derive(Debug)]
struct ModError {
    message: String,
}

impl fmt::Display for ModError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for ModError {}

pub fn load_mod_one() -> Result<(), Box<dyn Error + Send + Sync + 'static>> {
    let engine = Engine::default();
    let path = Path::new("comp_mods").join("mod_one.wasm");
    let module = Module::from_file(&engine, path)?;
    let mut store = Store::new(&engine, ());
    let instance = Instance::new(&mut store, &module, &[])?;
    
    let memory = instance.get_memory(&mut store, "memory")
        .ok_or_else(|| ModError { message: "Failed to get memory".to_string() })?;

    let dealloc = instance.get_typed_func::<(i32, i32), ()>(&mut store, "dealloc")
        .map_err(|e| ModError { message: format!("Failed to do dealloc function: {}", e) })?;
    
    // 1. Update the function signature to use i64
    let greet = instance.get_typed_func::<(i32, i32), i64>(&mut store, "greet")
        .map_err(|e| ModError { message: format!("Failed to get greet function: {}", e) })?;

    // Prepare input
    let name = "User";
    let name_bytes = name.as_bytes();

    // Allocate and write memory
    let name_ptr = write_to_memory(&mut store, &memory, &instance, name_bytes)?;

    // 2. Call greet and unpack the i64 result
    let packed_result = greet.call(&mut store, (name_ptr, name_bytes.len() as i32))
        .map_err(|e| ModError { message: format!("greet call failed: {}", e) })?;

    // 3. Unpack the i64 into two i32 values
    let greeting_ptr = (packed_result >> 32) as i32;
    let greeting_len = (packed_result & 0xFFFFFFFF) as i32;

    // 4. Read the greeting string
    let greeting = read_string_from_memory(&mut store, &memory, greeting_ptr, greeting_len as usize)?;
    println!("{}", greeting);

    // 5. Free memory (both input and output)
    dealloc.call(&mut store, (name_ptr, name_bytes.len() as i32))
        .map_err(|e| ModError { message: format!("free call failed: {}", e) })?;

    dealloc.call(&mut store, (greeting_ptr, greeting_len))
        .map_err(|e| ModError { message: format!("free call failed: {}", e) })?;
    
    Ok(())
}

pub fn load_mod_two() -> Result<(), Box<dyn Error + Send + Sync>> {
    let engine = Engine::default();
    let mut linker = Linker::new(&engine);
    
    // Add host function with proper signature
    linker.func_wrap(
        "env", 
        "log", 
        |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
            let memory = caller.get_export("memory")
                .ok_or_else(|| wasmtime::Error::msg("no memory export"))?
                .into_memory()
                .ok_or_else(|| wasmtime::Error::msg("not a memory"))?;
                
            let mut buffer = vec![0u8; len as usize];
            memory.read(&mut caller, ptr as usize, &mut buffer)
                .map_err(|e| wasmtime::Error::msg(format!("failed to read memory: {}", e)))?;
                
            let msg = String::from_utf8(buffer)
                .map_err(|e| wasmtime::Error::msg(format!("invalid utf-8: {}", e)))?;
                
            println!("[WASM] {}", msg);
            Ok(())
        }
    )?;
    
    // Load module
    let path = Path::new("comp_mods").join("mod_two.wasm");
    let module = Module::from_file(&engine, path)?;
    let mut store = Store::new(&engine, ());
    
    // Instantiate with linker
    let instance = linker.instantiate(&mut store, &module)?;
    
    // Call main function if it exists
    if let Ok(main_func) = instance.get_typed_func::<(), ()>(&mut store, "main") {
        main_func.call(&mut store, ())?;
    }
    
    Ok(())
}

fn write_to_memory(
    store: &mut Store<()>,
    memory: &Memory,
    instance: &Instance,
    data: &[u8],
) -> Result<i32, Box<dyn Error + Send + Sync + 'static>> {
    let alloc = instance.get_typed_func::<i32, i32>(&mut *store, "alloc")
        .map_err(|e| ModError { message: format!("Failed to get alloc function: {}", e) })?;
    
    let ptr = alloc.call(&mut *store, data.len() as i32)
        .map_err(|e| ModError { message: format!("alloc call failed: {}", e) })?;
    
    memory.write(store, ptr as usize, data)
        .map_err(|e| ModError { message: format!("memory write failed: {}", e) })?;
    
    Ok(ptr)
}

fn read_string_from_memory(
    store: &mut Store<()>,
    memory: &Memory,
    ptr: i32,
    len: usize,
) -> Result<String, Box<dyn Error + Send + Sync + 'static>> {
    let mut buffer = vec![0u8; len];
    memory.read(store, ptr as usize, &mut buffer)
        .map_err(|e| ModError { message: format!("failed to read string: {}", e) })?;
    
    String::from_utf8(buffer)
        .map_err(|e| ModError { message: format!("invalid utf-8: {}", e) }.into())
}