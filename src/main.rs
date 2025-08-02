#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use std::{
	env,
	future::Future,
	io::{stdout, Write},
	sync, task,
};

use rusticubes::run;

/// Entry point of the application
fn main() {
	initialize_vulkan();

	let mut output = stdout().lock();
	print_header(&mut output);
	
	run_app();

	print_footer(&mut output);
}

/// Disables Vulkan layers to prevent potential startup errors
#[inline]
fn initialize_vulkan() {
	unsafe {
		// Disable all Vulkan layers to avoid missing file errors
		env::set_var("VK_LAYER_PATH", ""); // Ignore custom layer paths
		env::set_var("VK_INSTANCE_LAYERS", ""); // Disable instance layers
		env::set_var("VK_DEVICE_LAYERS", ""); // Disable device layers
		env::set_var("VK_LAYER_DISABLE", "EOSOverlayVkLayer;");
	}
}

/// Prints application header
#[inline]
fn print_header(output: &mut impl Write) {
	writeln!(output, "\n\nBegin code:\n\n\n").unwrap();
}

/// Prints application footer
#[inline]
fn print_footer(output: &mut impl Write) {
	writeln!(output, "\n\nEnd code:\n\n").unwrap();
}

/// Custom waker implementation for manual future polling
struct ManualWaker {
	// Atomic flag to track if a wake signal has been received
	wake_flag: sync::atomic::AtomicBool,
}

impl std::task::Wake for ManualWaker {
	#[inline]
	fn wake(self: sync::Arc<Self>) {
		self.wake_flag.store(true, sync::atomic::Ordering::Relaxed);
	}

	#[inline]
	fn wake_by_ref(self: &sync::Arc<ManualWaker>) {
		self.wake_flag.store(true, sync::atomic::Ordering::Relaxed);
	}
}

/// Runs the application future until completion
#[inline]
fn run_app() {
	let future = run();
	let mut future = Box::pin(future);

	let waker = {
		let waker_instance = sync::Arc::new(ManualWaker {
			wake_flag: sync::atomic::AtomicBool::new(false),
		});
		let waker = task::Waker::from(waker_instance.clone());
		(waker, waker_instance)
	};

	let mut context = task::Context::from_waker(&waker.0);

	loop {
		// Check if we need to poll immediately due to wake signal
		if waker.1.wake_flag
			.swap(false, sync::atomic::Ordering::Relaxed)
		{
			if let task::Poll::Ready(_) = future.as_mut().poll(&mut context) {
				break;
			}
		}

		match future.as_mut().poll(&mut context) {
			task::Poll::Ready(_) => break,
			task::Poll::Pending => {
				// Task is pending - continue processing other work
				// In this manual loop, we just continue to the next iteration
			}
		}
	}
}
