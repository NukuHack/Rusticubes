use std::env;
use std::fs;
use std::io;
use std::path::Path;

fn main() -> io::Result<()> {
    // Tell Cargo to rerun this script if anything in the res/ directory changes
    println!("cargo:rerun-if-changed=resources/*");

    // Get the output directory from the environment variable
    let out_dir = env::var("OUT_DIR")
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let out_dir_path = Path::new(&out_dir);

    // Define source and destination paths
    let folder:&str = "resources";
    let source_dir = Path::new(folder);
    let destination_dir = out_dir_path.join(folder);
    println!("source {}", &source_dir.display());
    println!("destination {:?}", &destination_dir.display());

    // Copy the directory recursively
    copy_directory(source_dir, &destination_dir)?;

    Ok(())
}

// Recursively copies a directory and its contents to a new location
fn copy_directory(src: &Path, dst: &Path) -> io::Result<()> {
    // Create the destination directory if it doesn't exist
    fs::create_dir_all(dst)?;

    // Iterate over each entry in the source directory
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let filename = entry.file_name();
        let dst_path = dst.join(&filename);

        // Determine if we're dealing with a file or directory
        if entry.file_type()?.is_dir() {
            // Recursively copy subdirectories
            copy_directory(&path, &dst_path)?;
        } else {
            // Copy files directly
            fs::copy(&path, &dst_path)?;
        }
    }

    Ok(())
}