use anyhow::{Context, anyhow};
use tracing::{error, info};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconAttributes, TrayIconEvent};
use windows::{
	Win32::{
		Foundation::{HWND, LPARAM, WPARAM},
		System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW},
		UI::WindowsAndMessaging::{PostMessageW, WM_APP},
	},
	core::h,
};

use crate::audio::PlaybackState;

/// Named Enumeration for theme variants
#[derive(PartialEq, Eq, Debug)]
enum Theme {
	Light,
	Dark,
}

/// Subsystem that controls everything related to the tray icon itself
pub struct TrayIconSubsystem {
	icon_dark: Icon,
	icon_light: Icon,
	theme: Theme,

	tray_icon: TrayIcon,
}

/// `msg` range on the windows message type that the tray icon sends its events to
pub const TRAY_ICON_EVENT: u32 = WM_APP + 1;
/// wparam value corresponding to a single right click event
pub const EVENT_RIGHT_CLICK: usize = 1;
/// wparam value corresponding to a single left click event
pub const EVENT_LEFT_CLICK: usize = 0;

impl TrayIconSubsystem {
	/// Initializes the tray and tray icon's.
	///
	/// Events produced by the tray icon are sent to win32's event queue.
	/// Which should be queried independently
	///
	/// Example:
	/// ```rust
	/// // set up a custom event loop to receive tray_icon events
	/// unsafe {
	///    let mut msg = MSG::default();
	///    while GetMessageW(&mut msg, None, 0, 0).into() {
	///       DispatchMessageW(&msg);
	///
	///       match (msg.message, msg.wParam) {
	///          (_msg @ TRAY_ICON_EVENT, WPARAM(_p @ EVENT_LEFT_CLICK)) => audio.toggle_playback()?,
	///          (_msg @ TRAY_ICON_EVENT, WPARAM(_p @ EVENT_RIGHT_CLICK)) => PostQuitMessage(0),
	///
	///          // these are received on theme change. we get multiple so like, yeah either debounce or do conditional checks
	///          (0x320, _) => tray_icon.sync_system_scheme()?,
	///          _ => continue,
	///       }
	///    }
	/// }
	/// ```
	pub fn new() -> anyhow::Result<Self> {
		let icon_dark = {
			let image = image::open(std::path::Path::new(concat!(
				env!("CARGO_MANIFEST_DIR"),
				"/assets/Icon.png"
			)))?
			.into_rgba8();

			let (width, height) = image.dimensions();
			let rgba = image.into_raw();
			Icon::from_rgba(rgba, width, height).expect("Failed to open icon")
		};
		let icon_light = {
			let image = image::open(std::path::Path::new(concat!(
				env!("CARGO_MANIFEST_DIR"),
				"/assets/Icon-light.png"
			)))?
			.into_rgba8();

			let (width, height) = image.dimensions();
			let rgba = image.into_raw();
			Icon::from_rgba(rgba, width, height).expect("Failed to open icon")
		};

		let tray_icon = TrayIcon::new(TrayIconAttributes {
			tooltip: Some(format!("Playback: {:#?}", PlaybackState::Paused)),
			title: Some(String::from("ambience")),
			icon: None,
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
			match PostMessageW(Some(HWND(hwnd as _)), WM_APP + 1, WPARAM(w), LPARAM(0)) {
				Ok(()) => {},
				Err(e) => error!("Failed pushing to Win32 Queue: {:#?}", e),
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
	/// See comment at definition if you want to know why the `tray_icon` field isnt just pub
	pub fn set_tooltip<S: AsRef<str>>(&self, tooltip: Option<S>) -> anyhow::Result<()> {
		// Same as audio subsystem, making this pub can mean user can set the icon
		// to something other than what... hmm actually they cant but whatever idk
		// this is better
		self.tray_icon.set_tooltip(tooltip)?;
		Ok(())
	}

	/// Changes the tray icon to match the current system theme,
	/// Call this whenever the theme on your system changes
	pub fn sync_system_scheme(&mut self) -> anyhow::Result<()> {
		let theme = match get_current_theme() {
			Ok(t) => t,
			Err(e) => {
				error!("Error While trying to get theme: {e}");
				return Ok(());
			},
		};

		if theme != self.theme {
			match theme {
				Theme::Light => self.tray_icon.set_icon(Some(self.icon_dark.clone()))?,
				Theme::Dark => self.tray_icon.set_icon(Some(self.icon_light.clone()))?,
			}
			self.theme = theme;
			info!("Set Current Theme to: {:#?}", &self.theme);
		}
		Ok(())
	}
}

/// Queries systsem theme registry key and returns the theme as an enumerated variant,
///
/// Fails if there were any errors either retriving the regkey entry, or the
/// returned value was a variant that doesnt make sense and is hence invalid
fn get_current_theme() -> anyhow::Result<Theme> {
	unsafe {
		let mut data: u32 = 0;

		let mut _len = std::mem::size_of::<u32>() as u32;
		RegGetValueW(
			HKEY_CURRENT_USER,
			h!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
			h!("AppsUseLightTheme"),
			RRF_RT_REG_DWORD,
			None,
			Some(&mut data as *mut _ as *mut _),
			Some(&mut _len),
		)
		.ok()
		.context("Failed to read registry value for theme")?;

		match data {
			0 => Ok(Theme::Dark),
			1 => Ok(Theme::Light),
			n => Err(anyhow!("Unrecognized Theme Value: {n}")),
		}
	}
}
