
use crate::network::api;
use crate::block;
use crate::ext::{config, memory};
use crate::world::{handler, manager};
use crate::ui::manager::{UIState, close_pressed, UIManager, UIStateID, get_element_data_dy_id};
use crate::ui::element::UIElement;

impl UIManager {
	#[inline]
	pub fn setup_ui(&mut self) {
		self.clear_elements();

		let bg_panel = UIElement::panel(self.next_id())
			.with_position(-1.0, -1.0)
			.with_size(2.0, 2.0)
			.with_color(15, 15, 25)  // Darker background
			.with_z_index(-5);

		#[allow(unreachable_patterns)]
		match self.state {
			UIState::None => {
				self.state = UIState::BootScreen;
				self.setup_ui();
			},
			UIState::BootScreen => {
				self.add_element(bg_panel);
				self.setup_boot_screen_ui();
			},
			UIState::WorldSelection => {
				self.add_element(bg_panel);
				self.setup_world_selection_ui();
			},
			UIState::Loading => {
				self.add_element(bg_panel);
				self.setup_loading_screen_ui();
			},
			UIState::NewWorld => {
				self.add_element(bg_panel);
				self.setup_new_world_ui();
			},
			UIState::InGame => {
				self.setup_in_game_ui();
			},
			UIState::Multiplayer => {
				self.add_element(bg_panel);
				self.setup_multiplayer_ui();
			},
			UIState::Settings(..) => {
				self.add_element(bg_panel);
				self.setup_settings_ui();
			},
			UIState::Escape => {
				self.setup_escape_ui();
			},
			UIState::Confirm(..) => {
				self.add_element(bg_panel);
				self.setup_confirm_ui();
			},
			UIState::Error(..) => {
				self.add_element(bg_panel);
				self.setup_error_ui();
			},
			UIState::ConnectLocal => {
				self.add_element(bg_panel);
				self.setup_connect_local_ui();
			},
			UIState::Inventory(_) => {
				self.setup_inventory_ui();
			}
			_ => {},
		}
	}

	#[inline]
	fn setup_boot_screen_ui(&mut self) {
		// Main title with gradient effect
		let title = UIElement::label(self.next_id(), "Rusticubes")
			.with_position(-0.4, 0.3)
			.with_size(0.8, 0.2)
			.with_color(30, 30, 45)  // Dark panel
			.with_text_color(180, 180, 220) // Light blue-gray text
			.with_border((80, 80, 120, 255), 0.008)
			.with_z_index(10);
		self.add_element(title);

		// Button container panel with subtle glow
		let button_panel = UIElement::panel(self.next_id())
			.with_position(-0.35, -0.2)
			.with_size(0.7, 0.5)
			.with_color(25, 25, 40)  // Dark blue-gray
			.with_border((60, 60, 90, 255), 0.008)
			.with_z_index(1);
		self.add_element(button_panel);

		// Start button with hover-friendly colors
		let start_button = UIElement::button(self.next_id(), "Start")
			.with_position(-0.15, 0.0)
			.with_size(0.3, 0.1)
			.with_color(40, 80, 140)  // Deep blue
			.with_text_color(200, 220, 255) // Light blue
			.with_border((70, 110, 180, 255), 0.005)
			.with_z_index(6)
			.with_callback(|| {
				let ui_manager = &mut config::get_state().ui_manager;
				ui_manager.state = UIState::WorldSelection;
				ui_manager.setup_ui();
			});
		self.add_element(start_button);

		// Exit button with more contrast
		let exit_button = UIElement::button(self.next_id(), "Exit")
			.with_position(-0.15, -0.15)
			.with_size(0.3, 0.1)
			.with_color(120, 40, 40)  // Dark red
			.with_text_color(255, 180, 180) // Light red
			.with_border((160, 60, 60, 255), 0.005)
			.with_z_index(5)
			.with_callback(|| {
				close_pressed();
			});
		self.add_element(exit_button);

		let memory_button = UIElement::button(self.next_id(), "Memory")
			.with_position(0.55, 0.2)
			.with_size(0.35, 0.1)
			.with_color(40, 40, 60)  // Dark gray-blue
			.with_text_color(150, 170, 200) // Light blue-gray
			.with_border((70, 90, 120, 255), 0.005)
			.with_z_index(6)
			.with_callback(|| {
				memory::light_trim();
				memory::hard_clean(Some(config::get_state().device()));
			});
		self.add_element(memory_button);

		let setting_button = UIElement::button(self.next_id(), "Settings")
			.with_position(-0.9, 0.0)
			.with_size(0.4, 0.1)
			.with_color(40, 40, 60)  // Dark gray-blue
			.with_text_color(150, 170, 200) // Light blue-gray
			.with_border((70, 90, 120, 255), 0.005)
			.with_z_index(6)
			.with_callback(|| {
				let ui_manager = &mut config::get_state().ui_manager;
				ui_manager.state = UIState::Settings(UIStateID::from(&ui_manager.state));
				ui_manager.setup_ui();
			});
		self.add_element(setting_button);

		let multiplayer_button = UIElement::button(self.next_id(), "Multi")
			.with_position(0.55, -0.1)
			.with_size(0.35, 0.1)
			.with_color(40, 40, 60)  // Dark gray-blue
			.with_text_color(150, 170, 200) // Light blue-gray
			.with_border((70, 90, 120, 255), 0.005)
			.with_z_index(6)
			.with_callback(|| {
				let state = config::get_state();
				state.ui_manager.state = UIState::Multiplayer;
				state.ui_manager.setup_ui();
				match api::begin_online_search() {
					 Ok(o) => println!("worked: {}", o),
					 Err(e) => println!("not worked: {}", e),
				};
			});
		self.add_element(multiplayer_button);

		// Decorative elements
		let tree_picture = UIElement::image(self.next_id(), "happy-tree.png")
			.with_position(0.6, 0.5)
			.with_size(0.27, 0.45)
			.with_color(255, 255, 255)
			.with_border((80, 120, 180, 255), 0.008)
			.with_z_index(6);
		self.add_element(tree_picture);

		let tree_animation = UIElement::animation(self.next_id(), vec![
				"happy-tree.png",
				"cube.jpg"
			])
			.with_position(-0.8, 0.5)
			.with_size(0.27, 0.45)
			.with_color(255, 255, 255)
			.with_border((80, 120, 180, 255), 0.008)
			.with_z_index(6)
			.with_smooth_transition(true)
			.with_animation_duration(2.5);
		self.add_element(tree_animation);

		// Version label with subtle styling
		let version = UIElement::label(self.next_id(), format!("v{}", env!("CARGO_PKG_VERSION")))
			.with_position(0.7, -0.95)
			.with_size(0.2, 0.05)
			.with_color(30, 30, 45)  // Dark panel
			.with_text_color(120, 140, 180) // Blue-gray text
			.with_border((60, 80, 120, 127), 0.003)
			.with_z_index(8);
		self.add_element(version);
	}

