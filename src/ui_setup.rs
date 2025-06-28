use crate::ui_manager::*;
use crate::ui_element::UIElement;

impl UIManager {
    #[inline]
    pub fn setup_ui(&mut self) {
        self.clear_elements();

        let bg_panel = UIElement::panel(self.next_id())
            .with_position(-1.0, -1.0)
            .with_size(2.0, 2.0)
            .with_color(20, 20, 30)  // Dark background
            .with_z_index(-5);

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

    #[inline]
    fn setup_boot_screen_ui(&mut self) {
        // Title with shadow effect
        let title = UIElement::label(self.next_id(), "Rusticubes")
            .with_position(-0.4, 0.3)
            .with_size(0.8, 0.2)
            .with_color(255, 255, 255)  // White
            .with_border((230, 230, 230, 230), 0.005)
            .with_z_index(10);
        self.add_element(title);

        // Button container panel
        let button_panel = UIElement::panel(self.next_id())
            .with_position(-0.35, -0.2)
            .with_size(0.7, 0.5)
            .with_color(38, 38, 51)  // Dark blue-gray
            .with_border((76, 76, 102, 255), 0.005)
            .with_z_index(1);
        self.add_element(button_panel);

        // Start button
        let start_button = UIElement::button(self.next_id(), "Start")
            .with_position(-0.15, 0.0)
            .with_size(0.3, 0.1)
            .with_color(51, 127, 204)  // Blue
            .with_border((76, 153, 229, 255), 0.005)
            .with_z_index(6)
            .with_callback(|| {
                let state = super::config::get_state();
                state.ui_manager.state = UIState::WorldSelection;
                state.ui_manager.setup_ui();
            });
        self.add_element(start_button);

        // Exit button
        let exit_button = UIElement::button(self.next_id(), "Exit")
            .with_position(-0.15, -0.15)
            .with_size(0.3, 0.1)
            .with_color(204, 51, 51)  // Red
            .with_border((229, 76, 76, 255), 0.005)
            .with_z_index(5)
            .with_callback(|| {
                close_pressed();
            });
        self.add_element(exit_button);

        // Funny tree (used in github too)
        let tree_picture = UIElement::image(self.next_id(), "happy-tree.png")
            .with_position(0.6, 0.5)
            .with_size(0.27, 0.45)
            .with_color(255, 255, 255)  // White
            .with_border((76, 153, 229, 255), 0.005)
            .with_z_index(6);
        self.add_element(tree_picture);

        let memory_button = UIElement::button(self.next_id(), "Memory")
            .with_position(0.5, 0.2)
            .with_size(0.3, 0.1)
            .with_color(51, 51, 51)  // Dark gray
            .with_border((102, 102, 102, 255), 0.005)
            .with_z_index(6)
            .with_callback(|| {
                super::memory::clean_gpu_memory(super::config::get_state().device());
                super::memory::MemoryManager::light_trim();
                super::memory::MemoryManager::aggressive_trim();
                super::memory::force_memory_cleanup();
            });
        self.add_element(memory_button);

        let tree_animation = UIElement::animation(self.next_id(), vec![
                "happy-tree.png".to_string(),
                "cube.jpg".to_string()
            ])
            .with_position(-0.8, 0.5)
            .with_size(0.27, 0.45)
            .with_color(255, 255, 255)  // White
            .with_border((76, 153, 229, 255), 0.005)
            .with_z_index(6)
            .with_smooth_transition(true)
            .with_animation_duration(2.5);
        self.add_element(tree_animation);

        // Version label at bottom
        let version = UIElement::label(self.next_id(), format!("v{}", env!("CARGO_PKG_VERSION")))
            .with_position(0.7, -0.95)
            .with_size(0.2, 0.05)
            .with_color(179, 179, 179)  // Light gray
            .with_border((127, 127, 127, 127), 0.003)
            .with_z_index(8);
        self.add_element(version);
    }

    #[inline]
    fn setup_world_selection_ui(&mut self) {
        // Title with decorative border
        let title = UIElement::label(self.next_id(), "Select World")
            .with_position(-0.4, 0.6)
            .with_size(0.8, 0.15)
            .with_color(255, 255, 255)  // White
            .with_border((179, 179, 204, 255), 0.005)
            .with_z_index(10);
        self.add_element(title);

        // World list container
        let list_panel = UIElement::panel(self.next_id())
            .with_position(-0.6, -0.4)
            .with_size(1.2, 1.0)
            .with_color(38, 38, 51)  // Dark blue-gray
            .with_border((63, 63, 89, 255), 0.01)
            .with_z_index(1);
        self.add_element(list_panel);

        // New World button
        let new_w_button = UIElement::button(self.next_id(), "Create New World")
            .with_position(-0.3, 0.4)
            .with_size(0.6, 0.1)
            .with_color(76, 102, 153)  // Medium blue
            .with_border((102, 127, 179, 255), 0.005)
            .with_z_index(8)
            .with_callback(|| {
                let state = super::config::get_state();
                state.ui_manager.state = UIState::NewWorld;
                state.ui_manager.setup_ui();
            });
        self.add_element(new_w_button);

        let worlds = match super::world_manager::get_world_names() {
            Ok(worlds) => worlds,
            Err(e) => {
                eprintln!("Error loading world names: {}", e);
                Vec::new()
            }
        };

        // Add world buttons
        for (i, name) in worlds.iter().enumerate() {
            let y_pos = 0.2 - (i as f32 * 0.12);
            let name_clone = name.clone();
            let name_cl = name_clone.clone();

            let world_button = UIElement::button(self.next_id(), name.clone())
                .with_position(-0.4, y_pos)
                .with_size(0.8, 0.1)
                .with_color(63, 63, 102)  // Darker blue
                .with_border((89, 89, 127, 255), 0.005)
                .with_z_index(5)
                .with_callback(move || {
                    super::world_builder::join_world(&name_cl);
                });
            self.add_element(world_button);

            // Delete button with hover effects
            let delete_button = UIElement::button(self.next_id(), "del")
                .with_position(0.42, y_pos)
                .with_size(0.13, 0.1)
                .with_color(204, 51, 51)  // Red
                .with_border((229, 76, 76, 255), 0.005)
                .with_z_index(5)
                .with_callback(move || {
                    super::world_builder::del_world(&name_clone);
                });
            self.add_element(delete_button);
        }

        // Back button with consistent styling
        let back_button = UIElement::button(self.next_id(), "Back")
            .with_position(-0.1, -0.8)
            .with_size(0.2, 0.08)
            .with_color(127, 127, 127)  // Gray
            .with_border((153, 153, 153, 255), 0.005)
            .with_z_index(8)
            .with_callback(|| {
                let state = super::config::get_state();
                state.ui_manager.state = UIState::BootScreen;
                state.ui_manager.setup_ui();
            });
        self.add_element(back_button);
    }

    #[inline]
    fn setup_new_world_ui(&mut self) {
        // Title with decorative border
        let title = UIElement::label(self.next_id(), "Create New World")
            .with_position(-0.5, 0.4)
            .with_size(1.0, 0.15)
            .with_color(255, 255, 255)  // White
            .with_border((179, 179, 204, 255), 0.005)
            .with_z_index(10);
        self.add_element(title);

        // Form panel
        let form_panel = UIElement::panel(self.next_id())
            .with_position(-0.4, -0.3)
            .with_size(0.8, 0.7)
            .with_color(38, 38, 51)  // Dark blue-gray
            .with_border((63, 63, 89, 255), 0.01)
            .with_z_index(1);
        self.add_element(form_panel);

        // World name label
        let w_name_label = UIElement::label(self.next_id(), "World Name:")
            .with_position(-0.35, 0.1)
            .with_size(0.4, 0.08)
            .with_color(229, 229, 229)  // Light gray
            .with_z_index(3);
        self.add_element(w_name_label);

        // World Name input
        let input_id = self.next_id();
        let world_name_input = UIElement::input(input_id)
            .with_position(-0.35, -0.0)
            .with_size(0.7, 0.1)
            .with_color(51, 51, 76)  // Dark blue
            .with_placeholder("New World")
            .with_border((102, 102, 153, 255), 0.005)
            .with_z_index(5);
        self.add_element(world_name_input);

        // Generate button
        let gen_button = UIElement::button(self.next_id(), "Create World")
            .with_position(-0.3, -0.2)
            .with_size(0.6, 0.1)
            .with_color(76, 102, 153)  // Medium blue
            .with_border((102, 127, 179, 255), 0.005)
            .with_z_index(6)
            .with_callback(move || {
                let world_name = super::config::get_state()
                    .ui_manager()
                    .get_input_text(input_id)
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "New World".to_string());
                super::world_builder::join_world(&world_name);
            });
        self.add_element(gen_button);

        // Back button
        let back_button = UIElement::button(self.next_id(), "Back")
            .with_position(-0.1, -0.45)
            .with_size(0.2, 0.08)
            .with_color(127, 127, 127)  // Gray
            .with_border((153, 153, 153, 255), 0.005)
            .with_z_index(8)
            .with_callback(|| {
                let state = super::config::get_state();
                state.ui_manager.state = UIState::WorldSelection;
                state.ui_manager.setup_ui();
            });
        self.add_element(back_button);
    }

