
use rodio::{source::Source, Decoder, OutputStream};
use std::io::{Cursor, Write};

pub fn play_sound(path : String) {
    match play_embedded_sound(path.clone()) {
        Ok(duration) => {
            println!(
                "Played '{}' for {:.2} seconds",
                path,
                duration.as_secs_f32()
            );
        }
        Err(e) => {
            println!(
                "Failed to play '{}': {}. Falling back to terminal ping.",
                path, e
            );
            play_terminal_ping();
        }
    }
}


fn play_embedded_sound(path: String) -> Result<std::time::Duration, Box<dyn std::error::Error>> {
    // Get the sound file data and store it in a variable to ensure it lives long enough
    let sound_bytes = crate::get_bytes!(path.clone());
    let sound_data: &[u8] = &sound_bytes;

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