use wasmtime::*;
use std::collections::HashMap;
use std::error::Error;
use std::fmt;
use std::path::Path;

// rustc mods/mod_one.rs --target=wasm32-unknown-unknown --crate-type=cdylib -O -o comp_mods/mod_one.wasm

// cargo mods/mod_one.rs build --release --target wasm32-unknown-unknown --target-dir comp_mods/mod_one.wasm

#[derive(Debug)]
pub struct WasmError {
    message: String,
}

impl fmt::Display for WasmError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl Error for WasmError {}

impl From<wasmtime::Error> for WasmError {
    fn from(err: wasmtime::Error) -> Self {
        WasmError {
            message: err.to_string(),
        }
    }
}
impl From<std::string::FromUtf8Error> for WasmError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        WasmError {
            message: format!("UTF-8 conversion error: {}", err),
        }
    }
}
impl From<MemoryAccessError> for WasmError {
    fn from(err: MemoryAccessError) -> Self {
        WasmError {
            message: format!("Memory access error: {}", err),
        }
    }
}

struct ModuleData {
    instance: Instance,
    memory: Memory,
}
#[allow(dead_code)]
pub struct WasmRuntime {
    engine: Engine,
    store: Store<()>,
    linker: Linker<()>,
    instances: HashMap<String, ModuleData>,
}
#[allow(dead_code)]
impl WasmRuntime {
    pub fn new() -> Result<Self, WasmError> {
        let engine = Engine::default();
        let mut linker = Linker::new(&engine);
        let store = Store::new(&engine, ());

        // Setup common host functions
        Self::setup_host_functions(&mut linker)?;
        
        Ok(WasmRuntime {
            engine,
            store,
            linker,
            instances: HashMap::new(),
        })
    }
    
    fn setup_host_functions(linker: &mut Linker<()>) -> Result<(), WasmError> {
        // Define the log function
        linker.func_wrap(
            "env", 
            "log", 
            |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
                let memory = caller.get_export("memory")
                    .and_then(|e| e.into_memory())
                    .ok_or_else(|| wasmtime::Error::msg("memory not found"))?;
                    
                let mut buffer = vec![0u8; len as usize];
                memory.read(&caller, ptr as usize, &mut buffer)
                    .map_err(|e| wasmtime::Error::msg(format!("memory read failed: {}", e)))?;
                    
                let msg = String::from_utf8(buffer)
                    .map_err(|e| wasmtime::Error::msg(format!("invalid utf-8: {}", e)))?;
                println!("[WASM] {}", msg);
                
                Ok(())
            }
        )?;
        