	#[inline]
	fn setup_world_selection_ui(&mut self) {
		// Title with improved styling
		let title = UIElement::label(self.next_id(), "Select World")
			.with_position(-0.4, 0.6)
			.with_size(0.8, 0.15)
			.with_color(30, 30, 45)  // Dark panel
			.with_text_color(180, 200, 220) // Light blue text
			.with_border((80, 100, 140, 255), 0.008)
			.with_z_index(10);
		self.add_element(title);

		// World list container with better contrast
		let list_panel = UIElement::panel(self.next_id())
			.with_position(-0.6, -0.4)
			.with_size(1.2, 0.9)  // Slightly shorter
			.with_color(25, 25, 40)  // Dark blue-gray
			.with_border((60, 70, 100, 255), 0.01)
			.with_z_index(1);
		self.add_element(list_panel);

		// New World button with consistent styling
		let new_button = UIElement::button(self.next_id(), "Create New World")
			.with_position(-0.3, 0.4)
			.with_size(0.6, 0.08)
			.with_color(50, 70, 110)  // Medium blue
			.with_text_color(180, 200, 220) // Light blue text
			.with_border((80, 110, 160, 255), 0.005)
			.with_z_index(8)
			.with_callback(|| {
				let state = config::get_state();
				state.ui_manager.state = UIState::NewWorld;
				state.ui_manager.setup_ui();
			});
		self.add_element(new_button);

		let worlds:Vec<String> = match manager::get_world_names() {
			Ok(worlds) => worlds,
			Err(e) => {
				println!("Error loading world names: {}", e);
				Vec::new()
			}
		};

		// World buttons with improved styling
		for (i, name) in worlds.iter().enumerate() {
			let y_pos = 0.2 - (i as f32 * 0.12);
			let name_clone = name.clone();  // Clone once here

			let world_button = UIElement::button(self.next_id(), name)
				.with_position(-0.4, y_pos)
				.with_size(0.8, 0.1)
				.with_color(40, 50, 80)
				.with_text_color(180, 200, 220)
				.with_border((70, 90, 130, 255), 0.005)
				.with_z_index(5)
				.with_callback({
					let name_clone = name_clone.clone();  // Clone for this closure
					move || {
						handler::join_world(&name_clone);
					}
				});
			self.add_element(world_button);

			// Delete button with more contrast
			let delete_button = UIElement::button(self.next_id(), "X")
				.with_position(0.43, y_pos)
				.with_size(0.1, 0.1)
				.with_color(100, 40, 40)
				.with_text_color(255, 180, 180)
				.with_border((150, 60, 60, 255), 0.005)
				.with_z_index(5)
				.with_callback(move || {
					let name_clone = name_clone.clone();
					// Non-blocking dialog with callback
					config::get_state().ui_manager.dialogs.ask_with_callback(
						"Delete world?",
						move |confirmed| {
							if confirmed {
								manager::del_world(&name_clone);
							}
						}
					);
				});
			self.add_element(delete_button);
		}


		// Back button with consistent styling
		let back_button = UIElement::button(self.next_id(), "Back")
			.with_position(-0.1, -0.8)
			.with_size(0.2, 0.08)
			.with_color(60, 60, 80)  // Dark gray-blue
			.with_text_color(180, 190, 210) // Light blue-gray
			.with_border((90, 100, 130, 255), 0.005)
			.with_z_index(8)
			.with_callback(|| {
				let state = config::get_state();
				state.ui_manager.state = UIState::BootScreen;
				state.ui_manager.setup_ui();
			});
		self.add_element(back_button);
	}

