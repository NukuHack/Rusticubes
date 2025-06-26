
use super::ui_manager::UIState;
#[inline]
pub fn join_world(world_name: &str) {
    println!("Loading world: {}", world_name);

    super::game_state::start_world(&world_name);
    let state = super::config::get_state();
    state.ui_manager.state = UIState::Loading;
    state.ui_manager.setup_ui();
    state.ui_manager.state = UIState::InGame;
    state.ui_manager.setup_ui();
}
#[inline]
pub fn del_world(world_name: &str) {
	match super::world_manager::del_world(&world_name) {
	    Ok(_) => {
	        println!("Successfully deleted world '{}'", world_name);
	        let state = super::config::get_state();
	        state.ui_manager.setup_ui();
	    },
	    Err(e) => panic!("Failed to delete world '{}': {}", world_name, e),
	}
}