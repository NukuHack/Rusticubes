use std::io::{stdout, Write};
use console_test::run;
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
    let mut lock = stdout().lock();

    let mut player:Player = make_player();

    write!(lock, "Begin code:\n{}\n\n", player.name).unwrap();


    initialize();

}
fn initialize(){

    pollster::block_on(run());
    return;
}