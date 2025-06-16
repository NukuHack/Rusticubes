use std::fs;
use std::path::{Path, PathBuf};

fn main() {
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

            match needs_compression(path, &compressed_path) {
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
            "All files up-to-date ({} files, {} already compressed)",
            total_files, up_to_date_files
        );
        return;
    }

    println!(
        "Processing {} of {} files ({} up-to-date)...",
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
        "Compression completed: {} processed, {} skipped, {} up-to-date",
        processed_count, skipped_count, up_to_date_files
    );
}

fn process_file(source_path: &Path, compressed_path: &Path) -> std::io::Result<()> {
    // Read source file
    let data = fs::read(source_path)?;

    // Compress the data
    let compressed = lz4_flex::compress_prepend_size(&data);

    // Create parent directories if needed
    if let Some(parent) = compressed_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write compressed file
    fs::write(compressed_path, compressed)?;

    // Preserve original modification time if possible
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

fn needs_compression(source_path: &Path, compressed_path: &Path) -> std::io::Result<bool> {
    // If compressed file doesn't exist, we need to compress
    if !compressed_path.exists() {
        return Ok(true);
    }

    // Get modification times
    let source_modified = fs::metadata(source_path)?.modified()?;
    let compressed_modified = fs::metadata(compressed_path)?.modified()?;

    // Need to compress if source is newer
    Ok(source_modified > compressed_modified)
}

fn add_extension(path: PathBuf, extension: &str) -> PathBuf {
    let mut os_string = path.into_os_string();
    os_string.push(".");
    os_string.push(extension);
    PathBuf::from(os_string)
}
