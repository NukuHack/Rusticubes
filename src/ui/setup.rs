
use crate::network::api;
use crate::block;
use crate::ext::{ptr, memory, color::Solor};
use crate::world::{handler, manager};
use crate::ui::manager::{UIState, close_pressed, UIManager, UIStateID, get_element_data_dy_id};
use crate::ui::element::UIElement;

impl UIManager {
	#[inline]
	pub fn setup_ui(&mut self) {
		self.clear_elements();
		let theme = &ptr::get_settings().ui_theme;

		let bg_panel = UIElement::panel(self.next_id())
			.with_position(-1.0, -1.0)
			.with_size(2.0, 2.0)
			.with_style(&theme.bg_panel)
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
				self.setup_inventory_ui();
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
		let theme = &ptr::get_settings().ui_theme;
		// Main title
		let title = UIElement::label(self.next_id(), "Rusticubes")
			.with_position(-0.4, 0.3)
			.with_size(0.8, 0.2)
			.with_style(&theme.title_label)
			.with_z_index(10);
		self.add_element(title);

		// Button container panel
		let button_panel = UIElement::panel(self.next_id())
			.with_position(-0.35, -0.2)
			.with_size(0.7, 0.5)
			.with_style(&theme.panels.basic)
			.with_z_index(1);
		self.add_element(button_panel);

		// Start button
		let start_button = UIElement::button(self.next_id(), "Start")
			.with_position(-0.15, 0.0)
			.with_size(0.3, 0.1)
			.with_style(&theme.best_button)
			.with_z_index(6)
			.with_callback(|| {
				let ui_manager = &mut ptr::get_state().ui_manager;
				ui_manager.state = UIState::WorldSelection;
				ui_manager.setup_ui();
			});
		self.add_element(start_button);

		// Exit button
		let exit_button = UIElement::button(self.next_id(), "Exit")
			.with_position(-0.15, -0.15)
			.with_size(0.3, 0.1)
			.with_style(&theme.worst_button)
			.with_z_index(5)
			.with_callback(|| close_pressed());
		self.add_element(exit_button);

		let memory_button = UIElement::button(self.next_id(), "Memory")
			.with_position(0.55, 0.2)
			.with_size(0.35, 0.1)
			.with_style(&theme.buttons.nice)
			.with_z_index(6)
			.with_callback(|| {
				memory::light_trim();
				memory::hard_clean(Some(ptr::get_state().device()));
			});
		self.add_element(memory_button);

		let setting_button = UIElement::button(self.next_id(), "Settings")
			.with_position(-0.9, 0.0)
			.with_size(0.4, 0.1)
			.with_style(&theme.buttons.nice)
			.with_z_index(6)
			.with_callback(|| {
				let ui_manager = &mut ptr::get_state().ui_manager;
				ui_manager.state = UIState::Settings(UIStateID::from(&ui_manager.state));
				ui_manager.setup_ui();
			});
		self.add_element(setting_button);

		let multiplayer_button = UIElement::button(self.next_id(), "Multi")
			.with_position(0.55, -0.1)
			.with_size(0.35, 0.1)
			.with_style(&theme.buttons.nice)
			.with_z_index(6)
			.with_callback(|| {
				let state = ptr::get_state();
				state.ui_manager.state = UIState::Multiplayer;
				state.ui_manager.setup_ui();
				if let Err(e) = api::begin_online_search() {
					println!("not worked: {}", e)
				}
			});
		self.add_element(multiplayer_button);

		// Decorative elements
		let tree_picture = UIElement::image(self.next_id(), "happy-tree.png")
			.with_position(0.6, 0.5)
			.with_size(0.27, 0.45)
			.with_style(&theme.images.basic)
			.with_z_index(6);
		self.add_element(tree_picture);

		let tree_animation = UIElement::animation(self.next_id(), vec!["happy-tree.png", "cube.jpg"])
			.with_position(-0.8, 0.5)
			.with_size(0.27, 0.45)
			.with_style(&theme.images.basic)
			.with_z_index(6)
			.with_smooth_transition(true)
			.with_animation_duration(2.5);
		self.add_element(tree_animation);

		// Version label
		let version = UIElement::label(self.next_id(), format!("v{}", env!("CARGO_PKG_VERSION")))
			.with_position(0.7, -0.95)
			.with_size(0.2, 0.05)
			.with_style(&theme.labels.extra())
			.with_z_index(8);
		self.add_element(version);
	}

