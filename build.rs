use std::{
    fs, io,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    // Handle resource compression (your existing system)
    handle_resource_compression();

    // Handle WASM plugin compilation
    handle_wasm_compilation();
}

fn handle_resource_compression() {
    println!("cargo:rerun-if-changed=resources");

    let resource_path = Path::new("resources");
    let comp_resource_path = Path::new("comp_resources");

    // Handle case when neither directory exists
    if !resource_path.exists() && !comp_resource_path.exists() {
        eprintln!("Error: Neither 'resources' nor 'comp_resources' directory exists");
        eprintln!("Hint: Create a 'resources' directory with your files");
        panic!("Required directories not found");
    }

    // Handle case when only comp_resources exists
    if !resource_path.exists() {
        if comp_resource_path.exists() {
            println!("Note: 'resources' directory not found, but 'comp_resources' exists - nothing to do");
            return;
        }
    }

    // Create comp_resources if it doesn't exist
    if !comp_resource_path.exists() {
        println!("Creating 'comp_resources' directory...");
        fs::create_dir_all(comp_resource_path).expect("Failed to create comp_resources directory");
    }

    // Collect all files that need processing
    let mut files_to_process = Vec::new();
    let mut total_files = 0;
    let mut up_to_date_files = 0;

    for entry in walkdir::WalkDir::new(resource_path) {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                eprintln!("Warning: Failed to read directory entry: {}", e);
                continue;
            }
        };

        if entry.file_type().is_file() {
            total_files += 1;
            let path = entry.path();

            let relative_path = match path.strip_prefix(resource_path) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to get relative path for {}: {}",
                        path.display(),
                        e
                    );
                    continue;
                }
            };

            let mut compressed_path = comp_resource_path.join(relative_path).to_string_lossy().into_owned();
            compressed_path.push_str(".lz4");
            let compressed_path = PathBuf::from(compressed_path);

            match needs_update(path, &compressed_path) {
                Ok(true) => files_to_process.push((path.to_owned(), compressed_path)),
                Ok(false) => up_to_date_files += 1,
                Err(e) => {
                    eprintln!("Warning: Skipping {} due to error: {}", path.display(), e);
                }
            }
        }
    }

    // Early exit if nothing to do
    if files_to_process.is_empty() {
        println!(
            "All resource files up-to-date ({} files, {} already compressed)",
            total_files, up_to_date_files
        );
        return;
    }

    println!(
        "Processing {} of {} resource files ({} up-to-date)...",
        files_to_process.len(),
        total_files,
        up_to_date_files
    );

    // Process the files that need updating
    let mut processed_count = 0;
    let mut skipped_count = 0;

    for (source_path, compressed_path) in files_to_process {
        print!("Processing {}... ", source_path.display());

        if let Err(e) = process_file(&source_path, &compressed_path) {
            eprintln!("FAILED: {}", e);
            skipped_count += 1;
            continue;
        }

        processed_count += 1;
        println!("OK");
    }

    println!(
        "Resource compression completed: {} processed, {} skipped, {} up-to-date",
        processed_count, skipped_count, up_to_date_files
    );
}


fn process_file(source_path: &Path, compressed_path: &Path) -> io::Result<()> {
    let data = fs::read(source_path)?;
    let compressed = lz4_flex::compress_prepend_size(&data);

    if let Some(parent) = compressed_path.parent() {
        fs::create_dir_all(parent)?;
    }

    fs::write(compressed_path, compressed)?;

    if let Ok(metadata) = fs::metadata(source_path) {
        if let Ok(time) = metadata.modified() {
            let _ = filetime::set_file_mtime(
                compressed_path,
                filetime::FileTime::from_system_time(time),
            );
        }
    }

    Ok(())
}


