
use wasmtime::*;
use std::collections::HashMap;
use std::error::Error;
use std::{fmt,fs};
use std::path::{Path,PathBuf};

// rustc mods/mod_one.rs --target=wasm32-unknown-unknown --crate-type=cdylib -O -o comp_mods/mod_one.wasm

// cargo mods/mod_one.rs build --release --target wasm32-unknown-unknown --target-dir comp_mods/mod_one.wasm

// Error types remain the same as your original code
#[derive(Debug)]
#[allow(dead_code)]
pub enum WasmError {
	ModuleNotFound { module: String },
	MemoryNotFound,
	Utf8Conversion { error: String },
	Wasmtime { error: wasmtime::Error },
	MemoryAccess { error: MemoryAccessError },
	IOError { error: String },
	InvalidModuleName,
	BulkError { errors: Vec<(String, WasmError)> },
	FunctionNotFound { function: String },
	Unexpected,
}

impl Error for WasmError {}

impl fmt::Display for WasmError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Self::ModuleNotFound { module } => write!(f, "Module '{}' not found", module),
			Self::MemoryNotFound => write!(f, "Module has no memory"),
			Self::Utf8Conversion { error } => write!(f, "UTF-8 conversion error: {}", error),
			Self::Wasmtime { error } => write!(f, "Wasmtime error: {}", error),
			Self::MemoryAccess { error } => write!(f, "Memory access error: {}", error),
			Self::IOError { error } => write!(f, "IO failed: {}", error),
			Self::InvalidModuleName => write!(f, "Invalid module name"),
			Self::Unexpected => write!(f, "Unexpected error occurred"),
			Self::FunctionNotFound { function } => write!(f, "Function Not Found: {}", function),
			Self::BulkError { errors } => {
				write!(f, "Multiple errors occurred:\n")?;
				for (module, error) in errors {
					write!(f, "  - {}: {}\n", module, error)?;
				}
				Ok(())
			},
		}
	}
}
impl From<wasmtime::Error> for WasmError {
	fn from(err: wasmtime::Error) -> Self {
		Self::Wasmtime{error: err}
	}
}
impl From<std::string::FromUtf8Error> for WasmError {
	fn from(err: std::string::FromUtf8Error) -> Self {
		Self::Utf8Conversion{error : err.to_string()}
	}
}
impl From<std::io::Error> for WasmError {
	fn from(err: std::io::Error) -> Self {
		Self::Utf8Conversion{error : err.to_string()}
	}
}
impl From<MemoryAccessError> for WasmError {
	fn from(err: MemoryAccessError) -> Self {
		Self::MemoryAccess{error: err}
	}
}

#[derive(Clone)]
pub struct ModuleData {
	pub instance: Instance,
	pub memory: Memory,
	pub module: Module,  // Added to store the module for inspection
}

