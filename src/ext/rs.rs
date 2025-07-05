
use include_dir::include_dir;

// Include the assets directory at compile time
pub const RESOURCE_DIR: include_dir::Dir = include_dir!("$CARGO_MANIFEST_DIR/comp_resources");

// Macro to get raw bytes from included resources (equivalent to your old get_bytes!)
#[macro_export]
macro_rules! get_raw_data {
    ($path:expr) => {{
        use crate::ext::rs::RESOURCE_DIR;
        RESOURCE_DIR
            .get_file($path.clone())
            .map(|file| file.contents())
            .unwrap_or_else(|| panic!("File {} not found in embedded resources", $path))
    }};
}

// Updated get_bytes! macro that adds compression support while maintaining backward compatibility
#[macro_export]
macro_rules! get_bytes {
    ($path:expr) => {{
        use crate::ext::rs::RESOURCE_DIR;
        // First try to find a compressed version
        if let Some(file) = RESOURCE_DIR.get_file(format!("{}{}",$path, ".lz4")) {
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
#[inline]
pub fn load_icon_from_bytes() -> Option<Icon> {
    let Some((rgba,w,h)) = load_image_from_bytes("icon.png".to_string()) else { panic!() };

    match Icon::from_rgba(rgba, w, h) {
        Ok(icon) => Some(icon),
        Err(e) => {
            println!("Failed to create icon from RGBA data: {}", e);
            None
        }
    }
}

pub fn load_image_from_bytes(path: String) -> Option<(Vec<u8>,u32,u32)> {
    // Create a cursor to read from memory
    let reader_rgba = match ImageReader::new(Cursor::new(crate::get_bytes!(path.clone())))
        .with_guessed_format()
        .expect("Failed to guess format")
        .decode() 
    {
        Ok(img) => img.to_rgba8(),
        Err(e) => {
            println!("Failed to decode image: {}", e);
            return None;
        }
    };

    let (width, height) = reader_rgba.dimensions();

    // Convert to RGBA8 (16×16 = 1024 bytes)
    Some((reader_rgba.into_raw(),width,height))
}