use super::{AppEvent, EventLoopImpl, EventLoopProxyImpl, Theme};
use anyhow::{self, Context as _, Ok};

use tracing::*;
use windows::{
	Win32::{
		Foundation::{LPARAM, WPARAM},
		System::{Registry::*, Threading::GetCurrentThreadId},
		UI::WindowsAndMessaging::{WM_APP, *},
	},
	core,
};

const PLAYBACK_TOGGLE: u32 = WM_APP + 0;
const AUDIO_DEVICE_SWITCHED: u32 = WM_APP + 1;
const SYSTEM_THEME_CHANGED: u32 = WM_APP + 2;
const QUIT_APPLICATION: u32 = WM_APP + 3;

impl Into<u32> for AppEvent {
	fn into(self) -> u32 {
		match self {
			AppEvent::PlaybackToggle => PLAYBACK_TOGGLE,
			AppEvent::AudioDeviceSwitched => AUDIO_DEVICE_SWITCHED,
			AppEvent::SystemThemeChanged => SYSTEM_THEME_CHANGED,
			AppEvent::QuitApplication => QUIT_APPLICATION,
		}
	}
}

impl Into<MSG> for AppEvent {
	fn into(self) -> MSG {
		MSG {
			message: self.into(),
			..Default::default()
		}
	}
}

pub struct EventLoopWindows {
	thread_id: u32,
}

impl EventLoopImpl for EventLoopWindows {
	fn new() -> Self {
		Self {
			thread_id: unsafe { GetCurrentThreadId() },
		}
	}

	fn create_event_proxy(&self) -> EventLoopProxyWindows {
		// this is probably going to be a mpsc channel, though like lowkey on
		// windows it doesnt need to be all it really needs to be is something
		// that calls PostThreadMessageW
		EventLoopProxyWindows(self.thread_id)
	}

	fn pump(&self) -> anyhow::Result<AppEvent> {
		debug!("pump: thread_id={}", self.thread_id);
		unsafe {
			let mut msg = MSG::default();
			loop {
				if GetMessageW(&mut msg, None, 0, 0).into() {
					DispatchMessageW(&msg);

					return match msg.message {
						PLAYBACK_TOGGLE => Ok(AppEvent::PlaybackToggle),
						AUDIO_DEVICE_SWITCHED => Ok(AppEvent::AudioDeviceSwitched),
						SYSTEM_THEME_CHANGED => Ok(AppEvent::SystemThemeChanged),
						QUIT_APPLICATION => Ok(AppEvent::QuitApplication),

						// these are received on theme change. we get multiple so like, yeah
						// either debounce or do conditional checks
						0x320 => Ok(AppEvent::SystemThemeChanged),

						_ => continue,
					};
				} else {
					debug!("Quit Application because GetMessageW returned false");
					break;
				}
			}
			anyhow::bail!("Quit Application")
		}
	}
}

#[derive(Clone, Debug)]
pub struct EventLoopProxyWindows(u32);

impl EventLoopProxyImpl for EventLoopProxyWindows {
	fn send_event(&self, event: AppEvent) -> anyhow::Result<()> {
		unsafe {
			PostThreadMessageW(self.0, event.into(), WPARAM(0), LPARAM(0))?;
		}
		Ok(())
	}
}

/// Queries system theme registry key and returns the theme as an enumerated variant,
///
/// Fails if there were any errors either retrieving the regkey entry, or the
/// returned value was a variant that doesn't make sense and is hence invalid
pub fn get_current_theme() -> anyhow::Result<Theme> {
	unsafe {
		let mut data: u32 = 0;

		let mut _len = std::mem::size_of::<u32>() as u32;
		RegGetValueW(
			HKEY_CURRENT_USER,
			core::h!(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"),
			core::h!("AppsUseLightTheme"),
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
			n => Err(anyhow::anyhow!("Unrecognized Theme Value: {n}")),
		}
	}
}
