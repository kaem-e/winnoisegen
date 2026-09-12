use anyhow::Context as _;
use tracing::*;
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconAttributes, TrayIconEvent};

use crate::{
	audio::PlaybackState,
	platform::{AppEvent, EventProxy, Theme, get_current_theme},
};

/// Subsystem that controls everything related to the tray icon itself
pub struct TrayIconSubsystem {
	icon_dark: Icon,
	icon_light: Icon,
	theme: Theme,

	tray_icon: TrayIcon,
}

impl TrayIconSubsystem {
	/// Initializes the tray and tray icon's.
	///
	/// Events produced by the tray icon are sent to Win32's event queue.
	/// Which should be queried independently
	///
	/// Example:
	/// ```rust
	/// ```
	pub fn new(proxy: EventProxy) -> anyhow::Result<Self> {
		let icon_dark = {
			let image = image::open(std::path::Path::new(concat!(
				env!("CARGO_MANIFEST_DIR"),
				"/assets/icon-dark.png"
			)))?
			.into_rgba8();

			let (width, height) = image.dimensions();
			let rgba = image.into_raw();
			Icon::from_rgba(rgba, width, height).expect("Failed to open icon")
		};
		let icon_light = {
			let image = image::open(std::path::Path::new(concat!(
				env!("CARGO_MANIFEST_DIR"),
				"/assets/icon-light.png"
			)))?
			.into_rgba8();

			let (width, height) = image.dimensions();
			let rgba = image.into_raw();
			Icon::from_rgba(rgba, width, height).expect("Failed to open icon")
		};

		let tray_icon = TrayIcon::new(TrayIconAttributes {
			tooltip: Some(format!("Playback: {:#?}", PlaybackState::Paused)),
			title: Some(String::from("winnoisegen")),
			icon: None,
			..Default::default()
		})?;

		TrayIconEvent::set_event_handler(Some(move |event| {
			match event {
				TrayIconEvent::Click {
					button: MouseButton::Left,
					button_state: MouseButtonState::Up,
					..
				} => {
					proxy
						.send_event(AppEvent::PlaybackToggle)
						.context("error sending event to the proxy")
						.unwrap();
				},
				TrayIconEvent::Click {
					button: MouseButton::Right,
					button_state: MouseButtonState::Down,
					..
				} => {
					proxy
						.send_event(AppEvent::QuitApplication)
						.context("error sending event to the proxy")
						.unwrap();
				},
				_ => {},
			};
		}));

		// set icon corresponding to current theme
		let theme = get_current_theme()?;
		match theme {
			Theme::Light => tray_icon.set_icon(Some(icon_dark.clone()))?,
			Theme::Dark => tray_icon.set_icon(Some(icon_light.clone()))?,
		}

		Ok(Self {
			icon_dark,
			icon_light,
			theme,

			tray_icon,
		})
	}

	/// Sets the tooltip for this tray icon.
	/// See the comment at the definition if you want to know why the `tray_icon` field isn't just pub
	pub fn set_tooltip(&self, tooltip: &str) -> anyhow::Result<()> {
		self.tray_icon.set_tooltip(tooltip.into())?;
		Ok(())
	}

	/// Changes the tray icon to match the current system theme,
	/// Call this whenever the theme on your system changes
	pub fn set_theme(&mut self, theme: Theme) -> anyhow::Result<()> {
		if theme != self.theme {
			match theme {
				Theme::Light => self.tray_icon.set_icon(Some(self.icon_dark.clone()))?,
				Theme::Dark => self.tray_icon.set_icon(Some(self.icon_light.clone()))?,
			}
			self.theme = theme;
			debug!("Set Current Theme to: {:#?}", &self.theme);
		}
		Ok(())
	}
}