	#[inline]
	fn setup_world_selection_ui(&mut self) {
		let theme = &ptr::get_settings().ui_theme;
		// Title
		let title = UIElement::label(self.next_id(), "Select World")
			.with_position(-0.4, 0.6)
			.with_size(0.8, 0.15)
			.with_style(&theme.title_label)
			.with_z_index(10);
		self.add_element(title);

		// World list container
		let list_panel = UIElement::panel(self.next_id())
			.with_position(-0.6, -0.4)
			.with_size(1.2, 0.9)
			.with_style(&theme.panels.basic)
			.with_z_index(1);
		self.add_element(list_panel);

		// New World button
		let new_button = UIElement::button(self.next_id(), "Create New World")
			.with_position(-0.3, 0.4)
			.with_size(0.6, 0.08)
			.with_style(&theme.buttons.nice)
			.with_z_index(8)
			.with_callback(|| {
				let state = ptr::get_state();
				state.ui_manager.state = UIState::NewWorld;
				state.ui_manager.setup_ui();
			});
		self.add_element(new_button);

		let worlds: Vec<String> = match manager::get_world_names() {
			Ok(worlds) => worlds,
			Err(e) => {
				println!("Error loading world names: {}", e);
				Vec::new()
			}
		};

		// World buttons
		for (i, name) in worlds.iter().enumerate() {
			let y_pos = 0.2 - (i as f32 * 0.12);
			let name_clone = name.clone();

			let world_button = UIElement::button(self.next_id(), name)
				.with_position(-0.4, y_pos)
				.with_size(0.8, 0.1)
				.with_style(&theme.buttons.basic)
				.with_z_index(5)
				.with_callback({
					let name_clone = name_clone.clone();
					move || handler::join_world(&name_clone)
				});
			self.add_element(world_button);

			// Delete button
			let delete_button = UIElement::button(self.next_id(), "X")
				.with_position(0.43, y_pos)
				.with_size(0.1, 0.1)
				.with_style(&theme.buttons.bad)
				.with_z_index(5)
				.with_callback(move || {
					let name_clone = name_clone.clone();
					ptr::get_state().ui_manager.dialogs.ask_with_callback(
						"Delete world?",
						move |confirmed| {
							if confirmed { manager::del_world(&name_clone); }
						}
					);
				});
			self.add_element(delete_button);
		}

		// Back button
		let back_button = UIElement::button(self.next_id(), "Back")
			.with_position(-0.1, -0.8)
			.with_size(0.2, 0.08)
			.with_style(&theme.buttons.extra())
			.with_z_index(8)
			.with_callback(|| {
				let state = ptr::get_state();
				state.ui_manager.state = UIState::BootScreen;
				state.ui_manager.setup_ui();
			});
		self.add_element(back_button);
	}

	#[inline]
	fn setup_settings_ui(&mut self) {
		let theme = &ptr::get_settings().ui_theme;
		// Title
		let title = UIElement::label(self.next_id(), "Settings ... yah")
			.with_position(-0.4, 0.6)
			.with_size(0.8, 0.15)
			.with_style(&theme.title_label)
			.with_z_index(10);
		self.add_element(title);

		// Settings panel
		let list_panel = UIElement::panel(self.next_id())
			.with_position(-0.6, -0.4)
			.with_size(1.2, 0.9)
			.with_style(&theme.panels.basic)
			.with_z_index(1);
		self.add_element(list_panel);

		let setting_button_1 = UIElement::button(self.next_id(), "setting")
			.with_position(-0.4, 0.1)
			.with_size(0.8, 0.1)
			.with_style(&theme.buttons.basic)
			.with_z_index(5)
			.with_callback(|| println!("clicked setting_button_1"));
		self.add_element(setting_button_1);

		let setting_checkbox_1 = UIElement::checkbox(self.next_id(), Some("checkbox"))
			.with_position(-0.4, -0.02)
			.with_size(0.08, 0.1)
			.with_style(&theme.checkboxs.basic)
			.with_z_index(5)
			.with_callback(|| println!("clicked setting_checkbox_1"));
		self.add_element(setting_checkbox_1);

		let setting_slider_1 = UIElement::slider(self.next_id(), 0., 100.)
			.with_position(-0.4, -0.15)
			.with_size(0.8, 0.1)
			.with_style(&theme.sliders.basic)
			.with_z_index(5)
			.with_step(0.5)
			.with_value(10.)
			.with_callback(|| println!("clicked setting_slider_1"));
		self.add_element(setting_slider_1);

		let setting_multi_button_1 = UIElement::multi_state_button(self.next_id(), vec!("On", "Off"))
			.with_position(-0.4, -0.3)
			.with_size(0.8, 0.1)
			.with_style(&theme.buttons.basic)
			.with_z_index(5)
			.with_callback(|| println!("clicked setting_multi_button_1"));
		self.add_element(setting_multi_button_1);

		// Back button
		let back_button = UIElement::button(self.next_id(), "Back")
			.with_position(-0.1, -0.8)
			.with_size(0.2, 0.08)
			.with_style(&theme.buttons.extra())
			.with_z_index(8)
			.with_callback(|| close_pressed());
		self.add_element(back_button);
	}

