use include_dir::include_dir;
// Include the assets directory at compile time
pub const RESOURCE_DIR: include_dir::Dir = include_dir!("$CARGO_MANIFEST_DIR/resources");

// Macro to mimic include_bytes! but using include_dir
#[macro_export]
macro_rules! get_bytes {
    ($path:expr) => {{
        // Get the file at compile time
        match crate::resources::RESOURCE_DIR.get_file($path) {
            Some(file) => file.contents(),
            None => panic!("File {} not found in embedded resources", $path),
        }
    }};
}

#[macro_export]
macro_rules! get_string {
    ($path:expr) => {{
        match std::str::from_utf8(match crate::resources::RESOURCE_DIR.get_file($path) {
            Some(file) => file.contents(),
            None => panic!("File {} not found in embedded resources", $path),
        }) {
            Ok(s) => s,
            Err(e) => panic!("File {} contents are not valid UTF-8: {}", $path, e),
        }
    }};
}

//get_bytes!("calibri.ttf");
//get_bytes!("happy-tree.png");

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
