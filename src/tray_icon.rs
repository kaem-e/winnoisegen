use std::path::Path;

use image::open;
use tray_icon::{Icon, TrayIcon, TrayIconAttributes, TrayIconEvent, TrayIconEventReceiver};
use windows::Win32::Foundation::HWND;

pub struct TrayIconSubsystem {
	icon_dark: Icon,
	icon_light: Icon,

	tray_icon: TrayIcon,
}

impl TrayIconSubsystem {
	pub fn new() -> anyhow::Result<Self> {
		let icon_dark = {
			let image = open(Path::new(concat!(
				env!("CARGO_MANIFEST_DIR"),
				"/assets/Icon.png"
			)))?
			.into_rgba8();

			let (width, height) = image.dimensions();
			let rgba = image.into_raw();
			Icon::from_rgba(rgba, width, height).expect("Failed to open icon")
		};
		let icon_light = {
			let image = open(Path::new(concat!(
				env!("CARGO_MANIFEST_DIR"),
				"/assets/Icon-light.png"
			)))?
			.into_rgba8();

			let (width, height) = image.dimensions();
			let rgba = image.into_raw();
			Icon::from_rgba(rgba, width, height).expect("Failed to open icon")
		};

		let tray_icon = TrayIcon::new(TrayIconAttributes {
			tooltip: Some(String::from("penis balls tooltip")),
			title: Some(String::from("ambience")),
			icon: Some(icon_dark.clone()),
			..Default::default()
		})?;

		// HWND(tray_icon.window_handle());

		// // give event loop proxy to tray-icon and menu items within tray-icon
		// TrayIconEvent::set_event_handler(Some(move |event| {
		// 	proxy.send_event(UserEvent::TrayIconEvent(event)).log_err();
		// }));

		Ok(Self {
			icon_dark,
			icon_light,
			tray_icon,
		})
	}

	pub fn set_theme(&self) -> anyhow::Result<()> {
		// 	match theme {
		// 		Theme::Light => self.tray_icon.set_icon(Some(self.icon_dark.clone()))?,
		// 		Theme::Dark => self.tray_icon.set_icon(Some(self.icon_light.clone()))?,
		// 	}
		Ok(())
	}
}