	#[inline]
	fn setup_settings_ui(&mut self) {
		// Title with improved styling
		let title = UIElement::label(self.next_id(), "Settings ... yah")
			.with_position(-0.4, 0.6)
			.with_size(0.8, 0.15)
			.with_color(30, 30, 45)  // Dark panel
			.with_text_color(180, 200, 220) // Light blue text
			.with_border((80, 100, 140, 255), 0.008)
			.with_z_index(10);
		self.add_element(title);

		// World list container with better contrast
		let list_panel = UIElement::panel(self.next_id())
			.with_position(-0.6, -0.4)
			.with_size(1.2, 0.9)  // Slightly shorter
			.with_color(25, 25, 40)  // Dark blue-gray
			.with_border((60, 70, 100, 255), 0.01)
			.with_z_index(1);
		self.add_element(list_panel);

		let setting_button_1 = UIElement::button(self.next_id(), "setting")
			.with_position(-0.4, 0.1)
			.with_size(0.8, 0.1)
			.with_color(40, 50, 80)
			.with_text_color(180, 200, 220)
			.with_border((70, 90, 130, 255), 0.005)
			.with_z_index(5)
			.with_callback(|| {
				println!("clicked setting_button_1");
			});
		self.add_element(setting_button_1);

		let setting_checkbox_1 = UIElement::checkbox(self.next_id(), Some("checkbox"))
			.with_position(-0.4, -0.02)
			.with_size(0.08, 0.1)
			.with_color(40, 50, 80)
			.with_text_color(180, 200, 220)
			.with_border((70, 90, 130, 255), 0.005)
			.with_z_index(5)
			.with_callback(|| {
				println!("clicked setting_checkbox_1");
			});
		self.add_element(setting_checkbox_1);

		let setting_slider_1 = UIElement::slider(self.next_id(), 0., 100.)
			.with_position(-0.4, -0.15)
			.with_size(0.8, 0.1)
			.with_color(40, 50, 80)
			.with_text_color(180, 200, 220)
			.with_border((70, 90, 130, 255), 0.005)
			.with_z_index(5)
			.with_step(0.5)
			.with_value(10.)
			.with_callback(|| {
				println!("clicked setting_slider_1");
			});
		self.add_element(setting_slider_1);

		let setting_multi_button_1 = UIElement::multi_state_button(self.next_id(), vec!("On", "Off"))
			.with_position(-0.4, -0.3)
			.with_size(0.8, 0.1)
			.with_color(40, 50, 80)
			.with_text_color(180, 200, 220)
			.with_border((70, 90, 130, 255), 0.005)
			.with_z_index(5)
			.with_callback(|| {
				println!("clicked setting_multi_button_1");
			});
		self.add_element(setting_multi_button_1);

		// Back button with consistent styling
		let back_button = UIElement::button(self.next_id(), "Back")
			.with_position(-0.1, -0.8)
			.with_size(0.2, 0.08)
			.with_color(60, 60, 80)  // Dark gray-blue
			.with_text_color(180, 190, 210) // Light blue-gray
			.with_border((90, 100, 130, 255), 0.005)
			.with_z_index(8)
			.with_callback(|| close_pressed());
		self.add_element(back_button);
	}

