
use glam::Vec2;
use crate::ext::ptr;
use crate::ext::audio::set_bg_volume;
use crate::ui::manager::{close_pressed, UIManager, get_element_num_by_id};
use crate::ui::element::UIElement;

impl UIManager {
	#[inline]
	pub fn setup_settings_ui(&mut self) {
		let theme = &ptr::get_settings().ui_theme;
		let settings = &ptr::get_settings();
		// Title
		let title = UIElement::label(self.next_id(), "Settings ... yah")
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

		let volume_label = UIElement::label(self.next_id(), "Background volume")
			.with_position(Vec2::new(-0.4, -0.05))
			.with_size(Vec2::new(0.55, 0.08))
			.with_style(&theme.labels.basic)
			.with_z_index(6);
		self.add_element(volume_label);
		let id = self.next_id();
		let volume_slider = UIElement::slider(id, settings.music_settings.bg_volume.min, settings.music_settings.bg_volume.max)
			.with_position(Vec2::new(-0.4, -0.15))
			.with_size(Vec2::new(0.8, 0.1))
			.with_style(&theme.sliders.basic)
			.with_z_index(5)
			//.with_step(0.5)
			.with_value(settings.music_settings.bg_volume.val)
			.with_callback(move || {
				let data = get_element_num_by_id(&id);
				ptr::get_settings().music_settings.bg_volume.set(data);
				set_bg_volume(data);
			});
		self.add_element(volume_slider);




		// Back button
		let back_button = UIElement::button(self.next_id(), "Back")
			.with_position(Vec2::new(-0.1, -0.8))
			.with_size(Vec2::new(0.2, 0.08))
			.with_style(&theme.buttons.extra())
			.with_z_index(8)
			.with_callback(|| close_pressed());
		self.add_element(back_button);
	}

}
