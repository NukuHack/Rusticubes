use crate::ui_manager::close_pressed;
use crate::ui_manager::UIManager;
use crate::ui_manager::UIState;
use crate::ui_element::UIElement;

impl UIManager {
    pub fn setup_ui(&mut self) {
        self.clear_elements();

        let bg_panel =
            UIElement::new_panel(self.next_id(), (-1.0, -1.0), (2.0, 2.0), [0.08, 0.08, 0.12])
                .set_z_index(-5);

        match self.state {
            UIState::None => {
                self.state = UIState::BootScreen;
                self.setup_ui();
            }
            UIState::BootScreen => {
                self.add_element(bg_panel);
                self.setup_boot_screen_ui();
            }
            UIState::WorldSelection => {
                self.add_element(bg_panel);
                self.setup_world_selection_ui();
            }
            UIState::Loading => {
                self.add_element(bg_panel);
                self.setup_loading_screen_ui();
            }
            UIState::NewWorld => {
                self.add_element(bg_panel);
                self.setup_new_world_ui();
            }
            UIState::InGame => {
                self.setup_in_game_ui();
            }
        }
    }

    fn setup_boot_screen_ui(&mut self) {
        // Title with shadow effect
        let title = UIElement::new_label(
            self.next_id(),
            (-0.4, 0.3),
            (0.8, 0.2),
            [1.0, 1.0, 1.0],
            "Rusticubes".to_string(),
        )
        .set_border([0.9, 0.9, 0.9, 0.9], 0.005)
        .set_z_index(10);
        self.add_element(title);

        // Button container panel
        let button_panel =
            UIElement::new_panel(self.next_id(), (-0.35, -0.2), (0.7, 0.5), [0.15, 0.15, 0.2])
                .set_border([0.3, 0.3, 0.4, 1.0], 0.005)
                .set_z_index(1);
        self.add_element(button_panel);

        // Start button
        let start_button = UIElement::new_button(
            self.next_id(),
            (-0.15, 0.0),
            (0.3, 0.1),
            [0.2, 0.5, 0.8],
            "Start".to_string(),
            || {
                let state = super::config::get_state();
                state.ui_manager.state = UIState::WorldSelection;
                state.ui_manager.setup_ui();
            },
        )
        .set_border([0.3, 0.6, 0.9, 1.0], 0.005)
        .set_z_index(6);
        self.add_element(start_button);

        // Exit button
        let exit_button = UIElement::new_button(
            self.next_id(),
            (-0.15, -0.15),
            (0.3, 0.1),
            [0.8, 0.2, 0.2],
            "Exit".to_string(),
            || {
                close_pressed();
            },
        )
        .set_border([0.9, 0.3, 0.3, 1.0], 0.005)
        .set_z_index(5);
        self.add_element(exit_button);

        // Funny tree (used in github too)
        let tree_picture = UIElement::new_image(
            self.next_id(),
            (0.6, 0.5),
            (0.27, 0.45),
            [1f32, 1f32, 1f32],
            "happy-tree.png".to_string(),
        )
        .set_border([0.3, 0.6, 0.9, 1.0], 0.005)
        .set_z_index(6);
        self.add_element(tree_picture);

        let tree_picture = UIElement::new_animation(
            self.next_id(),
            (-0.6, 0.5),
            (-0.27, 0.45),
            [1f32, 1f32, 1f32],
            vec![
                "happy-tree.png".to_string(),
                "cube.jpg".to_string()
            ],
        )
        .set_border([0.3, 0.6, 0.9, 1.0], 0.005)
        .set_z_index(6)
        .anim_smooth(true)
        .set_anim_duration(2.5);
        self.add_element(tree_picture);

        // Version label at bottom
        let version = UIElement::new_label(
            self.next_id(),
            (0.7, -0.95),
            (0.2, 0.05),
            [0.7, 0.7, 0.7],
            format!("v{}", env!("CARGO_PKG_VERSION")),
        )
        .set_border([0.5, 0.5, 0.5, 0.5], 0.003)
        .set_z_index(8);
        self.add_element(version);
    }

