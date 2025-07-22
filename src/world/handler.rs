
use crate::world::manager::get_save_path;
use crate::ui::manager::UIState;
use crate::game::state;
use crate::network::api;
use crate::ext::ptr;

pub fn join_world(world_name: &str) {
	println!("Loading world: {}", world_name);

	state::start_world(&world_name);
	let ui_manager = &mut ptr::get_state().ui_manager;
	ui_manager.state = UIState::Loading;
	ui_manager.setup_ui();
	
	ui_manager.state = UIState::Escape;
	ui_manager.setup_ui();
}

pub fn create_world(world_name: String) {
	// Create the save path
	let save_path = get_save_path()
		.join("saves")
		.join(world_name);
	
	state::make_world(save_path);
}

pub fn join_local_world(world_name: &str) {
	println!("joining world : {}", world_name);
}

pub fn leave_world() {
	let state = ptr::get_state();
	state.is_world_running = false;

	ptr::drop_gamestate();
	if api::is_host() == Ok(true) {
		api::cleanup_network();
	}
}