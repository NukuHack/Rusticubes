
use crate::{
	ext::ptr,
	get_string,
	ui::{
		dialog,
		element::{self, UIElement, UIElementData, ElementData},
		render::{UIRenderer, Vertex},
	},
	item::ui_inventory::InventoryUIState
};
use winit::keyboard::KeyCode as Key;

#[derive(PartialEq, Clone, Copy)]
pub struct UIStateID(u32);

impl UIStateID {
	#[inline] pub fn new(id: u32) -> Self { Self(id) }
	#[inline] pub const fn default() -> Self { Self(0) }
}


// Update the UIStateID implementation to include Inventory
impl UIStateID {
	pub const fn from(state: &UIState) -> Self {
		#[allow(unreachable_patterns)]
		match state {
			UIState::None => UIStateID(0),
			UIState::BootScreen => UIStateID(1),
			UIState::WorldSelection => UIStateID(2),
			UIState::Multiplayer => UIStateID(3),
			UIState::NewWorld => UIStateID(4),
			UIState::Escape => UIStateID(5),
			UIState::InGame => UIStateID(6),
			UIState::Settings(..) => UIStateID(7),
			UIState::Confirm(..) => UIStateID(8),
			UIState::Loading => UIStateID(9),            
			UIState::Error(..) => UIStateID(10),
			UIState::ConnectLocal => UIStateID(11),
			UIState::Inventory(_) => UIStateID(12),
			_ => UIStateID(0),
		}
	}
}


#[derive(PartialEq, Clone, Default)]
pub enum UIState {
	#[default] None,
	BootScreen,
	WorldSelection,
	Multiplayer,
	ConnectLocal,
	NewWorld,
	Escape,
	InGame,
	Settings(UIStateID),
	Loading,
	Confirm(UIStateID, u8),
	Error(UIStateID, u8),

	Inventory(InventoryUIState),
}

impl UIState {
	pub const fn from(id: UIStateID) -> Self {
		#[allow(unreachable_patterns)]
		match id.0 {
			0 => UIState::None,
			1 => UIState::BootScreen,
			2 => UIState::WorldSelection,
			3 => UIState::Multiplayer,
			4 => UIState::NewWorld,
			5 => UIState::Escape,
			6 => UIState::InGame,
			7 => UIState::Settings(UIStateID::default()),
			8 => UIState::Confirm(UIStateID::default(), 0),
			9 => UIState::Loading,
			10 => UIState::Error(UIStateID::default(), 0),
			11 => UIState::ConnectLocal,
			12 => UIState::Inventory(InventoryUIState::default()),
			_ => UIState::None,
		}
	}

	pub const fn inner(&self) -> Option<u8> {
		match self {
			UIState::Confirm(_, id) | UIState::Error(_, id) => Some(*id),
			_ => None,
		}
	}
	
	pub const fn inner_state(&self) -> UIState {
		match self {
			UIState::Confirm(id, _) | UIState::Error(id, _) |
			UIState::Settings(id) => UIState::from(*id),
			_ => UIState::None,
		}
	}
}

pub fn close_pressed() {
	let state = ptr::get_state();
	match state.ui_manager.state.clone() {
		UIState::WorldSelection | UIState::Multiplayer => {
			state.ui_manager.state = UIState::BootScreen;
		},
		UIState::BootScreen => ptr::close_app(),
		UIState::InGame => {
			state.ui_manager.state = UIState::Escape;
			let game_state = ptr::get_gamestate();
			game_state.player_mut().controller().reset_keyboard();
			*game_state.running() = false;
			state.toggle_mouse_capture();
		},
		UIState::Escape => {
			state.ui_manager.state = UIState::InGame;
			*ptr::get_gamestate().running() = true;
			state.toggle_mouse_capture();
		},
		UIState::NewWorld => state.ui_manager.state = UIState::WorldSelection,
		UIState::Error(prev_state, dialog_id) | UIState::Confirm(prev_state, dialog_id) => {
			state.ui_manager.dialogs.cancel_dialog(dialog_id);
			state.ui_manager.state = UIState::from(prev_state);
		},
		UIState::Settings(prev_state) => state.ui_manager.state = UIState::from(prev_state),
		UIState::ConnectLocal => state.ui_manager.state = UIState::WorldSelection,
		UIState::Inventory(_) => {
			state.ui_manager.state = UIState::InGame;
			state.toggle_mouse_capture();
		},
		_ => return,
	}
	state.ui_manager.setup_ui();
}

