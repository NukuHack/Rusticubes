
use crate::ui::manager::UIState;
use crate::game::state;
use crate::ext::config;
use crate::network::api;

pub fn join_world(world_name: &str) {
    println!("Loading world: {}", world_name);

    state::start_world(&world_name);
    let ui_manager = &mut config::get_state().ui_manager;
    ui_manager.state = UIState::Loading;
    ui_manager.setup_ui();
    
    ui_manager.state = UIState::Escape;
    ui_manager.setup_ui();
}

pub fn create_world(world_name: String) {
    // Create the save path
    let save_path = config::get_save_path()
        .join("saves")
        .join(world_name);
	
	state::make_world(save_path);
}

pub fn join_local_world(world_name: &str) {
	println!("joining world : {}", world_name);
}

pub fn leave_world() {
    let state = config::get_state();
    state.is_world_running = false;

    config::drop_gamestate();
    if api::is_host() == Ok(true) {
        api::cleanup_network();
    }
}