	#[inline]
	fn setup_confirm_ui(&mut self) {
		// Title with improved styling
		let manager = &config::get_state().ui_manager;
		let dialog_id = manager.state.inner().unwrap_or(0);
		let prompt:String = manager.dialogs.get_pending_dialog(dialog_id).unwrap_or("Yeah?".to_string());
		let title = UIElement::label(self.next_id(), prompt)
			.with_position(-0.4, 0.6)
			.with_size(0.8, 0.15)
			.with_color(30, 30, 45)  // Dark panel
			.with_text_color(180, 200, 220) // Light blue text
			.with_border((80, 100, 140, 255), 0.008)
			.with_z_index(10);
		self.add_element(title);

		// World list container with better contrast
		let list_panel = UIElement::panel(self.next_id())
			.with_position(-0.9, -0.4)
			.with_size(1.8, 0.9)  // Slightly shorter
			.with_color(25, 25, 40)  // Dark blue-gray
			.with_border((60, 70, 100, 255), 0.01)
			.with_z_index(1);
		self.add_element(list_panel);

		let option_button_1 = UIElement::button(self.next_id(), "Yes")
			.with_position(-0.8, 0.0)
			.with_size(0.6, 0.1)
			.with_color(40, 50, 80)
			.with_text_color(180, 200, 220)
			.with_border((70, 90, 130, 255), 0.005)
			.with_z_index(5)
			.with_callback(move || {
				let ui_manager = &mut config::get_state().ui_manager;
				ui_manager.dialogs.respond(dialog_id.clone(), true);
				ui_manager.state = ui_manager.state.inner_state();
				ui_manager.setup_ui();
			});
		self.add_element(option_button_1);
		let option_button_2 = UIElement::button(self.next_id(), "NO")
			.with_position(0.2, 0.0)
			.with_size(0.6, 0.1)
			.with_color(40, 50, 80)
			.with_text_color(180, 200, 220)
			.with_border((70, 90, 130, 255), 0.005)
			.with_z_index(5)
			.with_callback(move || {
				let ui_manager = &mut config::get_state().ui_manager;
				ui_manager.dialogs.respond(dialog_id.clone(), false);
				ui_manager.state = ui_manager.state.inner_state();
				ui_manager.setup_ui();
			});
		self.add_element(option_button_2);

		// Back button with consistent styling
		let back_button = UIElement::button(self.next_id(), "Back")
			.with_position(-0.1, -0.8)
			.with_size(0.2, 0.08)
			.with_color(60, 60, 80)  // Dark gray-blue
			.with_text_color(180, 190, 210) // Light blue-gray
			.with_border((90, 100, 130, 255), 0.005)
			.with_z_index(8)
			.with_callback(|| close_pressed());
		self.add_element(back_button);
	}

	#[inline]
	fn setup_error_ui(&mut self) {
		// Title with improved styling
		let manager = &config::get_state().ui_manager;
		let dialog_id = manager.state.inner().unwrap_or(0);
		let prompt:String = manager.dialogs.get_pending_dialog(dialog_id).unwrap_or("ERROR!!".to_string());
		let title = UIElement::label(self.next_id(), prompt)
			.with_position(-0.4, 0.6)
			.with_size(0.8, 0.15)
			.with_color(30, 30, 45)  // Dark panel
			.with_text_color(180, 200, 220) // Light blue text
			.with_border((80, 100, 140, 255), 0.008)
			.with_z_index(10);
		self.add_element(title);

		// World list container with better contrast
		let list_panel = UIElement::panel(self.next_id())
			.with_position(-0.9, -0.4)
			.with_size(1.8, 0.9)  // Slightly shorter
			.with_color(25, 25, 40)  // Dark blue-gray
			.with_border((60, 70, 100, 255), 0.01)
			.with_z_index(1);
		self.add_element(list_panel);

		let option_button_1 = UIElement::button(self.next_id(), "Continue")
			.with_position(-0.8, 0.0)
			.with_size(0.6, 0.1)
			.with_color(40, 50, 80)
			.with_text_color(180, 200, 220)
			.with_border((70, 90, 130, 255), 0.005)
			.with_z_index(5)
			.with_callback(move || {
				let ui_manager = &mut config::get_state().ui_manager;
				ui_manager.dialogs.respond(dialog_id.clone(), true);
				ui_manager.state = ui_manager.state.inner_state();
				ui_manager.setup_ui();
			});
		self.add_element(option_button_1);
		let option_button_2 = UIElement::button(self.next_id(), "Cancel")
			.with_position(0.2, 0.0)
			.with_size(0.6, 0.1)
			.with_color(40, 50, 80)
			.with_text_color(180, 200, 220)
			.with_border((70, 90, 130, 255), 0.005)
			.with_z_index(5)
			.with_callback(move || {
				let ui_manager = &mut config::get_state().ui_manager;
				ui_manager.dialogs.respond(dialog_id.clone(), false);
				ui_manager.state = ui_manager.state.inner_state();
				ui_manager.setup_ui();
			});
		self.add_element(option_button_2);

		// Back button with consistent styling
		let back_button = UIElement::button(self.next_id(), "Back")
			.with_position(-0.1, -0.8)
			.with_size(0.2, 0.08)
			.with_color(60, 60, 80)  // Dark gray-blue
			.with_text_color(180, 190, 210) // Light blue-gray
			.with_border((90, 100, 130, 255), 0.005)
			.with_z_index(8)
			.with_callback(|| close_pressed());
		self.add_element(back_button);
	}

