
use super::ui_manager::UIState;

pub fn join_world(world_name: &str) {
    println!("Loading world: {}", world_name);

    super::game_state::start_world(&world_name);
    let state = super::config::get_state();
    state.ui_manager.state = UIState::Loading;
    state.ui_manager.setup_ui();
    state.ui_manager.state = UIState::InGame;
    state.ui_manager.setup_ui();
}

pub fn try_join_world(id: usize) {
	let world_name = super::config::get_state()
	    .ui_manager()
	    .get_input_text(id)
	    .map(|s| s.trim())  // Trim whitespace first
	    .filter(|s| !s.is_empty())  // Reject empty strings after trim
	    .map(|s| s.to_string())
	    .unwrap_or_else(|| {
	        // You might want to log this fallback behavior
	        "New World".to_string()
	    });
    super::world_builder::join_world(&world_name);
}
