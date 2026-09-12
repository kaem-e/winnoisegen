use anyhow::{self, Context as _, Ok};

use windows::{
	Win32::{
		Foundation::*,
		System::{Registry::*, Threading::GetCurrentThreadId},
		UI::WindowsAndMessaging::{WM_APP, *},
	},
	core::*,
};

/// Named Enumeration for theme variants
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[repr(u8)]
pub enum Theme {
	Light,
	Dark,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum AppEvent {
	PlaybackToggle,
	AudioDeviceSwitched,
	SystemThemeChanged,
	QuitApplication,
}

const PLAYBACK_TOGGLE: u32 = WM_APP + 0;
const AUDIO_DEVICE_SWITCHED: u32 = WM_APP + 1;
const SYSTEM_THEME_CHANGED: u32 = WM_APP + 2;
const QUIT_APPLICATION: u32 = WM_APP + 3;

impl TryFrom<&MSG> for AppEvent {
	type Error = anyhow::Error;

	fn try_from(value: &MSG) -> std::prelude::v1::Result<Self, Self::Error> {
		match value.message {
			PLAYBACK_TOGGLE => Ok(AppEvent::PlaybackToggle),
			AUDIO_DEVICE_SWITCHED => Ok(AppEvent::AudioDeviceSwitched),
			SYSTEM_THEME_CHANGED => Ok(AppEvent::SystemThemeChanged),
			QUIT_APPLICATION => Ok(AppEvent::QuitApplication),

			// these are received on theme change. we get multiple so like, yeah
			// either debounce or do conditional checks
			0x320 => Ok(AppEvent::SystemThemeChanged),

			_ => anyhow::bail!("Invalid input for message type"),
		}
	}
}

impl From<AppEvent> for u32 {
	fn from(value: AppEvent) -> Self {
		match value {
			AppEvent::PlaybackToggle => PLAYBACK_TOGGLE,
			AppEvent::AudioDeviceSwitched => AUDIO_DEVICE_SWITCHED,
			// AppEvent::VolumeIncrease => VOLUME_INCREASE,
			// AppEvent::VolumeDecrease => VOLUME_DECREASE,
			AppEvent::SystemThemeChanged => SYSTEM_THEME_CHANGED,
			AppEvent::QuitApplication => QUIT_APPLICATION,
		}
	}
}

pub struct EventLoop {
	thread_id: u32,
}

impl EventLoop {
	pub fn new() -> Self {
		Self {
			thread_id: unsafe { GetCurrentThreadId() },
		}
	}

	pub fn create_event_proxy(&self) -> EventProxy {
		// this is probably going to be a mpsc channel, though like lowkey on
		// windows it doesnt need to be all it really needs to be is something
		// that calls PostThreadMessageW
		EventProxy(self.thread_id)
	}

	pub fn pump(&self) -> Option<anyhow::Result<AppEvent>> {
		let mut msg = MSG::default();
		unsafe {
			if GetMessageW(&mut msg, None, 0, 0).into() {
				DispatchMessageW(&msg);
				Some(AppEvent::try_from(&msg))
			} else {
				None
			}
		}
	}
}

#[derive(Clone)]
pub struct EventProxy(u32);

impl EventProxy {
	pub fn send_event(&self, event: AppEvent) -> anyhow::Result<()> {
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
			n => Err(anyhow::anyhow!("Unrecognized Theme Value: {n}")),
		}
	}
}
