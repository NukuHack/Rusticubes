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

            let compressed_path = comp_resource_path.join(relative_path);
            let compressed_path = add_extension(compressed_path, "lz4");

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
    let target_list = Command::new("rustc")
        .arg("--print")
        .arg("target-list")
        .output();

    if let Ok(output) = target_list {
        let targets = String::from_utf8_lossy(&output.stdout);
        if !targets.contains("wasm32-unknown-unknown") {
            eprintln!("ERROR: wasm32-unknown-unknown target not installed!");
            eprintln!("Install it with: rustup target add wasm32-unknown-unknown");
            return;
        } else {
            println!("DEBUG: wasm32-unknown-unknown target is available");
        }
    }
    // Check if rustc is available
    let rustc_version = Command::new("rustc").arg("--version").output();
    match rustc_version {
        Ok(output) => {
            println!("DEBUG: rustc version: {}", String::from_utf8_lossy(&output.stdout).trim());
        }
        Err(e) => {
            println!("rustc not found in PATH: {}", e);
            return;
        }
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

fn compile_plugin(source_path: &Path, wasm_output: &Path) -> io::Result<()> {
    println!("DEBUG: Compiling {} to {}", source_path.display(), wasm_output.display());
    
    // Try simple compilation first
    let mut cmd = Command::new("rustc");
    cmd.arg(source_path)
        .arg("--target=wasm32-unknown-unknown")
        .arg("--crate-type=cdylib")
        .arg("-O")
        .arg("-o")
        .arg(wasm_output);

    println!("DEBUG: Running command: {:?}", cmd);
    let output = cmd.output()?;

    println!("DEBUG: Command exit status: {}", output.status);
    
    if !output.stdout.is_empty() {
        println!("DEBUG: STDOUT: {}", String::from_utf8_lossy(&output.stdout));
    }
    
    if !output.stderr.is_empty() {
        println!("DEBUG: STDERR: {}", String::from_utf8_lossy(&output.stderr));
    }

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to compile {}: {}", source_path.display(), stderr),
        ));
    }

    // Check if the output file was actually created
    if wasm_output.exists() {
        let metadata = fs::metadata(wasm_output)?;
        println!("DEBUG: Successfully created WASM file: {} bytes", metadata.len());
    } else {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("WASM file was not created: {}", wasm_output.display()),
        ));
    }

    Ok(())
}

// Removed the temp wrapper function for now - we'll use simpler compilation

fn needs_update(source_path: &Path, target_path: &Path) -> io::Result<bool> {
    if !target_path.exists() {
        return Ok(true);
    }

    let source_modified = fs::metadata(source_path)?.modified()?;
    let target_modified = fs::metadata(target_path)?.modified()?;

    Ok(source_modified > target_modified)
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

fn add_extension(path: PathBuf, extension: &str) -> PathBuf {
    let mut os_string = path.into_os_string();
    os_string.push(".");
    os_string.push(extension);
    PathBuf::from(os_string)
}