	#[inline]
	fn setup_multiplayer_ui(&mut self) {
		// Title with improved styling
		let title = UIElement::label(self.next_id(), "Select World")
			.with_position(-0.4, 0.6)
			.with_size(0.8, 0.15)
			.with_color(30, 30, 45)  // Dark panel
			.with_text_color(180, 200, 220) // Light blue text
			.with_border((80, 100, 140, 255), 0.008)
			.with_z_index(10);
		self.add_element(title);

		// World list container with better contrast
		let list_panel = UIElement::panel(self.next_id())
			.with_position(-0.6, -0.4)
			.with_size(1.2, 0.9)  // Slightly shorter
			.with_color(25, 25, 40)  // Dark blue-gray
			.with_border((60, 70, 100, 255), 0.01)
			.with_z_index(1);
		self.add_element(list_panel);

		let worlds:Vec<String> = api::get_discovered_hosts()
			.iter()
			.map(|s| s.world_name.clone())
			.collect();

		// World buttons with improved styling
		for (i, name) in worlds.iter().enumerate() {
			let y_pos = 0.2 - (i as f32 * 0.12);
			let name_clone = name.clone();  // Clone once here

			let world_button = UIElement::button(self.next_id(), name)
				.with_position(-0.4, y_pos)
				.with_size(0.8, 0.1)
				.with_color(40, 50, 80)
				.with_text_color(180, 200, 220)
				.with_border((70, 90, 130, 255), 0.005)
				.with_z_index(5)
				.with_callback({
					move || {
						handler::join_local_world(&name_clone);
					}
				});
			self.add_element(world_button);
		}

		let re_button = UIElement::button(self.next_id(), "refresh")
			.with_position(-0.4, -0.8)
			.with_size(0.25, 0.08)
			.with_color(60, 60, 80)  // Dark gray-blue
			.with_text_color(180, 190, 210) // Light blue-gray
			.with_border((90, 100, 130, 255), 0.005)
			.with_z_index(8)
			.with_callback(|| {
				match api::refresh_discovery() {
					Ok(a) => { println!("Refresh: {}", a); },
					Err(e) => { println!("Refresh error: {}", e); },
				}
				let state = config::get_state();
				state.ui_manager.setup_ui();
			});
		self.add_element(re_button);

		let connect_button = UIElement::button(self.next_id(), "manual connect")
			.with_position(0.15, -0.8)
			.with_size(0.4, 0.08)
			.with_color(60, 60, 80)  // Dark gray-blue
			.with_text_color(180, 190, 210) // Light blue-gray
			.with_border((90, 100, 130, 255), 0.005)
			.with_z_index(8)
			.with_callback(|| {
				let ui_manager = &mut config::get_state().ui_manager;
				ui_manager.state = UIState::ConnectLocal;
				ui_manager.setup_ui();
			});
		self.add_element(connect_button);

		// Back button with consistent styling
		let back_button = UIElement::button(self.next_id(), "Back")
			.with_position(-0.1, -0.8)
			.with_size(0.2, 0.08)
			.with_color(60, 60, 80)  // Dark gray-blue
			.with_text_color(180, 190, 210) // Light blue-gray
			.with_border((90, 100, 130, 255), 0.005)
			.with_z_index(8)
			.with_callback(|| close_pressed());
		self.add_element(back_button);
	}

