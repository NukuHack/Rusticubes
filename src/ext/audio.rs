
use crate::math;
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
#[inline]
fn get_ptr() -> Option<&'static mut AudioSystem> {
    let system_ptr = AUDIO_SYSTEM_PTR.load(Ordering::Acquire);
    if system_ptr.is_null() {
        None
    } else {
        unsafe { Some(&mut *system_ptr) }
    }
}
#[inline]
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


/// Set a new UI sound, stopping any currently playing UI sounds first
#[inline]
pub fn set_sound<T : Into<String>>(path: T) {
    let path: String = path.into();
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


/// Set a new background sound with looping
pub fn set_background<T : Into<String>>(path: T) {
    let path = path.into();
    if let Some(system) = get_ptr() {
        system.bg_sink.stop();
        
        if let Ok(source) = load_audio_source(path, true) {
            system.bg_sink.append(source);
        }
    }
}

/// Play a one-time UI sound
#[allow(dead_code)]
pub fn play_sound<T : Into<String>>(path: T) {
    let path = path.into();
    if let Some(system) = get_ptr() {
        if let Ok(source) = load_audio_source(path, false) {
            system.ui_sink.append(source);
        }
    }
}

/// Helper function to load and prepare audio source
fn load_audio_source(
    path: String,
    should_loop: bool
) -> Result<Box<dyn Source<Item = i16> + Send>, Box<dyn std::error::Error>> {
    let sound_bytes = crate::get_bytes!(path.clone());
    let cursor = Cursor::new(sound_bytes);
    let source = Decoder::new(cursor)?;
    
    let speed = math::random_float(0.8, 0.9);
    let source = source.speed(speed);
    
    let source: Box<dyn Source<Item = i16> + Send> = if should_loop {
        Box::new(source.repeat_infinite())
    } else {
        Box::new(source)
    };
    
    Ok(source)
}

// Common sound playing function
#[inline]
fn try_play_sound(
    path: String, 
    system: &mut AudioSystem, 
    is_background: bool
) -> Result<std::time::Duration, Box<dyn std::error::Error>> {
    let sound_bytes = crate::get_bytes!(path.clone());
    let cursor = Cursor::new(sound_bytes);
    let source = Decoder::new(cursor)?;
    
    let speed = math::random_float(0.8, 0.9);
    let source = source.speed(speed);
    
    let original_duration = source.total_duration().unwrap_or(std::time::Duration::from_secs(1));
    let duration = original_duration.div_f32(speed);
    
    let sink = if is_background {
        &system.bg_sink
    } else {
        &system.ui_sink
    };

    // Convert to a boxed source to handle different types uniformly
    let source: Box<dyn Source<Item = i16> + Send> = if is_background {
        Box::new(source.repeat_infinite())
    } else {
        Box::new(source)
    };
    
    sink.append(source);
    
    Ok(duration)
}
#[inline]
fn play_terminal_ping() {
    print!("\x07");
    let _ = std::io::stdout().flush();
}

// Control functions
#[inline]
#[allow(dead_code)]
pub fn set_background_volume(volume: f32) {
    if let Some(system) = get_ptr() {
        system.bg_sink.set_volume(volume);
    }
}
#[inline]
#[allow(dead_code)]
pub fn set_volume(volume: f32) {
    if let Some(system) = get_ptr() {
        system.ui_sink.set_volume(volume);
    }
}
#[inline]
pub fn stop_background() {
    if let Some(system) = get_ptr() {
        system.bg_sink.stop();
    }
}
#[inline]
pub fn stop_ui_sounds() {
    if let Some(system) = get_ptr() {
        system.ui_sink.stop();
    }
}
#[inline]
pub fn stop_all_sounds() {
    stop_background();
    stop_ui_sounds();
}
#[inline]
pub fn cleanup_audio() {
    let system_ptr = AUDIO_SYSTEM_PTR.swap(ptr::null_mut(), Ordering::AcqRel);
    if !system_ptr.is_null() {
        unsafe { let _ = Box::from_raw(system_ptr); }
    }
}