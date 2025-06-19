use rodio::{source::Source, Decoder, OutputStream};
use std::io::{Cursor, Write};
use std::thread;

pub fn play_sound(path: String) {
    // Spawn a new thread to play the sound
    thread::spawn(move || {
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
    });
}

fn play_embedded_sound(path: String) -> Result<std::time::Duration, Box<dyn std::error::Error>> {
    // Get the sound file data and own it
    let sound_bytes = crate::get_bytes!(path.clone());
    // Get a output stream handle to the default physical sound device
    let (_stream, stream_handle) = OutputStream::try_default()?;
    // Create a cursor to read the sound data - now owning the bytes directly
    let cursor = Cursor::new(sound_bytes);
    // Decode the sound data into a source
    let source = Decoder::new(cursor)?;
    // Adjust the volume (0.5 = half volume, 1.0 = normal, 2.0 = double volume)
    let volume = 0.2;
    let source = source.amplify(volume);
    // Adjust the speed (0.5 = half speed - twice as long , 1.0 = normal, 2.0 = double speed - half as long)
    let speed = super::math::random_float(0.8,0.9);
    let source = source.speed(speed);
    // Get the total duration of the sound
    let original_duration = source.total_duration().unwrap_or(std::time::Duration::from_secs(1));
    let duration = original_duration.div_f32(speed);
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