
use crate::ui::manager::UIState;
use crate::game::state;
use crate::config;

pub fn join_world(world_name: &str) {
    println!("Loading world: {}", world_name);

    state::start_world(&world_name);
    let ui_manager = &mut config::get_state().ui_manager;
    ui_manager.state = UIState::Loading;
    ui_manager.setup_ui();
    
    ui_manager.state = UIState::Escape;
    ui_manager.setup_ui();
}

pub fn create_world(id: usize) {
	let world_name = config::get_state()
	    .ui_manager()
	    .get_input_text(id)
	    .map(|s| s.trim())  // Trim whitespace first
	    .filter(|s| !s.is_empty())  // Reject empty strings after trim
	    .map(|s| s.to_string())
	    .unwrap_or_else(|| {
	        // You might want to log this fallback behavior
	        "New World".to_string()
	    });
    join_world(&world_name);
}

pub fn join_local_world(world_name: &str) {
	println!("joining world : {}", world_name);
}