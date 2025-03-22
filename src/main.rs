use std::io::{stdout, Write};
use test_app::run;
use std::env;
//use std::cmp::max;

struct Point{
    x: i32,
    y: i32,
}
struct Player {
    name: String,
    position: Point,
}
impl Player {
    fn log_pos(&self){
        print!("player pos: {};", self.position.x);
        println!("{}", self.position.y);
    }
}
fn make_player() -> Player {
    Player{name: String::from("Hero"), position:Point{x:0,y:0}}
}

fn main() {

    initialize();

    let mut lock = stdout().lock();
    let mut player:Player = make_player();
    write!(lock, "Begin code:\n{}\n\n", player.name).unwrap();

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
