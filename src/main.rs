
use std::{
    io::*,
    future::Future,
    task::{Context, Poll, Waker},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    env,
};

use test_app::run;


fn main() {
    initialize();
    let mut lock = stdout().lock();
    write!(lock, "Begin code:\n\n\n").unwrap();

    run_app();

    write!(lock, "\n\nEnd code:").unwrap();
}

// Custom Waker implementation
struct MyWaker {
    flag: AtomicBool,
}

impl std::task::Wake for MyWaker {
    fn wake(self: Arc<Self>) {
        self.flag.store(true, Ordering::Relaxed);
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.flag.store(true, Ordering::Relaxed);
    }
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
    // Initialize the future
    let future = run();
    let mut future = Box::pin(future);

    // Create the custom Waker
    let my_waker = Arc::new(MyWaker {
        flag: AtomicBool::new(false),
    });
    let waker = Waker::from(my_waker.clone());
    let mut context = Context::from_waker(&waker);

    loop {
        // Check if the Waker was triggered
        if my_waker.flag.swap(false, Ordering::Relaxed) {
            // Poll again immediately
            if let Poll::Ready(_) = future.as_mut().poll(&mut context) {
                break;
            }
        }

        // Poll the future
        match future.as_mut().poll(&mut context) {
            Poll::Ready(()) => break,
            Poll::Pending => {
                // Continue to next frame
            }
        }
    }
}
