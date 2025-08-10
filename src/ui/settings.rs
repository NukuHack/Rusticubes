
use glam::Vec2;
use crate::ext::ptr;
use crate::ext::audio::{set_bg_volume, set_fg_volume};
use crate::ui::manager::{close_pressed, UIManager, get_element_num_by_id};
use crate::ui::element::UIElement;

impl UIManager {
	#[inline]
	pub fn setup_settings_ui(&mut self) {
		let theme = &ptr::get_settings().ui_theme;
		let settings = &ptr::get_settings();
		// Title
		let title = UIElement::label(self.next_id(), "Settings ... yah".into())
			.with_position(Vec2::new(-0.4, 0.6))
			.with_size(Vec2::new(0.8, 0.15))
			.with_style(&theme.title_label)
			.with_z_index(10);
		self.add_element(title);

		// Settings panel
		let list_panel = UIElement::panel(self.next_id())
			.with_position(Vec2::new(-0.6, -0.4))
			.with_size(Vec2::new(1.2, 0.9))
			.with_style(&theme.panels.basic)
			.with_z_index(1);
		self.add_element(list_panel);

		let core_label = UIElement::label(self.next_id(), "Multithreading".into())
			.with_position(Vec2::new(-0.4, 0.14))
			.with_size(Vec2::new(0.55, 0.06))
			.with_style(&theme.labels.basic)
			.with_z_index(6);
		self.add_element(core_label);
		let core_slider = {
			if let Some(core_count) = std::thread::available_parallelism().ok() {
				let id = self.next_id();
				UIElement::slider(id, 1.0, core_count.get() as f32 - 1.0)
					.with_value((core_count.get() / 2 -1).max(4).min(1) as f32)
					.with_callback(move || {
						if !ptr::get_state().is_world_running { return; }

						let data = get_element_num_by_id(&id);
						let world = ptr::get_gamestate().world_mut();
						world.stop_generation_threads();
						world.start_generation_threads(data as u8);
					})
			} else {
				UIElement::slider(self.next_id(), 0.0, 1.0)
					.with_enabled(false)
					.with_value(0.0)
			}
			.with_position(Vec2::new(-0.4, 0.06))
			.with_size(Vec2::new(0.8, 0.08))
			.with_style(&theme.sliders.basic)
			.with_z_index(5)
			.with_step(1.0)
		};
		self.add_element(core_slider);

		let fgvolume_label = UIElement::label(self.next_id(), "Foreground volume".into())
			.with_position(Vec2::new(-0.4, -0.04))
			.with_size(Vec2::new(0.55, 0.06))
			.with_style(&theme.labels.basic)
			.with_z_index(6);
		self.add_element(fgvolume_label);
		let id = self.next_id();
		let fgvolume_slider = UIElement::slider(id, settings.music_settings.fg_volume.min, settings.music_settings.fg_volume.max)
			.with_position(Vec2::new(-0.4, -0.12))
			.with_size(Vec2::new(0.8, 0.08))
			.with_style(&theme.sliders.basic)
			.with_z_index(5)
			//.with_step(0.5)
			.with_value(settings.music_settings.fg_volume.val)
			.with_callback(move || {
				let data = get_element_num_by_id(&id);
				ptr::get_settings().music_settings.fg_volume.set(data);
				set_fg_volume(data);
			});
		self.add_element(fgvolume_slider);

		let bgvolume_label = UIElement::label(self.next_id(), "Background volume".into())
			.with_position(Vec2::new(-0.4, -0.22))
			.with_size(Vec2::new(0.55, 0.06))
			.with_style(&theme.labels.basic)
			.with_z_index(6);
		self.add_element(bgvolume_label);
		let id = self.next_id();
		let bgvolume_slider = UIElement::slider(id, settings.music_settings.bg_volume.min, settings.music_settings.bg_volume.max)
			.with_position(Vec2::new(-0.4, -0.3))
			.with_size(Vec2::new(0.8, 0.08))
			.with_style(&theme.sliders.basic)
			.with_z_index(5)
			//.with_step(0.5)
			.with_value(settings.music_settings.bg_volume.val)
			.with_callback(move || {
				let data = get_element_num_by_id(&id);
				ptr::get_settings().music_settings.bg_volume.set(data);
				set_bg_volume(data);
			});
		self.add_element(bgvolume_slider);

		// Back button
		let back_button = UIElement::button(self.next_id(), "Back".into())
			.with_position(Vec2::new(-0.1, -0.8))
			.with_size(Vec2::new(0.2, 0.08))
			.with_style(&theme.buttons.extra())
			.with_z_index(8)
			.with_callback(|| close_pressed());
		self.add_element(back_button);
	}

}
