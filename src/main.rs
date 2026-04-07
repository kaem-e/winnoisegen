#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![feature(portable_simd)]


#[cfg(not(target_os = "windows"))]
compile_error!("This application only supports Windows.");

// Potential TODO's:
// - Isolate all the windows specific code into its own platform module,
//   > honestly cba becase like we only need 2 or three things that are heavily tied
//   > to individual subsystems, so i like having them be actually in those
//   > subsystems
//
// - Move from fragile consts for message variants to proper enum named variants
//   > yeah.

use windows::Win32::{
	Foundation::WPARAM,
	UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG, PostQuitMessage},
};

use crate::{
	audio::AudioSubsystem,
	tray_icon::{EVENT_LEFT_CLICK, EVENT_RIGHT_CLICK, TRAY_ICON_EVENT, TrayIconSubsystem},
};

mod audio;
mod tray_icon;
mod utils;

fn main() -> anyhow::Result<()> {
	env_logger::builder()
		.filter_level(log::LevelFilter::Info)
		.init();

	let mut tray_icon = TrayIconSubsystem::new()?;

	let mut audio = AudioSubsystem::new()?;

	// set up a custom event loop to receive tray_icon events
	unsafe {
		let mut msg = MSG::default();
		while GetMessageW(&mut msg, None, 0, 0).into() {
			DispatchMessageW(&msg);

			match (msg.message, msg.wParam) {
				(_msg @ TRAY_ICON_EVENT, WPARAM(_p @ EVENT_LEFT_CLICK)) => {
					audio.toggle_playback()?;
					tray_icon
						.set_tooltip(Some(format!("Playback: {:#?}", audio.get_playback_state())))?;
				},
				(_msg @ TRAY_ICON_EVENT, WPARAM(_p @ EVENT_RIGHT_CLICK)) => PostQuitMessage(0),

				// these are received on theme change. we get multiple so like, yeah either debounce or do conditional checks
				(0x320, _) => tray_icon.sync_system_scheme()?,
				_ => continue,
			}
		}
	}

	drop(tray_icon);
	drop(audio);
	Ok(())
}
