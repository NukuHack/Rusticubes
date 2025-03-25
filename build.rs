
use std::{
    env,
    fs,
    io,
    path::Path,
    path::PathBuf,
};

fn main() -> io::Result<()> {
    const RESOURCES_DIR: &str = "resources";

    // Tell Cargo to rerun if any file in resources changes
    let source_dir = Path::new(RESOURCES_DIR);
    for file in walk_files(source_dir)? {
        println!("cargo:rerun-if-changed={}", file.display());
    }

    // Get the output directory from the environment variable
    let out_dir = env::var("OUT_DIR")
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let out_dir_path = Path::new(&out_dir);

    let destination_dir = out_dir_path.join(RESOURCES_DIR);
    println!("source {}", source_dir.display());
    println!("destination {}", destination_dir.display());

    // Copy the directory recursively
    copy_directory(source_dir, &destination_dir)?;

    Ok(())
}

// Helper function to recursively list all files in a directory
fn walk_files<P: AsRef<Path>>(path: P) -> io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let path = path.as_ref();
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();
            if entry.file_type()?.is_dir() {
                files.append(&mut walk_files(&path)?);
            } else {
                files.push(path);
            }
        }
    } else {
        files.push(path.to_path_buf());
    }
    Ok(files)
}

// Recursively copies a directory and its contents to a new location
fn copy_directory(src: &Path, dst: &Path) -> io::Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let filename = entry.file_name();
        let dst_path = dst.join(&filename);

        if entry.file_type()?.is_dir() {
            copy_directory(&path, &dst_path)?;
        } else {
            fs::copy(&path, &dst_path)?;
        }
    }

    Ok(())
}