	#[inline]
	fn setup_confirm_ui(&mut self) {
		let theme = &ptr::get_settings().ui_theme;
		let manager = &ptr::get_state().ui_manager;
		let dialog_id = manager.state.inner().unwrap_or(0);
		let prompt: String = manager.dialogs.get_pending_dialog(dialog_id).unwrap_or("Yeah?".to_string());
		
		let title = UIElement::label(self.next_id(), prompt)
			.with_position(-0.4, 0.6)
			.with_size(0.8, 0.15)
			.with_style(&theme.title_label)
			.with_z_index(10);
		self.add_element(title);

		let list_panel = UIElement::panel(self.next_id())
			.with_position(-0.9, -0.4)
			.with_size(1.8, 0.9)
			.with_style(&theme.panels.basic)
			.with_z_index(1);
		self.add_element(list_panel);

		let option_button_1 = UIElement::button(self.next_id(), "Yes")
			.with_position(-0.8, 0.0)
			.with_size(0.6, 0.1)
			.with_style(&theme.deny_button)
			.with_z_index(5)
			.with_callback(move || {
				let ui_manager = &mut ptr::get_state().ui_manager;
				ui_manager.dialogs.respond(dialog_id.clone(), true);
				ui_manager.state = ui_manager.state.inner_state();
				ui_manager.setup_ui();
			});
		self.add_element(option_button_1);
		
		let option_button_2 = UIElement::button(self.next_id(), "No")
			.with_position(0.2, 0.0)
			.with_size(0.6, 0.1)
			.with_style(&theme.okay_button)
			.with_z_index(5)
			.with_callback(move || {
				let ui_manager = &mut ptr::get_state().ui_manager;
				ui_manager.dialogs.respond(dialog_id.clone(), false);
				ui_manager.state = ui_manager.state.inner_state();
				ui_manager.setup_ui();
			});
		self.add_element(option_button_2);

		let back_button = UIElement::button(self.next_id(), "Back")
			.with_position(-0.1, -0.8)
			.with_size(0.2, 0.08)
			.with_style(&theme.buttons.extra())
			.with_z_index(8)
			.with_callback(|| close_pressed());
		self.add_element(back_button);
	}

	#[inline]
	fn setup_error_ui(&mut self) {
		let theme = &ptr::get_settings().ui_theme;
		let manager = &ptr::get_state().ui_manager;
		let dialog_id = manager.state.inner().unwrap_or(0);
		let prompt: String = manager.dialogs.get_pending_dialog(dialog_id).unwrap_or("ERROR!!".to_string());
		
		let title = UIElement::label(self.next_id(), prompt)
			.with_position(-0.4, 0.6)
			.with_size(0.8, 0.15)
			.with_style(&theme.title_label)
			.with_z_index(10);
		self.add_element(title);

		let list_panel = UIElement::panel(self.next_id())
			.with_position(-0.9, -0.4)
			.with_size(1.8, 0.9)
			.with_style(&theme.panels.basic)
			.with_z_index(1);
		self.add_element(list_panel);

		let option_button_1 = UIElement::button(self.next_id(), "Continue")
			.with_position(-0.8, 0.0)
			.with_size(0.6, 0.1)
			.with_style(&theme.deny_button)
			.with_z_index(5)
			.with_callback(move || {
				let ui_manager = &mut ptr::get_state().ui_manager;
				ui_manager.dialogs.respond(dialog_id.clone(), true);
				ui_manager.state = ui_manager.state.inner_state();
				ui_manager.setup_ui();
			});
		self.add_element(option_button_1);
		
		let option_button_2 = UIElement::button(self.next_id(), "Cancel")
			.with_position(0.2, 0.0)
			.with_size(0.6, 0.1)
			.with_style(&theme.okay_button)
			.with_z_index(5)
			.with_callback(move || {
				let ui_manager = &mut ptr::get_state().ui_manager;
				ui_manager.dialogs.respond(dialog_id.clone(), false);
				ui_manager.state = ui_manager.state.inner_state();
				ui_manager.setup_ui();
			});
		self.add_element(option_button_2);

		let back_button = UIElement::button(self.next_id(), "Back")
			.with_position(-0.1, -0.8)
			.with_size(0.2, 0.08)
			.with_style(&theme.buttons.extra())
			.with_z_index(8)
			.with_callback(|| close_pressed());
		self.add_element(back_button);
	}

