use std::{
    env,
    fs,
    io::{self, Result},
    path::{Path, PathBuf},
};

const RESOURCES_DIR: &str = "resources";

fn main() -> Result<()> {
    let source_dir = Path::new(RESOURCES_DIR);
    if !source_dir.exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Resources directory not found: {}", source_dir.display()),
        ));
    }

    // Collect all files in resources directory
    let files = walk_files(source_dir)?;
    for file in &files {
        println!("cargo:rerun-if-changed={}", file.display());
    }

    // Get output directory
    let out_dir = env::var("OUT_DIR")
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let destination_dir = Path::new(&out_dir).join(RESOURCES_DIR);
    println!(
        "Copying resources from {} to {}",
        source_dir.display(),
        destination_dir.display()
    );

    // Perform recursive copy
    copy_directory(source_dir, &destination_dir)?;

    Ok(())
}

/// Iteratively collects all files in a directory and its subdirectories
fn walk_files<P: AsRef<Path>>(path: P) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut stack = vec![path.as_ref().to_path_buf()];

    while let Some(current) = stack.pop() {
        if current.is_dir() {
            for entry in fs::read_dir(&current)? {
                let entry = entry?;
                let path = entry.path();
                if entry.file_type()?.is_dir() {
                    stack.push(path);
                } else {
                    files.push(path);
                }
            }
        } else {
            files.push(current);
        }
    }

    Ok(files)
}

/// Recursively copies a directory and its contents
fn copy_directory(src: &Path, dst: &Path) -> Result<()> {
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