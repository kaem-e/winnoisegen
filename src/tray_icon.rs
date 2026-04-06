use std::path::Path;

use image::open;
use tray_icon::{
	Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconAttributes, TrayIconEvent,
	TrayIconEventReceiver,
};
use windows::Win32::{
	Foundation::{HWND, LPARAM, WPARAM},
	UI::WindowsAndMessaging::{PostMessageW, WM_APP},
};

pub struct TrayIconSubsystem {
	icon_dark: Icon,
	icon_light: Icon,

	tray_icon: TrayIcon,
}

pub const TRAY_ICON_EVENT: u32 = WM_APP + 1;


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

		// Make Tray Icon Events send out events to the win32 event loop
		// SAFETY: blah blah yes this should be wrapped in a rawwindowhandle i dont care.
		// the as usize is because this is a *mut c_void which cant be sent to a thread safely
		// but were only giving it to this one single thread so like, shut up
		let hwnd = tray_icon.window_handle() as usize;
		TrayIconEvent::set_event_handler(Some(move |event| unsafe {
			let w = match event {
				TrayIconEvent::Click {
					button: MouseButton::Left,
					button_state: MouseButtonState::Up,
					..
				} => 0,
				TrayIconEvent::Click {
					button: MouseButton::Right,
					button_state: MouseButtonState::Down,
					..
				} => 1,
				_ => return,
			};
			PostMessageW(Some(HWND(hwnd as _)), WM_APP + 1, WPARAM(w), LPARAM(0));
		}));

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