	#[inline]
	fn setup_multiplayer_ui(&mut self) {
		let theme = &ptr::get_settings().ui_theme;
		let title = UIElement::label(self.next_id(), "Select World")
			.with_position(-0.4, 0.6)
			.with_size(0.8, 0.15)
			.with_style(&theme.title_label)
			.with_z_index(10);
		self.add_element(title);

		let list_panel = UIElement::panel(self.next_id())
			.with_position(-0.6, -0.4)
			.with_size(1.2, 0.9)
			.with_style(&theme.panels.basic)
			.with_z_index(1);
		self.add_element(list_panel);

		let worlds: Vec<String> = api::get_discovered_hosts()
			.iter()
			.map(|s| s.world_name.clone())
			.collect();

		for (i, name) in worlds.iter().enumerate() {
			let y_pos = 0.2 - (i as f32 * 0.12);
			let name_clone = name.clone();

			let world_button = UIElement::button(self.next_id(), name)
				.with_position(-0.4, y_pos)
				.with_size(0.8, 0.1)
				.with_style(&theme.buttons.basic)
				.with_z_index(5)
				.with_callback(move || handler::join_local_world(&name_clone));
			self.add_element(world_button);
		}

		let re_button = UIElement::button(self.next_id(), "refresh")
			.with_position(-0.4, -0.8)
			.with_size(0.25, 0.08)
			.with_style(&theme.buttons.nice)
			.with_z_index(8)
			.with_callback(|| {
				if let Err(e) = api::refresh_discovery() {
					println!("Refresh error: {}", e);
				}
				let state = ptr::get_state();
				state.ui_manager.setup_ui();
			});
		self.add_element(re_button);

		let connect_button = UIElement::button(self.next_id(), "manual connect")
			.with_position(0.15, -0.8)
			.with_size(0.5, 0.08)
			.with_style(&theme.buttons.nice)
			.with_z_index(8)
			.with_callback(|| {
				let ui_manager = &mut ptr::get_state().ui_manager;
				ui_manager.state = UIState::ConnectLocal;
				ui_manager.setup_ui();
			});
		self.add_element(connect_button);

		let back_button = UIElement::button(self.next_id(), "Back")
			.with_position(-0.1, -0.8)
			.with_size(0.2, 0.08)
			.with_style(&theme.buttons.extra())
			.with_z_index(8)
			.with_callback(|| close_pressed());
		self.add_element(back_button);
	}

	#[inline]
	fn setup_new_world_ui(&mut self) {
		let theme = &ptr::get_settings().ui_theme;
		let title = UIElement::label(self.next_id(), "Create New World")
			.with_position(-0.5, 0.4)
			.with_size(1.0, 0.15)
			.with_style(&theme.title_label)
			.with_z_index(10);
		self.add_element(title);

		let form_panel = UIElement::panel(self.next_id())
			.with_position(-0.4, -0.3)
			.with_size(0.8, 0.7)
			.with_style(&theme.panels.basic)
			.with_z_index(1);
		self.add_element(form_panel);

		let w_name_label = UIElement::label(self.next_id(), "World Name:")
			.with_position(-0.35, 0.1)
			.with_size(0.4, 0.08)
			.with_style(&theme.labels.basic)
			.with_z_index(3);
		self.add_element(w_name_label);

		let input_id = self.next_id();
		let world_name_input = UIElement::input(input_id)
			.with_position(-0.35, -0.0)
			.with_size(0.7, 0.1)
			.with_style(&theme.inputs.basic)
			.with_placeholder("New World")
			.with_z_index(5);
		self.add_element(world_name_input);

		let gen_button = UIElement::button(self.next_id(), "Create World")
			.with_position(-0.3, -0.2)
			.with_size(0.6, 0.1)
			.with_style(&theme.buttons.nice)
			.with_z_index(6)
			.with_callback(move || {
				handler::create_world(get_element_data_dy_id(input_id));
				let ui_manager = &mut ptr::get_state().ui_manager;
				ui_manager.state = UIState::WorldSelection;
				ui_manager.setup_ui();
			});
		self.add_element(gen_button);

		let back_button = UIElement::button(self.next_id(), "Back")
			.with_position(-0.1, -0.45)
			.with_size(0.2, 0.08)
			.with_style(&theme.buttons.extra())
			.with_z_index(8)
			.with_callback(|| close_pressed());
		self.add_element(back_button);
	}

