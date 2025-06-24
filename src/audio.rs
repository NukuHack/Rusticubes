use rodio::{Sink, Decoder, OutputStream, source::Source};
use std::io::{Cursor, Write};
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;

// Audio system structure with multiple sinks
struct AudioSystem {
    bg_sink: Sink,      // For background music
    ui_sink: Sink,      // For UI sounds
    _stream: OutputStream, // Keep stream alive
}

static AUDIO_SYSTEM_PTR: AtomicPtr<AudioSystem> = AtomicPtr::new(ptr::null_mut());

// Helper function to safely access the AudioSystem pointer
fn get_ptr() -> Option<&'static mut AudioSystem> {
    let system_ptr = AUDIO_SYSTEM_PTR.load(Ordering::Acquire);
    if system_ptr.is_null() {
        None
    } else {
        unsafe { Some(&mut *system_ptr) }
    }
}

pub fn init_audio() -> Result<(), Box<dyn std::error::Error>> {
    let (stream, stream_handle) = OutputStream::try_default()?;
    
    // Create separate sinks for different audio types
    let bg_sink = Sink::try_new(&stream_handle)?;
    let ui_sink = Sink::try_new(&stream_handle)?;
    
    // Set different volume levels for each sink
    bg_sink.set_volume(0.3);  // Lower volume for background music
    ui_sink.set_volume(0.5);  // Higher volume for UI sounds
    
    let system = Box::new(AudioSystem {
        bg_sink,
        ui_sink,
        _stream: stream,
    });
    
    let old_ptr = AUDIO_SYSTEM_PTR.swap(Box::into_raw(system), Ordering::AcqRel);
    if !old_ptr.is_null() {
        unsafe { let _ = Box::from_raw(old_ptr); }
    }
    
    Ok(())
}

// Play background music (longer sounds, loops, etc.)
pub fn play_background(path: String) {
    if let Some(system) = get_ptr() {
        match try_play_sound(path.clone(), system, true) {
            Ok(_duration) => {}
            Err(e) => {
                println!("Failed to play background '{}': {}", path, e);
            }
        }
    } else {
        println!("Audio system not initialized");
    }
}

// Play UI sound (short sounds, immediate playback)
pub fn play_sound(path: String) {
    if let Some(system) = get_ptr() {
        match try_play_sound(path.clone(), system, false) {
            Ok(_duration) => {}
            Err(e) => {
                println!("Failed to play UI sound '{}': {}. Falling back to terminal ping.", path, e);
                play_terminal_ping();
            }
        }
    } else {
        println!("Audio system not initialized, using terminal ping");
        play_terminal_ping();
    }
}

/// Set a new UI sound, stopping any currently playing UI sounds first
#[allow(dead_code)]
pub fn set_sound(path: String) {
    if let Some(system) = get_ptr() {
        // Clear all currently playing UI sounds
        system.ui_sink.stop();
        
        match try_play_sound(path.clone(), system, false) {
            Ok(_duration) => {}
            Err(e) => {
                println!("Failed to set UI sound '{}': {}. Falling back to terminal ping.", path, e);
                play_terminal_ping();
            }
        }
    } else {
        println!("Audio system not initialized, using terminal ping");
        play_terminal_ping();
    }
}

/// Set a new background sound, stopping any currently playing background music first
#[allow(dead_code)]
pub fn set_background(path: String) {
    if let Some(system) = get_ptr() {
        // Clear all currently playing background sounds
        system.bg_sink.stop();
        
        match try_play_sound(path.clone(), system, true) {
            Ok(_duration) => {}
            Err(e) => {
                println!("Failed to set background '{}': {}", path, e);
            }
        }
    } else {
        println!("Audio system not initialized");
    }
}

// Common sound playing function
fn try_play_sound(
    path: String, 
    system: &mut AudioSystem, 
    is_background: bool
) -> Result<std::time::Duration, Box<dyn std::error::Error>> {
    let sound_bytes = crate::get_bytes!(path.clone());
    let cursor = Cursor::new(sound_bytes);
    let source = Decoder::new(cursor)?;
    
    let volume = 1.0; // Volume is now controlled per-sink
    let source = source.amplify(volume);
    
    let speed = super::math::random_float(0.8, 0.9);
    let source = source.speed(speed);
    
    let original_duration = source.total_duration().unwrap_or(std::time::Duration::from_secs(1));
    let duration = original_duration.div_f32(speed);
    
    let sink = if is_background {
        &system.bg_sink
    } else {
        &system.ui_sink
    };
    sink.append(source);
    
    Ok(duration)
}

fn play_terminal_ping() {
    print!("\x07");
    let _ = std::io::stdout().flush();
}

// Control functions
#[allow(dead_code)]
pub fn set_background_volume(volume: f32) {
    if let Some(system) = get_ptr() {
        system.bg_sink.set_volume(volume);
    }
}

#[allow(dead_code)]
pub fn set_volume(volume: f32) {
    if let Some(system) = get_ptr() {
        system.ui_sink.set_volume(volume);
    }
}

pub fn stop_background() {
    if let Some(system) = get_ptr() {
        system.bg_sink.stop();
    }
}

pub fn stop_ui_sounds() {
    if let Some(system) = get_ptr() {
        system.ui_sink.stop();
    }
}

pub fn stop_all_sounds() {
    stop_background();
    stop_ui_sounds();
}

pub fn cleanup_audio() {
    let system_ptr = AUDIO_SYSTEM_PTR.swap(ptr::null_mut(), Ordering::AcqRel);
    if !system_ptr.is_null() {
        unsafe { let _ = Box::from_raw(system_ptr); }
    }
}