        // Define alloc function - using WASM memory allocation
        linker.func_wrap(
            "env",
            "alloc",
            |mut caller: Caller<'_, ()>, size: i32| -> Result<i32, wasmtime::Error> {
                let memory = caller.get_export("memory")
                    .and_then(|e| e.into_memory())
                    .ok_or_else(|| wasmtime::Error::msg("memory not found"))?;
                
                // Find some space in memory (simplified)
                let current_size = memory.size(&caller);
                let needed_pages = ((size as usize + 65535) / 65536) as u64;
                
                if current_size < needed_pages {
                    memory.grow(&mut caller, needed_pages - current_size)
                        .map_err(|e| wasmtime::Error::msg(format!("failed to grow memory: {}", e)))?;
                }
                
                // Return offset from start of memory (simple allocation)
                Ok((current_size * 65536) as i32)
            }
        )?;

        // Define dealloc function (no-op for simplicity)
        linker.func_wrap(
            "env",
            "dealloc",
            |_ptr: i32, _size: i32| {
                // No-op for now
            }
        )?;
        
        Ok(())
    }
    
    pub fn load_module(&mut self, name: &str, path: &Path) -> Result<(), WasmError> {
        let module = Module::from_file(&self.engine, path)?;
        
        // Use the main linker that already has all host functions defined
        let instance = self.linker.instantiate(&mut self.store, &module)?;
        let memory = instance.get_memory(&mut self.store, "memory")
            .ok_or_else(|| WasmError { message: "Module has no memory".to_string() })?;
        
        self.instances.insert(name.to_string(), ModuleData {
            instance,
            memory,
        });
        
        Ok(())
    }
        
    pub fn call_function_with_data(&mut self, module: &str, func: &str, data: &[u8]) -> Result<String, WasmError> {
        // Split into discrete operations to avoid overlapping borrows
        let input_ptr = {
            let m_data = self.instances.get_mut(module)
                .ok_or_else(|| WasmError { message: format!("Module '{}' not found", module) })?;
            
            let alloc = m_data.instance.get_typed_func::<i32, i32>(&mut self.store, "alloc")?;
            let ptr = alloc.call(&mut self.store, data.len() as i32)?;
            m_data.memory.write(&mut self.store, ptr as usize, data)?;
            ptr
        };

        let packed_result = {
            let m_data = self.instances.get_mut(module)
                .ok_or_else(|| WasmError { message: format!("Module '{}' not found", module) })?;
            
            let greet = m_data.instance.get_typed_func::<(i32, i32), i64>(&mut self.store, func)?;
            greet.call(&mut self.store, (input_ptr, data.len() as i32))?
        };

        let result = {
            let m_data = self.instances.get_mut(module)
                .ok_or_else(|| WasmError { message: format!("Module '{}' not found", module) })?;
            
            let result_ptr = (packed_result >> 32) as i32;
            let result_len = (packed_result & 0xFFFFFFFF) as i32;
            
            let mut buffer = vec![0u8; result_len as usize];
            m_data.memory.read(&mut self.store, result_ptr as usize, &mut buffer)?;
            String::from_utf8(buffer)?
        };

        Ok(result)
    }

    pub fn call_function_simple(&mut self, module: &str, func: &str) -> Result<(), WasmError> {
        let m_data = self.instances.get_mut(module)
            .ok_or_else(|| WasmError { message: format!("Module '{}' not found", module) })?;
        
        let func = m_data.instance.get_typed_func::<(), ()>(&mut self.store, func)?;
        func.call(&mut self.store, ())?;
        Ok(())
    }

    pub fn call_function_i32(&mut self, module: &str, func: &str, arg: i32) -> Result<i32, WasmError> {
        let m_data = self.instances.get_mut(module)
            .ok_or_else(|| WasmError { message: format!("Module '{}' not found", module) })?;
        
        let func = m_data.instance.get_typed_func::<i32, i32>(&mut self.store, func)?;
        let result = func.call(&mut self.store, arg)?;
        Ok(result)
    }

    pub fn call_function_two_i32(&mut self, module: &str, func: &str, arg1: i32, arg2: i32) -> Result<i32, WasmError> {
        let m_data = self.instances.get_mut(module)
            .ok_or_else(|| WasmError { message: format!("Module '{}' not found", module) })?;
        
        let func = m_data.instance.get_typed_func::<(i32, i32), i32>(&mut self.store, func)?;
        let result = func.call(&mut self.store, (arg1, arg2))?;
        Ok(result)
    }
    
    fn get_instance_mut(&mut self, name: &str) -> Result<&mut ModuleData, WasmError> {
        self.instances.get_mut(name)
            .ok_or_else(|| WasmError { message: format!("Module '{}' not found", name) })
    }
    
    pub fn list_modules(&self) -> Vec<String> {
        self.instances.keys().cloned().collect()
    }
    
    pub fn unload_module(&mut self, name: &str) -> Result<(), WasmError> {
        self.instances.remove(name);
        Ok(())
    }
    
    pub fn has_module(&self, name: &str) -> bool {
        self.instances.contains_key(name)
    }
}

// Example usage functions
impl WasmRuntime {
    // mod one focuses on variable passing (alloc de-alloc) and stuff
    pub fn load_mod_one(&mut self) -> Result<(), WasmError> {
        let path = Path::new("comp_mods").join("mod_one.wasm");
        self.load_module("mod_one", &path)?;
        
        let greeting = self.call_function_with_data("mod_one", "greet", "User".as_bytes())?;
        println!("{}", greeting);
        
        Ok(())
    }
    // mod two focuses on function passing, using function from main code
    pub fn load_mod_two(&mut self) -> Result<(), WasmError> {
        let path = Path::new("comp_mods").join("mod_two.wasm");
        self.load_module("mod_two", &path)?;
        
        self.call_function_simple("mod_two", "main")?;
        
        Ok(())
    }
}