	#[inline]
	fn setup_connect_local_ui(&mut self) {
		let theme = &ptr::get_settings().ui_theme;
		let title = UIElement::label(self.next_id(), "Manual Connect")
			.with_position(-0.5, 0.4)
			.with_size(1.0, 0.15)
			.with_style(&theme.title_label)
			.with_z_index(10);
		self.add_element(title);

		let form_panel = UIElement::panel(self.next_id())
			.with_position(-0.4, -0.3)
			.with_size(0.8, 0.7)
			.with_style(&theme.panels.basic)
			.with_z_index(1);
		self.add_element(form_panel);

		let w_ip_label = UIElement::label(self.next_id(), "Server IP:")
			.with_position(-0.35, 0.1)
			.with_size(0.4, 0.08)
			.with_style(&theme.labels.basic)
			.with_z_index(3);
		self.add_element(w_ip_label);

		let input_id = self.next_id();
		let world_ip_input = UIElement::input(input_id)
			.with_position(-0.35, -0.0)
			.with_size(0.7, 0.1)
			.with_style(&theme.inputs.basic)
			.with_placeholder("255.255.255.255")
			.with_z_index(5);
		self.add_element(world_ip_input);

		let connect_button = UIElement::button(self.next_id(), "Connect Server")
			.with_position(-0.3, -0.2)
			.with_size(0.6, 0.1)
			.with_style(&theme.buttons.nice)
			.with_z_index(6)
			.with_callback(move || {
				if let Err(e) = api::connect_to_host(&get_element_data_dy_id(input_id)) {
					println!("Error: {}", e);
				}
				let ui_manager = &mut ptr::get_state().ui_manager;
				ui_manager.state = UIState::WorldSelection;
				ui_manager.setup_ui();
			});
		self.add_element(connect_button);

		let back_button = UIElement::button(self.next_id(), "Back")
			.with_position(-0.1, -0.45)
			.with_size(0.2, 0.08)
			.with_style(&theme.buttons.extra())
			.with_z_index(8)
			.with_callback(|| close_pressed());
		self.add_element(back_button);
	}

	#[inline]
	fn setup_loading_screen_ui(&mut self) {
		let theme = &ptr::get_settings().ui_theme;
		let loading_panel = UIElement::panel(self.next_id())
			.with_position(-0.3, -0.1)
			.with_size(0.6, 0.2)
			.with_style(&theme.panels.bad)
			.with_z_index(10);
		self.add_element(loading_panel);

		let loading_text = UIElement::label(self.next_id(), "Loading...")
			.with_position(-0.25, -0.05)
			.with_size(0.5, 0.1)
			.with_style(&theme.labels.basic)
			.with_z_index(15);
		self.add_element(loading_text);

		let progress_bg = UIElement::panel(self.next_id())
			.with_position(-0.25, -0.15)
			.with_size(0.5, 0.03)
			.with_style(&theme.panels.basic)
			.with_z_index(8);
		self.add_element(progress_bg);

		let progress_bar = UIElement::panel(self.next_id())
			.with_position(-0.245, -0.145)
			.with_size(0.01, 0.02)
			.with_style(&theme.best_button)
			.with_z_index(8);
		self.add_element(progress_bar);
	}