    #[inline]
    fn setup_loading_screen_ui(&mut self) {
        // Loading panel
        let loading_panel = UIElement::panel(self.next_id())
            .with_position(-0.3, -0.1)
            .with_size(0.6, 0.2)
            .with_color(25, 25, 38)  // Very dark blue
            .with_border((76, 76, 102, 255), 0.01)
            .with_z_index(10);
        self.add_element(loading_panel);

        // Loading text with animation
        let loading_text = UIElement::label(self.next_id(), "Loading...")
            .with_position(-0.25, -0.05)
            .with_size(0.5, 0.1)
            .with_color(255, 255, 255)  // White
            .with_z_index(15);
        self.add_element(loading_text);

        // Progress bar background
        let progress_bg = UIElement::panel(self.next_id())
            .with_position(-0.25, -0.15)
            .with_size(0.5, 0.03)
            .with_color(12, 12, 25)  // Very dark
            .with_border((51, 51, 76, 255), 0.005)
            .with_z_index(8);
        self.add_element(progress_bg);

        // Progress bar (animated)
        let progress_bar = UIElement::panel(self.next_id())
            .with_position(-0.245, -0.145)
            .with_size(0.01, 0.02) // Will be animated
            .with_color(76, 127, 204)  // Bright blue
            .with_z_index(8);
        self.add_element(progress_bar);
    }