	#[inline]
	fn setup_new_world_ui(&mut self) {
		// Title with improved styling
		let title = UIElement::label(self.next_id(), "Create New World")
			.with_position(-0.5, 0.4)
			.with_size(1.0, 0.15)
			.with_color(30, 30, 45)  // Dark panel
			.with_text_color(180, 200, 220) // Light blue text
			.with_border((80, 100, 140, 255), 0.008)
			.with_z_index(10);
		self.add_element(title);

		// Form panel with better contrast
		let form_panel = UIElement::panel(self.next_id())
			.with_position(-0.4, -0.3)
			.with_size(0.8, 0.7)
			.with_color(25, 25, 40)  // Dark blue-gray
			.with_border((60, 70, 100, 255), 0.01)
			.with_z_index(1);
		self.add_element(form_panel);

		// World name label
		let w_name_label = UIElement::label(self.next_id(), "World Name:")
			.with_position(-0.35, 0.1)
			.with_size(0.4, 0.08)
			.with_color(30, 30, 45)  // Dark panel
			.with_text_color(180, 200, 220) // Light blue text
			.with_z_index(3);
		self.add_element(w_name_label);

		// World Name input with better styling
		let input_id = self.next_id();
		let world_name_input = UIElement::input(input_id)
			.with_position(-0.35, -0.0)
			.with_size(0.7, 0.1)
			.with_color(40, 50, 70)  // Dark blue-gray
			.with_text_color(200, 220, 240) // Light blue text
			.with_placeholder("New World")
			.with_border((80, 100, 140, 255), 0.005)
			.with_z_index(5);
		self.add_element(world_name_input);

		// Generate button with consistent styling
		let gen_button = UIElement::button(self.next_id(), "Create World")
			.with_position(-0.3, -0.2)
			.with_size(0.6, 0.1)
			.with_color(50, 70, 110)  // Medium blue
			.with_text_color(180, 200, 220) // Light blue text
			.with_border((80, 110, 160, 255), 0.005)
			.with_z_index(6)
			.with_callback(move || {
				handler::create_world(get_element_data_dy_id(input_id));
				let ui_manager = &mut config::get_state().ui_manager;
				ui_manager.state = UIState::WorldSelection;
				ui_manager.setup_ui();
			});
		self.add_element(gen_button);

		// Back button with consistent styling
		let back_button = UIElement::button(self.next_id(), "Back")
			.with_position(-0.1, -0.45)
			.with_size(0.2, 0.08)
			.with_color(60, 60, 80)  // Dark gray-blue
			.with_text_color(180, 190, 210) // Light blue-gray
			.with_border((90, 100, 130, 255), 0.005)
			.with_z_index(8)
			.with_callback(|| close_pressed());
		self.add_element(back_button);
	}

	#[inline]
	fn setup_connect_local_ui(&mut self) {
		// Title with improved styling
		let title = UIElement::label(self.next_id(), "Manual Connect")
			.with_position(-0.5, 0.4)
			.with_size(1.0, 0.15)
			.with_color(30, 30, 45)  // Dark panel
			.with_text_color(180, 200, 220) // Light blue text
			.with_border((80, 100, 140, 255), 0.008)
			.with_z_index(10);
		self.add_element(title);

		// Form panel with better contrast
		let form_panel = UIElement::panel(self.next_id())
			.with_position(-0.4, -0.3)
			.with_size(0.8, 0.7)
			.with_color(25, 25, 40)  // Dark blue-gray
			.with_border((60, 70, 100, 255), 0.01)
			.with_z_index(1);
		self.add_element(form_panel);

		// World name label
		let w_name_label = UIElement::label(self.next_id(), "Server IP:")
			.with_position(-0.35, 0.1)
			.with_size(0.4, 0.08)
			.with_color(30, 30, 45)  // Dark panel
			.with_text_color(180, 200, 220) // Light blue text
			.with_z_index(3);
		self.add_element(w_name_label);

		// World Name input with better styling
		let input_id = self.next_id();
		let world_name_input = UIElement::input(input_id)
			.with_position(-0.35, -0.0)
			.with_size(0.7, 0.1)
			.with_color(40, 50, 70)  // Dark blue-gray
			.with_text_color(200, 220, 240) // Light blue text
			.with_placeholder("255.255.255.255")
			.with_border((80, 100, 140, 255), 0.005)
			.with_z_index(5);
		self.add_element(world_name_input);

		// Generate button with consistent styling
		let gen_button = UIElement::button(self.next_id(), "Connect Server")
			.with_position(-0.3, -0.2)
			.with_size(0.6, 0.1)
			.with_color(50, 70, 110)  // Medium blue
			.with_text_color(180, 200, 220) // Light blue text
			.with_border((80, 110, 160, 255), 0.005)
			.with_z_index(6)
			.with_callback(move || {
				match api::connect_to_host(&get_element_data_dy_id(input_id)) {
					Ok(a) => { println!("Nice: {}", a); },
					Err(e) => { println!("Error: {}", e); },
				}
				;
				let ui_manager = &mut config::get_state().ui_manager;
				ui_manager.state = UIState::WorldSelection;
				ui_manager.setup_ui();
			});
		self.add_element(gen_button);

		// Back button with consistent styling
		let back_button = UIElement::button(self.next_id(), "Back")
			.with_position(-0.1, -0.45)
			.with_size(0.2, 0.08)
			.with_color(60, 60, 80)  // Dark gray-blue
			.with_text_color(180, 190, 210) // Light blue-gray
			.with_border((90, 100, 130, 255), 0.005)
			.with_z_index(8)
			.with_callback(|| close_pressed());
		self.add_element(back_button);
	}
	#[inline]
	fn setup_loading_screen_ui(&mut self) {
		// Loading panel with better contrast
		let loading_panel = UIElement::panel(self.next_id())
			.with_position(-0.3, -0.1)
			.with_size(0.6, 0.2)
			.with_color(20, 20, 35)  // Very dark blue
			.with_border((60, 80, 120, 255), 0.01)
			.with_z_index(10);
		self.add_element(loading_panel);

		// Loading text with better visibility
		let loading_text = UIElement::label(self.next_id(), "Loading...")
			.with_position(-0.25, -0.05)
			.with_size(0.5, 0.1)
			.with_color(20, 20, 35)  // Dark panel
			.with_text_color(180, 200, 220) // Light blue text
			.with_z_index(15);
		self.add_element(loading_text);

		// Progress bar background
		let progress_bg = UIElement::panel(self.next_id())
			.with_position(-0.25, -0.15)
			.with_size(0.5, 0.03)
			.with_color(15, 20, 30)  // Very dark
			.with_border((40, 60, 90, 255), 0.005)
			.with_z_index(8);
		self.add_element(progress_bg);

		// Progress bar with brighter color
		let progress_bar = UIElement::panel(self.next_id())
			.with_position(-0.245, -0.145)
			.with_size(0.01, 0.02)
			.with_color(80, 140, 220)  // Bright blue
			.with_z_index(8);
		self.add_element(progress_bar);
	}

