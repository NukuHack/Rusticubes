
use wasmtime::*;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use std::path::Path;
use crate::mods::api::{WasmError, WasmRuntime};

// Core override system
#[derive(Clone)]
pub struct WasmOverrideSystem {
	runtime: Arc<RwLock<WasmRuntime>>,
	function_map: Arc<RwLock<HashMap<String, String>>>,
}

impl WasmOverrideSystem {
	pub fn new(runtime: WasmRuntime) -> Self {
		Self {
			runtime: Arc::new(RwLock::new(runtime)),
			function_map: Arc::new(RwLock::new(HashMap::new())),
		}
	}

	pub fn load_module(&self, path: &Path) -> Result<(), WasmError> {
		let module_name = path.file_stem()
			.and_then(|s| s.to_str())
			.ok_or(WasmError::InvalidModuleName)?;
		
		// Load the WASM module
		{
			let mut runtime = self.runtime.write().unwrap();
			runtime.load_module(module_name, path)?;
		}
		
		// Get the module's exported functions
		let module = {
			let runtime = self.runtime.read().unwrap();
			runtime.get_module(module_name)?.clone()
		};
		
		let mut function_map = self.function_map.write().unwrap();
		
		// Register all exported functions as potential overrides
		for export in module.exports() {
			if let ExternType::Func(_) = export.ty() {
				function_map.insert(export.name().to_string(), module_name.to_string());
			}
		}
		
		Ok(())
	}

	pub fn call_function(&self, name: &str, args: &[i32]) -> Result<i64, WasmError> {
		// Get module name first and release the lock immediately
		let module_name = {
			let function_map = self.function_map.read().unwrap();
			function_map.get(name).cloned().ok_or(WasmError::FunctionNotFound {
				function: name.to_string(),
			})?
		};
				
		// Try for 1 second, then give up
		match self.runtime.try_write() {
			Ok(mut runtime_guard) => {
				return runtime_guard.execute_wasm_fn(&module_name, |instance, store| {
					match args.len() {
						2 => {
							let func = instance.get_typed_func::<(i32, i32), i64>(&mut *store, name)?;
							func.call(store, (args[0], args[1])).map_err(Into::into)
						}
						3 => {
							let func = instance.get_typed_func::<(i32, i32, i32), i64>(&mut *store, name)?;
							func.call(store, (args[0], args[1], args[2])).map_err(Into::into)
						}
						_ => {
							println!("DEBUG: Invalid arg count: {}", args.len());
							Err(WasmError::FunctionNotFound { function: name.to_string() })
						},
					}
				});
			}
			Err(e) => {
				println!("Error: {}", e);
				Err(WasmError::FunctionNotFound { function: "deadlock".to_string() })
			}
		}
	}
}

// Modified macro definition
#[macro_export]
macro_rules! define_overridable {
	// First invocation in a module creates the shared override system
	($vis:vis mod $module_name:ident { $($rest:tt)* }) => {
		$vis mod $module_name {
			thread_local! {
				static OVERRIDE_SYSTEM: std::cell::RefCell<Option<crate::mods::over::WasmOverrideSystem>> = std::cell::RefCell::new(None);
			}
			#[allow(dead_code)]
			$vis fn set_override_system(system: crate::mods::over::WasmOverrideSystem) {
				OVERRIDE_SYSTEM.with(|s| *s.borrow_mut() = Some(system));
			}

			$crate::define_overridable!(@inner $($rest)*);
		}
	};
	
	// Inner macro for processing individual functions
	(@inner $vis:vis fn $name:ident($($arg:ident: $ty:ty),*) -> $ret:ty $body:block $($rest:tt)*) => {
		$vis fn $name($($arg: $ty),*) -> $ret {
			let args_vec = vec![$(($arg as i32)),*];
			
			OVERRIDE_SYSTEM.with(|system| {
				if let Some(system) = system.borrow().as_ref() {
					if let Ok(result) = system.call_function(stringify!($name), &args_vec) {
						return result as $ret;
					}
				}
				
				$body
			})
		}

		$crate::define_overridable!(@inner $($rest)*);
	};
	
	// Termination case
	(@inner) => {};
}

// Example usage
define_overridable! {
	pub mod example {
		pub fn add(a: i32, b: i32) -> i64 {
			(a + b).into()
		}
		
		pub fn complex_calc(x: i32, y: i32, z: i32) -> i64 {
			(x * y + z).into()
		}
	}
}


pub fn main() -> Result<(), Box<dyn std::error::Error>> {
	println!("Basic Add: {}", example::add(2, 3)); // Should be 5 (2+3)
	println!("Basic Complex: {}", example::complex_calc(1, 3, 4)); // Should be 7 (1*3+4)

	// Initialize the runtime and override system
	let runtime = WasmRuntime::new()?;
	let override_system = WasmOverrideSystem::new(runtime);
	
	// Load the WASM module with error handling
	let wasm_path = Path::new("comp_mods/math.wasm");
	override_system.load_module(wasm_path)
		.map_err(|e| format!("Failed to load module: {}", e))?;
	
	// Set the override system
	example::set_override_system(override_system.clone());
	println!("Wasm Add: {}", example::add(2, 3)); // Should be -1 (2-3)
	println!("Wasm Complex: {}", example::complex_calc(1, 3, 4)); // Should be 16 ((1+3)*4)
	
	Ok(())
}