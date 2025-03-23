
use std::io::*;
use std::future::Future;
use std::task::{Context, Poll, Waker};
use std::env;

use test_app::run;

fn main() {
    //initialize();
    let mut lock = stdout().lock();
    write!(lock, "Begin code:\n\n\n").unwrap();

    run_app();

    write!(lock, "\n\nEnd code:").unwrap();
}
fn initialize(){
    unsafe {
        // Disable Vulkan layers to avoid errors from missing files
        env::set_var("VK_LAYER_PATH", ""); // Ignore custom layer paths
        env::set_var("VK_INSTANCE_LAYERS", ""); // Disable all instance layers
        env::set_var("VK_DEVICE_LAYERS", ""); // Disable all device layers (optional)
        env::set_var("VK_LAYER_DISABLE", "EOSOverlayVkLayer;bdcamvk");
    }
    return;
}
fn run_app(){
    let future = run();
    // Convert the future into a pinned box
    let mut future = Box::pin(future);
    // Create a dummy Waker (needed for Context)
    let waker = Waker::noop();
    let mut context = Context::from_waker(&waker);
    // Poll the future in a loop (e.g., inside your game frame loop)
    loop {
        // Poll the future
        match future.as_mut().poll(&mut context) {
            Poll::Ready(()) => {
                // The future completed
                break;
            },
            Poll::Pending => {
                // The future is not ready yet, continue polling next frame
            }
        }
    }
}