	#[inline]
	fn setup_escape_ui(&mut self) {
		let theme = &ptr::get_settings().ui_theme;
		let bg_panel = UIElement::panel(self.next_id())
			.with_position(-1.0, -1.0)
			.with_size(2.0, 2.0)
			.with_color(Solor::Black.i().with_a(50))
			.with_alpha(40)
			.with_z_index(-5);
		self.add_element(bg_panel);

		let save_button = UIElement::button(self.next_id(), "Save World")
			.with_position(-0.8, 0.15)
			.with_size(0.4, 0.08)
			.with_style(&theme.buttons.nice)
			.with_z_index(8)
			.with_callback(|| {
				let save_path = ptr::get_gamestate().save_path();
				if let Err(e) = manager::save_entire_world(save_path) {
					println!("Error: {}", e);
				}
			});
		self.add_element(save_button);
		
		let load_button = UIElement::button(self.next_id(), "Load World")
			.with_position(-0.8, 0.0)
			.with_size(0.4, 0.08)
			.with_style(&theme.buttons.nice)
			.with_z_index(8)
			.with_callback(|| {
				let save_path = ptr::get_gamestate().save_path();
				if let Err(e) = manager::load_entire_world(save_path) {
					println!("Error: {}", e);
				}
			});
		self.add_element(load_button);
		
		let setting_button = UIElement::button(self.next_id(), "Settings")
			.with_position(-0.8, -0.15)
			.with_size(0.4, 0.08)
			.with_style(&theme.buttons.nice)
			.with_z_index(6)
			.with_callback(|| {
				let ui_manager = &mut ptr::get_state().ui_manager;
				ui_manager.state = UIState::Settings(UIStateID::from(&ui_manager.state));
				ui_manager.setup_ui();
			});
		self.add_element(setting_button);

		let memory_button = UIElement::button(self.next_id(), "Memory")
			.with_position(-0.8, -0.3)
			.with_size(0.4, 0.08)
			.with_style(&theme.buttons.nice)
			.with_z_index(6)
			.with_callback(|| {
				memory::light_trim();
				memory::hard_clean(Some(ptr::get_state().device()));
			});
		self.add_element(memory_button);

		let side_panel = UIElement::panel(self.next_id())
			.with_position(0.4, -0.9)
			.with_size(0.6, 1.8)
			.with_style(&theme.panels.basic)
			.with_z_index(1);
		self.add_element(side_panel);

		let panel_title = UIElement::label(self.next_id(), "Game Menu")
			.with_position(0.45, 0.75)
			.with_size(0.5, 0.1)
			.with_style(&theme.title_label)
			.with_z_index(10);
		self.add_element(panel_title);

		let clean_button = UIElement::button(self.next_id(), "Clean World")
			.with_position(0.45, 0.4)
			.with_size(0.5, 0.1)
			.with_style(&theme.buttons.bad)
			.with_z_index(8)
			.with_callback(|| block::extra::add_full_world());
		self.add_element(clean_button);

		let host_button = UIElement::button(self.next_id(), "Host World")
			.with_position(0.5, 0.22)
			.with_size(0.4, 0.08)
			.with_style(&theme.buttons.extra())
			.with_z_index(8)
			.with_callback(|| { 
				if let Err(e) = api::begin_online_giveaway() {
					println!("not worked: {}", e);
				}
			});
		self.add_element(host_button);

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
				.with_style(&theme.labels.bad)
				.with_z_index(5);
			self.add_element(help_text);
		}

		let back_button = UIElement::button(self.next_id(), "Back to World")
			.with_position(0.5, -0.8)
			.with_size(0.4, 0.08)
			.with_style(&theme.buttons.extra())
			.with_z_index(8)
			.with_callback(|| close_pressed());
		self.add_element(back_button);

		let close_button = UIElement::button(self.next_id(), "Quit World")
			.with_position(-0.2, -0.8)
			.with_size(0.4, 0.08)
			.with_style(&theme.worst_button)
			.with_z_index(8)
			.with_callback(|| {
				let state = ptr::get_state();
				state.ui_manager.state = UIState::BootScreen;
				state.ui_manager.setup_ui();
				handler::leave_world();
			});
		self.add_element(close_button);
	}

	#[inline]
	fn setup_in_game_ui(&mut self) {
		let theme = &ptr::get_settings().ui_theme;
		let crosshair_v = UIElement::divider(self.next_id())
			.with_position(0.0, -0.02)
			.with_size(0.02, 0.06)
			.with_style(&theme.dividers.basic)
			.with_z_index(20);
		let crosshair_h = UIElement::divider(self.next_id())
			.with_position(-0.02, 0.0)
			.with_size(0.06, 0.02)
			.with_style(&theme.dividers.basic)
			.with_z_index(20);

		self.add_element(crosshair_v);
		self.add_element(crosshair_h);
	}
}