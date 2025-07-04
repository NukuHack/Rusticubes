
use crate::block;
use std::iter::Iterator;
use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    keyboard::KeyCode as Key,
};

impl<'a> super::State<'a> {

    #[inline]
    pub fn handle_events(&mut self,event: &WindowEvent) -> bool{

        // should rework this like this :
            // send the entire event to "window event handler"
        // if not processed 
            // send the entire event to "UI event handler"
        // if not processed again
            // send the entire event to "world event handler"

        // if not processed then basically it's an event what should not be processed entirely so probably log it or idk 

        match event {
            WindowEvent::CloseRequested => {super::config::close_app(); true},
            WindowEvent::Resized(physical_size) => self.resize(*physical_size),
            WindowEvent::RedrawRequested => {
                self.window().request_redraw();
                self.update();
                match self.render() {
                    Ok(_) => true,
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        self.resize(*self.size())
                    },
                    Err(wgpu::SurfaceError::OutOfMemory | wgpu::SurfaceError::Other) => {
                        println!("Surface error");
                        super::config::close_app(); true
                    }
                    Err(wgpu::SurfaceError::Timeout) => {
                        println!("Surface timeout");
                        true
                    },
                }
            },
            WindowEvent::ModifiersChanged(modifiers) => {
                self.input_system.modifiers = modifiers.state();
                true
            },
            WindowEvent::KeyboardInput { .. } => {
                self.handle_key_input(event);
                true
            },
            _ => {
                self.handle_mouse_input(event);
                true
            }
        }
    }
    #[inline]
    pub fn handle_key_input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event: winit::event::KeyEvent {
                    physical_key,
                    state // ElementState::Released or ElementState::Pressed
                    , .. },..
            } => {
                let key = match physical_key {
                    winit::keyboard::PhysicalKey::Code(code) => *code,
                    _ => {
                        println!("You called a function that can only be called with a keyboard input ... without a keyboard input ... FF"); 
                        return false;
                    },
                };
                // Handle UI input first if there's a focused element
                if let Some(_focused_idx) = self.ui_manager.focused_element {
                    if self.is_world_running {
                        super::config::get_gamestate().player_mut().controller().reset_keyboard(); // Temporary workaround
                    }
                    
                    if *state == ElementState::Pressed {
                        // Handle special keys for UI
                        self.ui_manager.handle_key_input(key,self.input_system.modifiers.shift_key());
                        return true;
                    }
                    return true;
                }

                // Toggle mouse capture when ALT is pressed
                if key == Key::AltLeft || key == Key::AltRight {
                    if *state == ElementState::Pressed {
                        self.toggle_mouse_capture();
                    }
                    return true;
                }

                // Handle game controls if no UI element is focused
                // `key` is of type `KeyCode` (e.g., KeyCode::W)
                // `state` is of type `ElementState` (Pressed or Released)
                if self.is_world_running {
                    super::config::get_gamestate().player_mut().controller().process_keyboard(&key, &state);
                    match key {
                        Key::KeyF => {
                            if *state == ElementState::Pressed {
                                block::extra::place_looked_cube();
                                return true
                            }
                            return false;
                        },
                        Key::KeyR => {
                            if *state == ElementState::Pressed {
                                block::extra::remove_targeted_block();
                                return true
                            }
                            return false;
                        },
                        Key::KeyE => {
                            if *state == ElementState::Pressed {
                                block::extra::toggle_looked_point();
                                return true
                            }
                            return false;
                        },
                        Key::KeyL => {
                            if *state == ElementState::Pressed {
                                block::extra::add_full_chunk();
                                return true
                            }
                            return false;
                        },
                        _ => false,
                    };
                }
                match key {
                    Key::AltLeft | Key::AltRight => {
                        self.center_mouse();
                        true
                    },
                    Key::Escape => {
                        if *state == ElementState::Pressed {
                            crate::ui::manager::close_pressed();
                            return true;
                        }
                        false
                    },
                    Key::F1 => {
                        if *state == ElementState::Pressed {
                            self.ui_manager.toggle_visibility();
                            return true
                        }
                        false
                    },
                    Key::F11 => {
                        if *state == ElementState::Pressed {
                            let window = self.window();
                            
                            if window.fullscreen().is_some() {
                                // If already fullscreen, exit fullscreen
                                window.set_fullscreen(None);
                            } else {
                                // Otherwise enter fullscreen
                                let current_monitor = window.current_monitor().unwrap_or_else(|| {
                                    window.available_monitors().next().expect("No monitors available")
                                });
                                
                                window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(Some(current_monitor))));
                            }
                            return true;
                        }
                        false
                    },
                    _ => false,
                }
            },
            _ => false
        }
    }
    #[inline]
    pub fn handle_mouse_input(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::MouseInput { button, state, .. } => {
                match (button, *state) {
                    (MouseButton::Left, ElementState::Pressed) => {
                        self.input_system.mouse_button_state.left = true;
                        if self.ui_manager.visibility!=false{
                        // Use the stored current mouse position
                        if let Some(current_position) = self.input_system.previous_mouse {
                            crate::ui::manager::handle_ui_click(&mut self.ui_manager, self.render_context.size.into(), &current_position);
                        }
                        }
                        true
                    }
                    (MouseButton::Left, ElementState::Released) => {
                        self.input_system.mouse_button_state.left = false;
                        true
                    }
                    (MouseButton::Right, ElementState::Pressed) => {
                        self.input_system.mouse_button_state.right = true;
                        true
                    }
                    (MouseButton::Right, ElementState::Released) => {
                        self.input_system.mouse_button_state.right = false;
                        true
                    }
                    _ => false,
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                if *self.input_system.mouse_captured() == true {
                    // Calculate relative movement from center
                    let size = self.size();
                    let center_x = size.width as f64 / 2.0;
                    let center_y = size.height as f64 / 2.0;
                    
                    let delta_x = (position.x - center_x) as f32;
                    let delta_y = (position.y - center_y) as f32;
                    
                    // Process mouse movement for camera control

                    if self.is_world_running {
                        super::config::get_gamestate().player_mut().controller().process_mouse(delta_x, delta_y);
                    }
                    // Reset cursor to center
                    self.center_mouse();
                    self.input_system.previous_mouse = Some(winit::dpi::PhysicalPosition::new(center_x, center_y));
                    return true;
                } else {
                    // Handle normal mouse movement for UI
                    if self.input_system.mouse_button_state.right {
                        if let Some(prev) = self.input_system.previous_mouse {
                            let delta_x = (position.x - prev.x) as f32;
                            let delta_y = (position.y - prev.y) as f32;
                            if self.is_world_running {
                                super::config::get_gamestate().player_mut().controller().process_mouse(delta_x, delta_y);
                            }
                        }
                    }
                    
                    // Handle UI hover
                    crate::ui::manager::handle_ui_hover(&mut self.ui_manager, self.render_context.size.into(), position);
                    self.input_system.previous_mouse = Some(*position);
                    return true;
                }

            }
            WindowEvent::MouseWheel { delta, .. } => {
                if self.is_world_running {
                    super::config::get_gamestate().player_mut().controller().process_scroll(delta);
                }
                true
            }
            _ => false,
        };
        false
    }


}