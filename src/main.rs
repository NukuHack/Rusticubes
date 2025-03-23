use std::io::{stdout, Write};
use test_app::run;
use std::env;
use std::cmp::*;

fn main() {
    initialize();

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
    pollster::block_on(run());
    return;
}
