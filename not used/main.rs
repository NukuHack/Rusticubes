use include_dir::{include_dir, Dir};
use rodio::{source::Source, Decoder, OutputStream};
use std::io::{Cursor, Write};

// Include the assets directory at compile time
static ASSETS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/assets");

fn main() {
    println!("Sound player - type 'quit' to exit");

    loop {
        println!("\nEnter the name of the sound file you want to play (e.g., sound.ogg):");
        let mut file_name = String::new();
        std::io::stdin()
            .read_line(&mut file_name)
            .expect("Failed to read input");
        let file_name = file_name.trim();

        if file_name.eq_ignore_ascii_case("quit") {
            println!("Goodbye!");
            break;
        }

        match play_embedded_sound(file_name) {
            Ok(duration) => {
                println!(
                    "Played '{}' for {:.2} seconds",
                    file_name,
                    duration.as_secs_f32()
                );
            }
            Err(e) => {
                println!(
                    "Failed to play '{}': {}. Falling back to terminal ping.",
                    file_name, e
                );
                play_terminal_ping();
            }
        }
    }
}

fn play_embedded_sound(file_name: &str) -> Result<std::time::Duration, Box<dyn std::error::Error>> {
    // Get the sound file data
    let sound_data = get_texture(file_name)
        .ok_or_else(|| format!("Sound file '{}' not found in embedded assets", file_name))?;

    // Get a output stream handle to the default physical sound device
    let (_stream, stream_handle) = OutputStream::try_default()?;

    // Create a cursor to read the sound data
    let cursor = Cursor::new(sound_data);

    // Decode the sound data into a source
    let source = Decoder::new(cursor)?;

    // Get the total duration of the sound
    let duration = source
        .total_duration()
        .unwrap_or(std::time::Duration::from_secs(1));

    // Play the sound directly on the device
    let _ = stream_handle.play_raw(source.convert_samples());

    // Sleep while the sound plays (now dynamically based on duration)
    std::thread::sleep(duration);

    Ok(duration)
}

fn play_terminal_ping() {
    // This generates a simple beep sound in the console (works on most terminals)
    print!("\x07");
    let _ = std::io::stdout().flush();
}

// Access embedded files
fn get_texture(name: &str) -> Option<&'static [u8]> {
    ASSETS_DIR.get_file(name).map(|f| f.contents())
}
