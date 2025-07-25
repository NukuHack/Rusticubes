use crate::ptr::get_settings;
use crate::hs::math;
use rodio::{Sink, Decoder, OutputStream, source::Source};
use std::io::{Cursor, Write};
use std::sync::atomic::{AtomicPtr, Ordering};
use std::ptr;

/// Audio system structure with multiple sinks
struct AudioSystem {
	bg_sink: Sink,         // For background music
	fg_sink: Sink,         // For UI sounds
	_stream: OutputStream, // Keep stream alive
}

static AUDIO_SYSTEM_PTR: AtomicPtr<AudioSystem> = AtomicPtr::new(ptr::null_mut());

/// Helper function to safely access the AudioSystem pointer
#[inline]
fn get_audio_system() -> Option<&'static mut AudioSystem> {
	let system_ptr = AUDIO_SYSTEM_PTR.load(Ordering::Acquire);
	if system_ptr.is_null() {
		None
	} else {
		unsafe { Some(&mut *system_ptr) }
	}
}

/// Initialize the audio system
#[inline]
pub fn init_audio() -> Result<(), Box<dyn std::error::Error>> {
	let (stream, stream_handle) = OutputStream::try_default()?;
	let music_settings = &get_settings().music_settings;
	
	// Create separate sinks for different audio types
	let bg_sink = Sink::try_new(&stream_handle)?;
	let fg_sink = Sink::try_new(&stream_handle)?;
	
	// Set different volume levels for each sink
	bg_sink.set_volume(music_settings.bg_volume.val * music_settings.main_volume.val); 
	fg_sink.set_volume(music_settings.fg_volume.val * music_settings.main_volume.val); 
	
	let system = Box::new(AudioSystem {
		bg_sink,
		fg_sink,
		_stream: stream,
	});
	
	let old_ptr = AUDIO_SYSTEM_PTR.swap(Box::into_raw(system), Ordering::AcqRel);
	if !old_ptr.is_null() {
		unsafe { let _ = Box::from_raw(old_ptr); }
	}
	
	Ok(())
}

/// Set a new background sound with looping - falls back to terminal ping on error
pub fn set_bg<T: Into<String>>(path: T) {
	let path = path.into();
	
	match get_audio_system() {
		Some(system) => {
			system.bg_sink.stop();
			
			match try_play_bg_sound(path, system) {
				Ok(_duration) => {
					// Successfully started background audio
				}
				Err(_) => {
					// Failed to play audio, use terminal ping as fallback
					play_terminal_ping();
				}
			}
		}
		None => {
			// Audio system not initialized, use terminal ping
			play_terminal_ping();
		}
	}
}

/// Play a one-time UI sound - falls back to terminal ping on error
pub fn set_fg<T: Into<String>>(path: T) {
	let path = path.into();
	
	match get_audio_system() {
		Some(system) => {
			system.fg_sink.stop();
			
			match try_play_fg_sound(path, system) {
				Ok(_duration) => {
					// Successfully played foreground sound
				}
				Err(_) => {
					// Failed to play audio, use terminal ping as fallback
					play_terminal_ping();
				}
			}
		}
		None => {
			// Audio system not initialized, use terminal ping
			play_terminal_ping();
		}
	}
}

/// Play background sound with speed and randomization settings
#[inline]
fn try_play_bg_sound(
	path: String, 
	system: &mut AudioSystem
) -> Result<std::time::Duration, Box<dyn std::error::Error>> {
	let sound_bytes = crate::get_bytes!(path);
	let cursor = Cursor::new(sound_bytes);
	let source = Decoder::new(cursor)?;
	
	let original_duration = source.total_duration()
		.unwrap_or(std::time::Duration::from_secs(1));

	let music_settings = &get_settings().music_settings;
	let speed = calculate_playback_speed(
		music_settings.bg_speed.val * music_settings.main_speed.val,
		music_settings.use_random,
		music_settings.random_value
	);
	
	let source = source.speed(speed);
	let duration = original_duration.div_f32(speed);
	
	// Apply looping and append to background sink
	let source: Box<dyn Source<Item = i16> + Send> = 
		Box::new(source.repeat_infinite());
	
	system.bg_sink.append(source);
	
	Ok(duration)
}

/// Play foreground sound with speed and randomization settings
#[inline]
fn try_play_fg_sound(
	path: String, 
	system: &mut AudioSystem, 
) -> Result<std::time::Duration, Box<dyn std::error::Error>> {
	let sound_bytes = crate::get_bytes!(path);
	let cursor = Cursor::new(sound_bytes);
	let source = Decoder::new(cursor)?;
	
	let original_duration = source.total_duration()
		.unwrap_or(std::time::Duration::from_secs(1));

	let music_settings = &get_settings().music_settings;
	let speed = calculate_playback_speed(
		music_settings.fg_speed.val * music_settings.main_speed.val,
		music_settings.use_random,
		music_settings.random_value
	);
	
	let source = source.speed(speed);
	let duration = original_duration.div_f32(speed);
	
	// No looping for foreground sounds
	let source: Box<dyn Source<Item = i16> + Send> = Box::new(source);
	
	system.fg_sink.append(source);
	
	Ok(duration)
}

/// Calculate playback speed with optional randomization
#[inline]
fn calculate_playback_speed(base_speed: f32, use_random: bool, random_value: f32) -> f32 {
	if use_random {
		let random_factor = math::random_float(1.0 - random_value, 1.0 + random_value);
		base_speed * random_factor
	} else {
		base_speed
	}
}

/// Play terminal bell sound as fallback
#[inline]
fn play_terminal_ping() {
	print!("\x07");
	let _ = std::io::stdout().flush();
}

// Volume control functions
#[inline]
pub fn set_bg_volume(volume: f32) {
	if let Some(system) = get_audio_system() {
		system.bg_sink.set_volume(volume);
	}
}

#[inline]
pub fn set_fg_volume(volume: f32) {
	if let Some(system) = get_audio_system() {
		system.fg_sink.set_volume(volume);
	}
}

// Playback control functions
#[inline]
pub fn stop_bg() {
	if let Some(system) = get_audio_system() {
		system.bg_sink.stop();
	}
}

#[inline]
pub fn stop_fg() {
	if let Some(system) = get_audio_system() {
		system.fg_sink.stop();
	}
}

#[inline]
pub fn stop_all_sounds() {
	stop_bg();
	stop_fg();
}

/// Clean up audio system resources
#[inline]
pub fn cleanup_audio() {
	let system_ptr = AUDIO_SYSTEM_PTR.swap(ptr::null_mut(), Ordering::AcqRel);
	if !system_ptr.is_null() {
		unsafe { let _ = Box::from_raw(system_ptr); }
	}
}