pub struct WasmRuntime {
	pub engine: Engine,
	pub store: Store<()>,
	pub linker: Linker<()>,
	pub instances: HashMap<String, ModuleData>,
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
		linker.func_wrap( "env", "log", |mut caller: Caller<'_, ()>, ptr: i32, len: i32| {
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
		linker.func_wrap( "env", "alloc", |mut caller: Caller<'_, ()>, size: i32| -> Result<i32, wasmtime::Error> {
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
		linker.func_wrap( "env", "dealloc", |_ptr: i32, _size: i32| {
				// No-op for now
			}
		)?;
		
		Ok(())
	}
	

	pub fn load_module(&mut self, name: &str, path: &Path) -> Result<(), WasmError> {
		let module = Module::from_file(&self.engine, path)?;
		let instance = self.linker.instantiate(&mut self.store, &module)?;
		let memory = instance.get_memory(&mut self.store, "memory")
			.ok_or(WasmError::MemoryNotFound)?;
		
		self.instances.insert(name.to_string(), ModuleData {
			instance,
			memory,
			module,
		});
		
		Ok(())
	}
	
	pub fn get_module(&self, name: &str) -> Result<&Module, WasmError> {
		self.instances.get(name)
			.map(|data| &data.module)
			.ok_or_else(|| WasmError::ModuleNotFound { module: name.to_string() })
	}
		
	pub fn call_function_with_data(&mut self, module: &str, func: &str, data: &[u8]) -> Result<String, WasmError> {
		// Split into discrete operations to avoid overlapping borrows
		let input_ptr = {
			let m_data = self.instances.get_mut(module)
				.ok_or_else(|| WasmError::ModuleNotFound { module: module.to_string() })?;
			
			let alloc = m_data.instance.get_typed_func::<i32, i32>(&mut self.store, "alloc")?;
			let ptr = alloc.call(&mut self.store, data.len() as i32)?;
			m_data.memory.write(&mut self.store, ptr as usize, data)?;
			ptr
		};

		let packed_result = {
			let m_data = self.instances.get_mut(module)
				.ok_or_else(|| WasmError::ModuleNotFound { module: module.to_string() })?;
			
			let greet = m_data.instance.get_typed_func::<(i32, i32), i64>(&mut self.store, func)?;
			greet.call(&mut self.store, (input_ptr, data.len() as i32))?
		};

		let result = {
			let m_data = self.instances.get_mut(module)
				.ok_or_else(|| WasmError::ModuleNotFound { module: module.to_string() })?;
			
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
			.ok_or_else(|| WasmError::ModuleNotFound { module: module.to_string() })?;
		
		let func = m_data.instance.get_typed_func::<(), ()>(&mut self.store, func)?;
		func.call(&mut self.store, ())?;
		Ok(())
	}

	pub fn call_function_i32(&mut self, module: &str, func: &str, arg: i32) -> Result<i32, WasmError> {
		let m_data = self.instances.get_mut(module)
			.ok_or_else(|| WasmError::ModuleNotFound { module: module.to_string() })?;
		
		let func = m_data.instance.get_typed_func::<i32, i32>(&mut self.store, func)?;
		let result = func.call(&mut self.store, arg)?;
		Ok(result)
	}

	pub fn call_function_two_i32(&mut self, module: &str, func: &str, arg1: i32, arg2: i32) -> Result<i32, WasmError> {
		let m_data = self.instances.get_mut(module)
			.ok_or_else(|| WasmError::ModuleNotFound { module: module.to_string() })?;
		
		let func = m_data.instance.get_typed_func::<(i32, i32), i32>(&mut self.store, func)?;
		let result = func.call(&mut self.store, (arg1, arg2))?;
		Ok(result)
	}
	
	pub fn get_instance_mut(&mut self, name: &str) -> Result<&mut ModuleData, WasmError> {
		self.instances.get_mut(name)
			.ok_or_else(|| WasmError::ModuleNotFound { module: name.to_string() })
	}

	// Renamed to be more explicit and avoid confusion
	pub fn execute_wasm_fn<F, R>(&mut self, module: &str, f: F) -> Result<R, WasmError>
	where
		F: FnOnce(&mut Instance, &mut Store<()>) -> Result<R, WasmError>,
	{
		// Get the ModuleData first
		let module_data = self.instances.get_mut(module)
			.ok_or_else(|| WasmError::ModuleNotFound { module: module.to_string() })?;
		
		// Then execute with both components
		f(&mut module_data.instance, &mut self.store)
	}

	
	pub fn list_modules(&self) -> Vec<String> {
		self.instances.keys().cloned().collect()
	}

	pub fn unload_module(&mut self, name: &str) -> Result<(), WasmError> {
		self.instances.remove(name)
			.map(|_| ())
			.ok_or(WasmError::ModuleNotFound{module : name.to_string()})
	}

	pub fn clear_all_modules(&mut self) -> Result<(), WasmError> {
		let modules = self.list_modules();
		let mut errors = Vec::new();

		for module in modules {
			if let Err(e) = self.unload_module(&module) {
				errors.push((module.clone(), e));
			}
		}

		if !errors.is_empty() {
			return Err(WasmError::BulkError{errors});
		}

		Ok(())
	}
	
	pub fn has_module(&self, name: &str) -> bool {
		self.instances.contains_key(name)
	}
}

impl WasmRuntime {
	/// Scans a directory for all .wasm files
	pub fn find_wasm_modules(directory: &str) -> Result<Vec<PathBuf>, WasmError> {
		let dir_path = Path::new(directory);
		if !dir_path.exists() {
			return Err(WasmError::IOError{ error: "Directory does not exists".to_string()});
		}

		let wasm_files = fs::read_dir(dir_path)?
			.filter_map(|entry| {
				let entry = entry.ok()?;
				let path = entry.path();
				if path.extension()?.to_str()? == "wasm" {
					Some(path)
				} else {
					None
				}
			})
			.collect();

		Ok(wasm_files)
	}

	/// Executes a function in a loaded module with optional data
	fn execute_module_function(
		&mut self,
		module_name: &str,
		function_name: &str,
		data: Option<&[u8]>,
	) -> Result<Option<String>, WasmError> {
		match data {
			Some(d) => {
				let result = self.call_function_with_data(module_name, function_name, d)?;
				Ok(Some(result))
			}
			None => {
				self.call_function_simple(module_name, function_name)?;
				Ok(None)
			}
		}
	}

	/// Initializes all modules by running their "main" function
	pub fn initialize_all_modules(&mut self) -> Result<(), WasmError> {
		let modules = WasmRuntime::find_wasm_modules("comp_mods")?;

		for path in modules {
			let module_name = path.file_stem()
				.and_then(|s| s.to_str())
				.ok_or(WasmError::InvalidModuleName)?;

			self.load_module(module_name, &path)?;
			self.execute_module_function(module_name, "main", None)?;
		}

		Ok(())
	}

	/// Example usage showing how to work with specific modules
	pub fn run_extra_mod(&mut self) -> Result<(), WasmError> {
		// Interact with specific modules as needed
		let greeting = self.execute_module_function(
			"mod_one", 
			"greet", 
			Some("User".as_bytes())
		)?;
		if let Some(g) = greeting {
			println!("{}", g);
		}

		Ok(())
	}
}

// I would like to make some "listeners" or idk to modify / overwrite the vanilla code



pub fn main() -> Result<(), Box<dyn std::error::Error>> {
	let mut wasm_modder = WasmRuntime::new()?;
		
	// Propagate initialization errors
	wasm_modder.initialize_all_modules()?;
	
	// Run extra mod and propagate any errors
	wasm_modder.run_extra_mod()?;
	
	Ok(())
}