pub struct UIManager {
	//basic stuff
	pub state: UIState,
	pub visibility: bool,
	//rendering stuff
	pub vertex_buffer: wgpu::Buffer,
	pub index_buffer: wgpu::Buffer,
	pub pipeline: wgpu::RenderPipeline,
	// main data
	pub elements: Vec<UIElement>,
	pub focused_element: Option<(usize, usize)>,
	renderer: UIRenderer,
	// extra for double callbacks
	pub dialogs: dialog::DialogManager,
	// helper stuff, mainly for init
	next_id: usize,
}


/// this is the screen related UI layout, also use f32 for position setting
/// -1,1|    |    |   |1,1 
/// ----+----+----+---+----
///     |    |    |   |    
/// ----+----+----+---+----
///     |    |0,0 |   |    
/// ----+----+----+---+----
///     |    |    |   |    
/// ----+----+----+---+----
///-1,-1|    |    |   |1,-1
// i use AI so i tried to make this as understandable for AI as i can ... even with all those stupid formatting they do with the whitespaces ...
impl UIManager {
	#[inline]
	pub fn new(device: &wgpu::Device, config: &wgpu::SurfaceConfiguration, queue: &wgpu::Queue) -> Self {
		let renderer = UIRenderer::new(device, queue);
		let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
			bind_group_layouts: &[renderer.bind_group_layout(), renderer.uniform_bind_group_layout()],
			..Default::default()
		});

		let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
			label: Some("UI Shader"),
			source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::from(get_string!("ui_shader.wgsl"))),
		});

		let ui_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
			label: Some("UI Pipeline"),
			layout: Some(&pipeline_layout),
			vertex: wgpu::VertexState {
				module: &shader,
				entry_point: Some("vs_main"),
				buffers: &[Vertex::desc()],
				compilation_options: Default::default(),
			},
			fragment: Some(wgpu::FragmentState {
				module: &shader,
				entry_point: Some("fs_main"),
				targets: &[Some(wgpu::ColorTargetState {
					format: config.format,
					blend: Some(wgpu::BlendState::ALPHA_BLENDING),
					write_mask: wgpu::ColorWrites::ALL,
				})],
				compilation_options: Default::default(),
			}),
			depth_stencil : None,
			primitive: wgpu::PrimitiveState::default(),
			multisample: wgpu::MultisampleState::default(),
			multiview: None,
			cache: None,
		});

		let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label: Some("UI Vertex Buffer"),
			size: 2048 * std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
			usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});

		let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
			label: Some("UI Index Buffer"),
			size: 2048 * std::mem::size_of::<u32>() as wgpu::BufferAddress,
			usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
			mapped_at_creation: false,
		});

		Self {
			state: Default::default(),
			vertex_buffer,
			index_buffer,
			pipeline: ui_pipeline,
			elements: Vec::new(),
			focused_element: None,
			visibility: true,
			dialogs: dialog::DialogManager::new(),
			renderer,
			next_id: 1,
		}
	}

	#[inline] pub const fn renderer(&self) -> &UIRenderer { &self.renderer }
	#[inline] pub const fn renderer_mut(&mut self) -> &mut UIRenderer { &mut self.renderer }
	
	#[inline]
	pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, delta: f32) {
		self.update_anim(delta);

		if true { // decided to remove the condition ...
			self.remake_mesh(device, queue);
		}
	}
	fn remake_mesh(&mut self, _device: &wgpu::Device, queue: &wgpu::Queue) {
		let (vertices, indices) = {
			// Isolate the renderer borrow in a smaller scope
			let renderer = &mut self.renderer;
			renderer.process_elements(&self.elements)
		};
		if !vertices.is_empty() {
			queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertices));
		}
		if !indices.is_empty() {
			queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&indices));
		}
	}
	
	#[inline]
	fn update_anim(&mut self, delta: f32) {
		self.elements.iter_mut()
			.filter(|e| matches!(e.data, UIElementData::Animation{..}))
			.for_each(|e| e.update_anim(delta));
	}
	
	#[inline]
	pub fn add_element(&mut self, mut element: UIElement) -> usize {
		if element.id == 0 {
			element.id = self.next_id;
			self.next_id += 1;
		}
		self.elements.push(element);
		self.next_id-1
	}
	
	#[inline]
	pub fn remove_element(&mut self, id: usize) -> bool {
		let Some(element) = self.elements.iter().position(|e| e.id == id) else { return false; };

		if let Some((idx, _)) = self.focused_element {
			if idx == element {
				self.clear_focused_element();
			}
		}
		self.elements.remove(element);
		true
	}

	#[inline] pub fn visible_elements(&self) -> Vec<&UIElement> { self.elements.iter().filter(|e| e.visible).collect() }
	#[inline] pub fn get_element(&self, id: usize) -> Option<&UIElement> { self.elements.iter().find(|e| e.id == id) }
	#[inline] pub fn get_element_mut(&mut self, id: usize) -> Option<&mut UIElement> { self.elements.iter_mut().find(|e| e.id == id) }
	#[inline] pub fn elements_with_parent(&self, parent: usize) -> Vec<&UIElement> { self.elements.iter().filter(|e| e.parent.map_or(false, |p| p.0 == parent)).collect() }
	#[inline] pub fn elements_with_parent_mut(&mut self, parent: usize) -> Vec<&mut UIElement> { self.elements.iter_mut().filter(|e| e.parent.map_or(false, |p| p.0 == parent)).collect() }
	 
	#[inline] pub fn clear_elements(&mut self) { self.elements.clear(); self.clear_focused_element(); self.next_id = 1; }
	
	#[inline]
	pub fn handle_key_input(&mut self, key: Key, shift: bool) -> bool {
		if matches!(self.focused_element, Some((_, 0))) {
			let Some(element) = self.get_focused_element_mut() else { return false; };
			if !(element.visible && element.enabled) { return false; }

			if !element.is_input() {
				if key != Key::Escape { return false; }
				self.clear_focused_element();
				return false;
			}
			match key {
				Key::Backspace => {
					let Some(text_mut) = element.get_text_mut() else { return false; };
					
					element::handle_backspace(text_mut);
				},
				Key::Enter | Key::Escape => self.clear_focused_element(),
				_ => if let Some(c) = element::key_to_char(key, shift) {
					let Some(text_mut) = element.get_text_mut() else { return false; };
					
					element::process_text_input(text_mut, c);
				}
			}
			return true;
		} else if matches!(self.focused_element, Some((_, 3))) {
			if key == Key::Escape {
				let inv = ptr::get_gamestate().player_mut().inventory_mut();
				let itm = inv.remove_cursor().unwrap();
				inv.add_item_anywhere(itm);
				close_pressed();
				self.setup_ui();
				return true;
			}
		}
		// we do not handle idx 2 because i don't think we need to ...


		false
	}
	
	#[inline] pub const fn clear_focused_element(&mut self) { self.focused_element = None; }
		
	#[inline] pub const fn toggle_visibility(&mut self) { self.visibility = !self.visibility; }
	#[inline] pub const fn focused_is_some(&self) -> bool { if self.focused_element.is_some() { true } else { false } }
	#[inline] pub fn get_focused_element(&self) -> Option<&UIElement> { if let Some((idx,_)) = self.focused_element { self.get_element(idx) } else { None } }
	#[inline] pub fn get_focused_element_mut(&mut self) -> Option<&mut UIElement> { if let Some((idx,_)) = self.focused_element { self.get_element_mut(idx) } else { None } }
	#[inline] pub const fn next_id(&mut self) -> usize { let id = self.next_id; self.next_id += 1; id }
	
	#[inline]
	pub fn render<'a>(&'a self, render_pass: &mut wgpu::RenderPass<'a>) {
		if self.visibility {
			self.renderer.render(self, render_pass);
		}
	}
}

#[inline]
pub fn get_element_data_by_id(id: &usize) -> Option<ElementData> {
	ptr::get_state()
		.ui_manager()
		.get_element(*id)
		.map(|element| element.get_element_data())
}
#[inline]
pub fn get_element_str_by_id(id: &usize) -> String {
	get_element_data_by_id(id)
		.and_then(|data| data.text())
		.unwrap_or("Null".to_string())
}
#[inline]
pub fn get_element_num_by_id(id: &usize) -> f32 {
	get_element_data_by_id(id)
		.and_then(|data| data.num())
		.unwrap_or(0.)
}