    #[inline]
    fn setup_in_game_ui(&mut self) {
        // Side panel for in-game UI
        let side_panel = UIElement::panel(self.next_id())
            .with_position(0.4, -0.9)
            .with_size(0.6, 1.8)
            .with_color(25, 25, 38)  // Dark blue-gray
            .with_border((51, 51, 76, 255), 0.01)
            .with_z_index(1);
        self.add_element(side_panel);

        // Panel title
        let panel_title = UIElement::label(self.next_id(), "Game Menu")
            .with_position(0.45, 0.75)
            .with_size(0.5, 0.1)
            .with_color(255, 255, 255)  // White
            .with_border((127, 127, 153, 255), 0.005)
            .with_z_index(10);
        self.add_element(panel_title);

        // Clean world button
        let clean_button = UIElement::button(self.next_id(), "Clean World")
            .with_position(0.45, 0.4)
            .with_size(0.5, 0.1)
            .with_color(153, 76, 76)  // Reddish
            .with_border((179, 102, 102, 255), 0.005)
            .with_z_index(8)
            .with_callback(|| {
                println!("Clean world button clicked!");
                super::cube_extra::add_full_world();
            });
        self.add_element(clean_button);

        // Help text
        let help_text_1 = UIElement::label(self.next_id(), "ALT to lock")
            .with_position(0.5, 0.1)
            .with_size(0.4, 0.08)
            .with_color(255, 255, 255)  // White
            .with_border((127, 127, 153, 255), 0.005)
            .with_z_index(5);
        let help_text_2 = UIElement::label(self.next_id(), "L to fill chunk")
            .with_position(0.5, -0.05)
            .with_size(0.4, 0.08)
            .with_color(255, 255, 255)  // White
            .with_border((127, 127, 153, 255), 0.005)
            .with_z_index(5);
        let help_text_3 = UIElement::label(self.next_id(), "R to break")
            .with_position(0.5, -0.2)
            .with_size(0.4, 0.08)
            .with_color(255, 255, 255)  // White
            .with_border((127, 127, 153, 255), 0.005)
            .with_z_index(5);
        let help_text_4 = UIElement::label(self.next_id(), "F to place")
            .with_position(0.5, -0.35)
            .with_size(0.4, 0.08)
            .with_color(255, 255, 255)  // White
            .with_border((127, 127, 153, 255), 0.005)
            .with_z_index(5);
        let help_text_5 = UIElement::label(self.next_id(), "ESC to leave")
            .with_position(0.5, -0.5)
            .with_size(0.4, 0.08)
            .with_color(255, 255, 255)  // White
            .with_border((127, 127, 153, 255), 0.005)
            .with_z_index(5);

        self.add_element(help_text_1);
        self.add_element(help_text_2);
        self.add_element(help_text_3);
        self.add_element(help_text_4);
        self.add_element(help_text_5);

        // Close button
        let close_button = UIElement::button(self.next_id(), "Exit Game")
            .with_position(0.55, -0.8)
            .with_size(0.3, 0.08)
            .with_color(204, 51, 51)  // Red
            .with_border((229, 76, 76, 255), 0.005)
            .with_z_index(8)
            .with_callback(|| {
                println!("Close button clicked!");
                close_pressed();
            });
        self.add_element(close_button);

        // Crosshair with better visibility
        let crosshair_v = UIElement::divider(self.next_id())
            .with_position(0.0, -0.02)
            .with_size(0.02, 0.06)
            .with_color(255, 255, 255)  // White
            .with_z_index(20);
        
        let crosshair_h = UIElement::divider(self.next_id())
            .with_position(-0.02, 0.0)
            .with_size(0.06, 0.02)
            .with_color(255, 255, 255)  // White
            .with_z_index(20);

        self.add_element(crosshair_v);
        self.add_element(crosshair_h);
    }
}