

use crate::ui_manager::UIState;



pub fn join_world(world_name: &str) {
    println!("Loading world: {}", world_name);

    super::start_world(&world_name);
    let state = super::config::get_state();
    state.ui_manager.state = UIState::Loading;
    state.ui_manager.setup_ui();
    state.ui_manager.state = UIState::InGame;
    state.ui_manager.setup_ui();

}


pub fn del_world(world_name: &str) {
	match super::file_manager::del_world(&world_name) {
	    Ok(_) => {
	        println!("Successfully deleted world '{}'", world_name);
	        let state = super::config::get_state();
	        state.ui_manager.setup_ui();
	    },
	    Err(e) => panic!("Failed to delete world '{}': {}", world_name, e),
	}
}