    fn setup_world_selection_ui(&mut self) {
        // Title with decorative border
        let title = UIElement::new_label(
            self.next_id(),
            (-0.4, 0.6),
            (0.8, 0.15),
            [1.0, 1.0, 1.0],
            "Select World".to_string(),
        )
        .set_border([0.7, 0.7, 0.8, 1.0], 0.005)
        .set_z_index(10);
        self.add_element(title);

        // World list container
        let list_panel =
            UIElement::new_panel(self.next_id(), (-0.6, -0.4), (1.2, 1.0), [0.15, 0.15, 0.2])
                .set_border([0.25, 0.25, 0.35, 1.0], 0.01)
                .set_z_index(1);
        self.add_element(list_panel);

        // New World button
        let new_w_button = UIElement::new_button(
            self.next_id(),
            (-0.3, 0.4),
            (0.6, 0.1),
            [0.3, 0.4, 0.6],
            "Create New World".to_string(),
            || {
                let state = super::config::get_state();
                state.ui_manager.state = UIState::NewWorld;
                state.ui_manager.setup_ui();
            },
        )
        .set_border([0.4, 0.5, 0.7, 1.0], 0.005)
        .set_z_index(8);
        self.add_element(new_w_button);

        let worlds = match super::file_manager::get_world_names() {
            Ok(worlds) => worlds,
            Err(e) => {
                eprintln!("Error loading world names: {}", e);
                Vec::new()
            }
        };

        // Add world buttons
        for (i, name) in worlds.iter().enumerate() {
            let y_pos = 0.2 - (i as f32 * 0.12);
            
            // Create one clone that will be moved into both closures
            let name_clone = name.clone();

            let world_button = UIElement::new_button(
                self.next_id(),
                (-0.4, y_pos),
                (0.8, 0.1),
                [0.25, 0.25, 0.4],
                name.clone(),  // This clone is for the button text display
                {
                    let name_clone = name_clone.clone();
                    move || {
                        super::world_builder::join_world(&name_clone);
                    }
                },
            )
            .set_border([0.35, 0.35, 0.5, 1.0], 0.005)
            .set_z_index(5);
            self.add_element(world_button);

            // Delete button with hover effects
            let delete_button = UIElement::new_button(
                self.next_id(),
                (0.42, y_pos),
                (0.13, 0.1),
                [0.8, 0.2, 0.2],
                String::from("del"),
                move || {
                    super::world_builder::del_world(&name_clone);
                },
            )
            .set_border([0.9, 0.3, 0.3, 1.0], 0.005)
            .set_z_index(5);
            self.add_element(delete_button);
        }

        // Back button with consistent styling
        let back_button = UIElement::new_button(
            self.next_id(),
            (-0.1, -0.8),
            (0.2, 0.08),
            [0.5, 0.5, 0.5],
            "Back".to_string(),
            || {
                let state = super::config::get_state();
                state.ui_manager.state = UIState::BootScreen;
                state.ui_manager.setup_ui();
            },
        )
        .set_border([0.6, 0.6, 0.6, 1.0], 0.005)
        .set_z_index(8);
        self.add_element(back_button);
    }

    fn setup_new_world_ui(&mut self) {
        // Title with decorative border
        let title = UIElement::new_label(
            self.next_id(),
            (-0.5, 0.4),
            (1.0, 0.15),
            [1.0, 1.0, 1.0],
            "Create New World".to_string(),
        )
        .set_border([0.7, 0.7, 0.8, 1.0], 0.005)
        .set_z_index(10);
        self.add_element(title);

        // Form panel
        let form_panel =
            UIElement::new_panel(self.next_id(), (-0.4, -0.3), (0.8, 0.7), [0.15, 0.15, 0.2])
                .set_border([0.25, 0.25, 0.35, 1.0], 0.01)
                .set_z_index(1);
        self.add_element(form_panel);

        // World name label
        let w_name_label = UIElement::new_label(
            self.next_id(),
            (-0.35, 0.1),
            (0.3, 0.08),
            [0.9, 0.9, 0.9],
            "World Name:".to_string(),
        )
        .set_z_index(3);
        self.add_element(w_name_label);

        // World Name input
        let input_id = self.next_id();
        let world_name_input = UIElement::new_input(
            input_id,
            (-0.35, -0.0),
            (0.7, 0.1),
            [0.2, 0.2, 0.3],
            Some("New World".to_string()),
        )
        .set_border([0.4, 0.4, 0.6, 1.0], 0.005)
        .set_z_index(5);
        self.add_element(world_name_input);

        // Generate button
        let gen_button = UIElement::new_button(
            self.next_id(),
            (-0.2, -0.2),
            (0.4, 0.1),
            [0.3, 0.4, 0.6],
            "Create World".to_string(),
            move || {
                let state = super::config::get_state();
                let world_name = state
                    .ui_manager
                    .get_input_text(input_id)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "New World".to_string());
                super::world_builder::join_world(&world_name);
            },
        )
        .set_border([0.4, 0.5, 0.7, 1.0], 0.005)
        .set_z_index(6);
        self.add_element(gen_button);