fn handle_wasm_compilation() {
    println!("cargo:rerun-if-changed=mods");

    let mods_path = Path::new("mods");
    let comp_mods_path = Path::new("comp_mods");

    if !mods_path.exists() {
        if comp_mods_path.exists() {
            println!("Note: 'mods' directory not found, but 'comp_mods' exists - nothing to do");
        } else {
            println!("Note: No 'mods' directory found - no plugins to compile");
        }
        return;
    }

    if !comp_mods_path.exists() {
        fs::create_dir_all(comp_mods_path).expect("Failed to create comp_mods directory");
    }

    let mut files_to_compile = Vec::new();
    let mut total_plugins = 0;
    let mut up_to_date_plugins = 0;

    for entry in fs::read_dir(mods_path).expect("Failed to read mods directory") {
        let entry = entry.expect("Invalid directory entry");
        let path = entry.path();
        
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
            total_plugins += 1;
            
            let plugin_name = path.file_stem().unwrap().to_str().unwrap();
            let wasm_output = comp_mods_path.join(format!("{}.wasm", plugin_name));
            
            match needs_update(&path, &wasm_output) {
                Ok(true) => files_to_compile.push((path, wasm_output)),
                Ok(false) => up_to_date_plugins += 1,
                Err(e) => eprintln!("Warning: Skipping {} due to error: {}", path.display(), e),
            }
        }
    }

    if files_to_compile.is_empty() {
        println!(
            "All plugins up-to-date ({} plugins, {} already compiled)",
            total_plugins, up_to_date_plugins
        );
        return;
    }

    println!(
        "Compiling {} of {} plugins ({} up-to-date)...",
        files_to_compile.len(),
        total_plugins,
        up_to_date_plugins
    );

    // Check if wasm32-unknown-unknown target is installed
    if !is_wasm_target_installed() {
        eprintln!("ERROR: wasm32-unknown-unknown target not installed!");
        eprintln!("Install it with: rustup target add wasm32-unknown-unknown");
        return;
    }

    let mut success_count = 0;
    let mut fail_count = 0;

    for (source_path, wasm_output) in files_to_compile {
        print!("Compiling {}... ", source_path.display());
        
        if let Err(e) = compile_plugin(&source_path, &wasm_output) {
            eprintln!("FAILED: {}", e);
            fail_count += 1;
        } else {
            println!("OK");
            success_count += 1;
        }
    }

    println!(
        "Plugin compilation completed: {} succeeded, {} failed, {} up-to-date",
        success_count, fail_count, up_to_date_plugins
    );
}

fn is_wasm_target_installed() -> bool {
    let target_list = Command::new("rustc")
        .arg("--print")
        .arg("target-list")
        .output()
        .map(|output| String::from_utf8_lossy(&output.stdout).into_owned())
        .unwrap_or_default();
    
    target_list.contains("wasm32-unknown-unknown")
}

fn compile_plugin(source_path: &Path, wasm_output: &Path) -> io::Result<()> {
    // First try with cargo if there's a Cargo.toml in the mods directory
    if let Ok(cargo_result) = try_compile_with_cargo(source_path, wasm_output) {
        return cargo_result;
    }

    // Fall back to rustc if cargo fails or isn't available
    compile_with_rustc(source_path, wasm_output)
}

fn try_compile_with_cargo(source_path: &Path, wasm_output: &Path) -> io::Result<io::Result<()>> {
    let mods_dir = source_path.parent().expect("Source file has no parent directory");
    let cargo_toml = mods_dir.join("Cargo.toml");
    
    if !cargo_toml.exists() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "No Cargo.toml found"));
    }

    let crate_name = source_path.file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "Invalid file name"))?;

    let output = Command::new("cargo")
        .current_dir(mods_dir)
        .arg("build")
        .arg("--release")
        .arg("--target")
        .arg("wasm32-unknown-unknown")
        .arg("--target-dir")
        .arg(wasm_output.parent().expect("WASM output has no parent directory"))
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Ok(Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Cargo build failed: {}", stderr),
        )));
    }

    // Cargo puts the output in target/wasm32-unknown-unknown/release/[crate_name].wasm
    let cargo_output_path = wasm_output.parent()
        .unwrap()
        .join("wasm32-unknown-unknown")
        .join("release")
        .join(format!("{}.wasm", crate_name));

    if cargo_output_path.exists() {
        fs::rename(&cargo_output_path, wasm_output)?;
        Ok(Ok(()))
    } else {
        Ok(Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Cargo didn't produce expected output at {}", cargo_output_path.display()),
        )))
    }
}

fn compile_with_rustc(source_path: &Path, wasm_output: &Path) -> io::Result<()> {
    let mut cmd = Command::new("rustc");
    cmd.arg(source_path)
        .arg("--target=wasm32-unknown-unknown")
        .arg("--crate-type=cdylib")
        .arg("-C")
        .arg("opt-level=3")  // Equivalent to --release optimizations
        .arg("-C")
        .arg("debug-assertions=off")
        .arg("-C")
        .arg("lto")  // Link-time optimization
        .arg("-o")
        .arg(wasm_output);

    let output = cmd.output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to compile {}: {}", source_path.display(), stderr),
        ));
    }

    if !wasm_output.exists() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("WASM file was not created: {}", wasm_output.display()),
        ));
    }

    Ok(())
}

fn needs_update(source_path: &Path, target_path: &Path) -> io::Result<bool> {
    if !target_path.exists() {
        return Ok(true);
    }

    let source_modified = fs::metadata(source_path)?.modified()?;
    let target_modified = fs::metadata(target_path)?.modified()?;

    Ok(source_modified > target_modified)
}
