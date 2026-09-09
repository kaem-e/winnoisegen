#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![feature(portable_simd)]

#[cfg(not(target_os = "windows"))]
compile_error!("This application only supports Windows.");


use windows::Win32::{
	Foundation::WPARAM,
	UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG, PostQuitMessage},
};

use crate::{
	audio::AudioSubsystem,
	tray_icon::{EVENT_LEFT_CLICK, EVENT_RIGHT_CLICK, EVENTGROUP_TRAYICON, TrayIconSubsystem},
};

mod audio;
mod tray_icon;
mod utils;

fn main() -> anyhow::Result<()> {
	tracing_subscriber::fmt()
		.with_max_level(tracing::Level::INFO)
		.init();

	let mut tray_icon = TrayIconSubsystem::new()?;

	let mut audio = AudioSubsystem::new()?;

	// set up a custom event loop to receive tray_icon events
	unsafe {
		let mut msg = MSG::default();
		while GetMessageW(&mut msg, None, 0, 0).into() {
			DispatchMessageW(&msg);

			match (msg.message, msg.wParam) {
				(_msg @ EVENTGROUP_TRAYICON, WPARAM(_p @ EVENT_LEFT_CLICK)) => {
					audio.toggle_playback()?;
					tray_icon
						.set_tooltip(Some(format!("Playback: {:#?}", audio.get_playback_state())))?;
				},
				(_msg @ EVENTGROUP_TRAYICON, WPARAM(_p @ EVENT_RIGHT_CLICK)) => PostQuitMessage(0),

				// these are received on theme change. we get multiple so like, yeah either debounce or do conditional checks
				(0x320, _) => tray_icon.sync_system_scheme()?,
				_ => continue,
			}
		}
	}

	Ok(())
}