        // Back button
        let back_button = UIElement::new_button(
            self.next_id(),
            (-0.1, -0.45),
            (0.2, 0.08),
            [0.5, 0.5, 0.5],
            "Back".to_string(),
            || {
                let state = super::config::get_state();
                state.ui_manager.state = UIState::WorldSelection;
                state.ui_manager.setup_ui();
            },
        )
        .set_border([0.6, 0.6, 0.6, 1.0], 0.005)
        .set_z_index(8);
        self.add_element(back_button);
    }

    fn setup_loading_screen_ui(&mut self) {
        // Loading panel
        let loading_panel =
            UIElement::new_panel(self.next_id(), (-0.3, -0.1), (0.6, 0.2), [0.1, 0.1, 0.15])
                .set_border([0.3, 0.3, 0.4, 1.0], 0.01)
                .set_z_index(10);
        self.add_element(loading_panel);

        // Loading text with animation
        let loading_text = UIElement::new_label(
            self.next_id(),
            (-0.25, -0.05),
            (0.5, 0.1),
            [1.0, 1.0, 1.0],
            "Loading...".to_string(),
        )
        .set_z_index(15);
        self.add_element(loading_text);

        // Progress bar background
        let progress_bg = UIElement::new_panel(
            self.next_id(),
            (-0.25, -0.15),
            (0.5, 0.03),
            [0.05, 0.05, 0.1],
        )
        .set_border([0.2, 0.2, 0.3, 1.0], 0.005)
        .set_z_index(8);
        self.add_element(progress_bg);

        // Progress bar (animated)
        let progress_bar = UIElement::new_panel(
            self.next_id(),
            (-0.245, -0.145),
            (0.01, 0.02), // Will be animated
            [0.3, 0.5, 0.8],
        )
        .set_z_index(8);
        self.add_element(progress_bar);
    }

    fn setup_in_game_ui(&mut self) {
        // Side panel for in-game UI
        let side_panel =
            UIElement::new_panel(self.next_id(), (0.4, -0.9), (0.6, 1.8), [0.1, 0.1, 0.15])
                .set_border([0.2, 0.2, 0.3, 1.0], 0.01)
                .set_z_index(1);
        self.add_element(side_panel);

        // Panel title
        let panel_title = UIElement::new_label(
            self.next_id(),
            (0.45, 0.75),
            (0.5, 0.1),
            [1.0, 1.0, 1.0],
            "Game Menu".to_string(),
        )
        .set_border([0.5, 0.5, 0.6, 1.0], 0.005)
        .set_z_index(10);
        self.add_element(panel_title);

        // Clean world button
        let clean_button = UIElement::new_button(
            self.next_id(),
            (0.45, 0.4),
            (0.5, 0.15),
            [0.6, 0.3, 0.3],
            "Clean World".to_string(),
            || {
                println!("Clean world button clicked!");
                super::cube_extra::add_full_world();
            },
        )
        .set_border([0.7, 0.4, 0.4, 1.0], 0.005)
        .set_z_index(8);
        self.add_element(clean_button);

        // Help text
        let help_text_1 = UIElement::new_label(
            self.next_id(),
            (0.5, 0.1),
            (0.4, 0.1),
            [1.0, 1.0, 1.0],
            "Press ALT to lock".to_string(),
        )
        .set_border([0.5, 0.5, 0.6, 1.0], 0.005)
        .set_z_index(5);
        self.add_element(help_text_1);

        let help_text_2 = UIElement::new_label(
            self.next_id(),
            (0.5, -0.05),
            (0.4, 0.1),
            [1.0, 1.0, 1.0],
            "Press L to fill chunk".to_string(),
        )
        .set_border([0.5, 0.5, 0.6, 1.0], 0.005)
        .set_z_index(5);
        self.add_element(help_text_2);

        let help_text_3 = UIElement::new_label(
            self.next_id(),
            (0.5, -0.2),
            (0.4, 0.1),
            [1.0, 1.0, 1.0],
            "Press R to break".to_string(),
        )
        .set_border([0.5, 0.5, 0.6, 1.0], 0.005)
        .set_z_index(5);
        self.add_element(help_text_3);

        let help_text_4 = UIElement::new_label(
            self.next_id(),
            (0.5, -0.35),
            (0.4, 0.1),
            [1.0, 1.0, 1.0],
            "Press F to place".to_string(),
        )
        .set_border([0.5, 0.5, 0.6, 1.0], 0.005)
        .set_z_index(5);
        self.add_element(help_text_4);

        let help_text_5 = UIElement::new_label(
            self.next_id(),
            (0.5, -0.5),
            (0.4, 0.1),
            [1.0, 1.0, 1.0],
            "Press ESC to leave".to_string(),
        )
        .set_border([0.5, 0.5, 0.6, 1.0], 0.005)
        .set_z_index(5);
        self.add_element(help_text_5);

        // Close button
        let close_button = UIElement::new_button(
            self.next_id(),
            (0.55, -0.8),
            (0.3, 0.1),
            [0.8, 0.2, 0.2],
            "Exit Game".to_string(),
            || {
                println!("Close button clicked!");
                close_pressed();
            },
        )
        .set_border([0.9, 0.3, 0.3, 1.0], 0.005)
        .set_z_index(8);
        self.add_element(close_button);

        // Crosshair with better visibility
        let crosshair_v =
            UIElement::new_divider(self.next_id(), (0.0, -0.02), (0.02, 0.06), [1.0, 1.0, 1.0])
                .set_z_index(20);
        let crosshair_h =
            UIElement::new_divider(self.next_id(), (-0.02, 0.0), (0.06, 0.02), [1.0, 1.0, 1.0])
                .set_z_index(20);

        self.add_element(crosshair_v);
        self.add_element(crosshair_h);
    }
}