	#[inline]
	fn setup_escape_ui(&mut self) {
		let bg_panel = UIElement::panel(self.next_id())
			.with_position(-1.0, -1.0)
			.with_size(2.0, 2.0)
			.with_color(0, 0, 0)
			.with_alpha(40)
			.with_z_index(-5);
		self.add_element(bg_panel);

		let save_button = UIElement::button(self.next_id(), "Save World")
			.with_position(-0.8, 0.15)
			.with_size(0.4, 0.08)
			.with_color(40, 40, 60)  // Dark gray-blue
			.with_text_color(150, 170, 200) // Light blue-gray
			.with_border((70, 90, 120, 255), 0.005)
			.with_z_index(8)
			.with_callback(|| 
				{
					let save_path = config::get_gamestate().save_path();
					match manager::save_entire_world(save_path) {
						Ok(_) => {},
						Err(e) => { println!("Error: {}", e); },
					};
				});
		self.add_element(save_button);
		let load_button = UIElement::button(self.next_id(), "Load World")
			.with_position(-0.8, 0.0)
			.with_size(0.4, 0.08)
			.with_color(40, 40, 60)  // Dark gray-blue
			.with_text_color(150, 170, 200) // Light blue-gray
			.with_border((70, 90, 120, 255), 0.005)
			.with_z_index(8)
			.with_callback(|| 
				{
					let save_path = config::get_gamestate().save_path();
					match manager::load_entire_world(save_path) {
						Ok(_) => {},
						Err(e) => { println!("Error: {}", e); },
					};
				});
		self.add_element(load_button);
		let setting_button = UIElement::button(self.next_id(), "Settings")
			.with_position(-0.8, -0.15)
			.with_size(0.4, 0.08)
			.with_color(40, 40, 60)  // Dark gray-blue
			.with_text_color(150, 170, 200) // Light blue-gray
			.with_border((70, 90, 120, 255), 0.005)
			.with_z_index(6)
			.with_callback(|| {
				let ui_manager = &mut config::get_state().ui_manager;
				ui_manager.state = UIState::Settings(UIStateID::from(&ui_manager.state));
				ui_manager.setup_ui();
			});
		self.add_element(setting_button);

		let memory_button = UIElement::button(self.next_id(), "Memory")
			.with_position(-0.8, -0.3)
			.with_size(0.4, 0.08)
			.with_color(40, 40, 60)
			.with_text_color(150, 170, 200)
			.with_border((70, 90, 120, 255), 0.005)
			.with_z_index(6)
			.with_callback(|| {
				memory::light_trim();
				memory::hard_clean(Some(config::get_state().device()));
			});
		self.add_element(memory_button);

		// Side panel with better contrast
		let side_panel = UIElement::panel(self.next_id())
			.with_position(0.4, -0.9)
			.with_size(0.6, 1.8)
			.with_color(20, 20, 35)  // Dark blue-gray
			.with_border((50, 60, 90, 255), 0.01)
			.with_z_index(1);
		self.add_element(side_panel);

		// Panel title with improved styling
		let panel_title = UIElement::label(self.next_id(), "Game Menu")
			.with_position(0.45, 0.75)
			.with_size(0.5, 0.1)
			.with_color(30, 30, 45)  // Dark panel
			.with_text_color(180, 200, 220) // Light blue text
			.with_border((80, 100, 140, 255), 0.008)
			.with_z_index(10);
		self.add_element(panel_title);

		// Clean world button with better contrast
		let clean_button = UIElement::button(self.next_id(), "Clean World")
			.with_position(0.45, 0.4)
			.with_size(0.5, 0.1)
			.with_color(90, 50, 50)  // Dark reddish
			.with_text_color(255, 180, 180) // Light red
			.with_border((140, 80, 80, 255), 0.005)
			.with_z_index(8)
			.with_callback(|| block::extra::add_full_world());
		self.add_element(clean_button);

		let host_button = UIElement::button(self.next_id(), "Host World")
			.with_position(0.5, 0.22)
			.with_size(0.4, 0.08)
			.with_color(40, 40, 60)  // Dark gray-blue
			.with_text_color(150, 170, 200) // Light blue-gray
			.with_border((70, 90, 120, 255), 0.005)
			.with_z_index(8)
			.with_callback(|| 
				{
					match api::begin_online_giveaway() {
						Ok(o) => println!("worked: {}", o),
						Err(e) => println!("not worked: {}", e),
					};
				});
		self.add_element(host_button);

		// Help text with better contrast
		let help_texts = [
			("ALT to lock", 0.1),
			("L to fill chunk", -0.05),
			("R to break", -0.2),
			("F to place", -0.35),
			("ESC to leave", -0.5)
		];

		for (_i, (text, y_pos)) in help_texts.iter().enumerate() {
			let help_text = UIElement::label(self.next_id(), *text)
				.with_position(0.5, *y_pos)
				.with_size(0.4, 0.08)
				.with_color(30, 30, 45)  // Dark panel
				.with_text_color(180, 200, 220) // Light blue text
				.with_border((80, 100, 140, 255), 0.005)
				.with_z_index(5);
			self.add_element(help_text);
		}

		// Close button with better contrast
		let back_button = UIElement::button(self.next_id(), "Back to World")
			.with_position(0.5, -0.8)
			.with_size(0.4, 0.08)
			.with_color(30, 30, 45)  // Dark panel
			.with_text_color(180, 200, 220) // Light blue text
			.with_border((70, 90, 120, 255), 0.005)
			.with_z_index(8)
			.with_callback(|| close_pressed());
		self.add_element(back_button);

		// Close button with better contrast
		let close_button = UIElement::button(self.next_id(), "Quit World")
			.with_position(-0.2, -0.8)
			.with_size(0.4, 0.08)
			.with_color(120, 40, 40)  // Dark red
			.with_text_color(220, 180, 180) // Light red
			.with_border((160, 60, 60, 255), 0.005)
			.with_z_index(8)
			.with_callback(|| {
				let state = config::get_state();
				state.ui_manager.state = UIState::BootScreen;
				state.ui_manager.setup_ui();

				handler::leave_world();
			});
		self.add_element(close_button);

	}

	#[inline]
	fn setup_in_game_ui(&mut self) {
		// Close button with better contrast
		let close_button = UIElement::button(self.next_id(), "X")
			.with_position(0.92, -0.92)
			.with_size(0.08, 0.08)
			.with_color(120, 40, 40)  // Dark red
			.with_text_color(220, 180, 180) // Light red
			.with_border((160, 60, 60, 255), 0.005)
			.with_z_index(8)
			.with_callback(|| close_pressed());
		self.add_element(close_button);

		// Cross-hair with better visibility
		let crosshair_v = UIElement::divider(self.next_id())
			.with_position(0.0, -0.02)
			.with_size(0.02, 0.06)
			.with_color(220, 240, 255)  // Bright white
			.with_z_index(20);
		let crosshair_h = UIElement::divider(self.next_id())
			.with_position(-0.02, 0.0)
			.with_size(0.06, 0.02)
			.with_color(220, 240, 255)  // Bright white
			.with_z_index(20);

		self.add_element(crosshair_v);
		self.add_element(crosshair_h);
	}
}