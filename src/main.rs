#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![feature(portable_simd)]

use windows::{
	Win32::{
		Foundation::WPARAM,
		System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW},
		UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG, PostQuitMessage},
	},
	core::h,
};

use crate::{
	audio::AudioSubsystem,
	tray_icon::{EVENT_LEFT_CLICK, EVENT_RIGHT_CLICK, TRAY_ICON_EVENT, Theme, TrayIconSubsystem},
};

mod audio;
mod tray_icon;
mod utils;

fn get_current_theme() -> Theme {
	use log::error;

	let theme = unsafe {
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
		.map(|_| data)
	};

	// If result is successful, 0 means Dark Mode, 1 means Light Mode
	// result.is_ok() && data == 0
	// TODO: Ideally we should just return a result from this function as well and let tray_icon handle this part
	match theme {
		Ok(0) => Theme::Dark,
		Ok(1) => Theme::Light,
		Err(e) => {
			error!("Failed to get current theme regkey: {:#?}", e);
			Theme::Dark
		},
		_ => Theme::Dark,
	}
}

fn main() -> anyhow::Result<()> {
	env_logger::builder()
		.filter_level(log::LevelFilter::Info)
		.init();

	let mut tray_icon = TrayIconSubsystem::new(get_current_theme())?;
	let mut audio = AudioSubsystem::new()?;

	unsafe {
		let mut msg = MSG::default();
		while GetMessageW(&mut msg, None, 0, 0).into() {
			DispatchMessageW(&msg);

			match (msg.message, msg.wParam) {
				(_msg @ TRAY_ICON_EVENT, WPARAM(_p @ EVENT_LEFT_CLICK)) => audio.toggle_playback()?,
				(_msg @ TRAY_ICON_EVENT, WPARAM(_p @ EVENT_RIGHT_CLICK)) => PostQuitMessage(0),

				// these are received on theme change. we get multiple so like, yeah either debounce or do conditional checks
				(0x320, _) => tray_icon.set_theme(get_current_theme())?,
				_ => continue,
			}
		}
	}

	drop(tray_icon);
	drop(audio);
	Ok(())
}
