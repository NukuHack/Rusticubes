use include_dir::include_dir;

// Include the assets directory at compile time
pub const RESOURCE_DIR: include_dir::Dir = include_dir!("$CARGO_MANIFEST_DIR/comp_resources");

// Macro to get raw bytes from included resources (equivalent to your old get_bytes!)
#[macro_export]
macro_rules! get_raw_data {
    ($path:expr) => {{
        use crate::resources::RESOURCE_DIR;
        RESOURCE_DIR
            .get_file($path)
            .map(|file| file.contents())
            .unwrap_or_else(|| panic!("File {} not found in embedded resources", $path))
    }};
}

// Updated get_bytes! macro that adds compression support while maintaining backward compatibility
#[macro_export]
macro_rules! get_bytes {
    ($path:expr) => {{
        use crate::resources::RESOURCE_DIR;
        // First try to find a compressed version
        if let Some(file) = RESOURCE_DIR.get_file(concat!($path, ".lz4")) {
            lz4_flex::decompress_size_prepended(file.contents())
                .unwrap_or_else(|e| panic!("Failed to decompress {}: {}", $path, e))
        } else {
            // Fall back to uncompressed version (original behavior)
            crate::get_raw_data!($path).to_vec()
        }
    }};
}

// Updated get_string! macro that works with both compressed and uncompressed resources
#[macro_export]
macro_rules! get_string {
    ($path:expr) => {{
        let bytes = $crate::get_bytes!($path);
        String::from_utf8(bytes)
            .unwrap_or_else(|e| panic!("File {} is not valid UTF-8: {}", $path, e))
    }};
}

use image::io::Reader as ImageReader;
use std::io::Cursor;
use winit::window::Icon;

pub fn load_icon_from_bytes() -> Option<Icon> {
    // Create a cursor to read from memory
    let reader = match ImageReader::new(Cursor::new(get_bytes!("icon.png")))
        .with_guessed_format()
        .map_err(|e| {
            println!("Failed to guess image format: {}", e);
            e
        }) {
        Ok(reader) => reader,
        Err(_) => return None,
    };

    let image = match reader.decode().map_err(|e| {
        println!("Failed to decode image: {}", e);
        e
    }) {
        Ok(img) => img,
        Err(_) => return None,
    };

    let rgba = image.into_rgba8();
    let (width, height) = rgba.dimensions();

    match Icon::from_rgba(rgba.into_raw(), width, height) {
        Ok(icon) => Some(icon),
        Err(e) => {
            println!("Failed to create icon from RGBA data: {}", e);
            None
        }
    }
}
