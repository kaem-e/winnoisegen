#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![feature(portable_simd)]

#[cfg(not(target_os = "windows"))]
compile_error!("This application only supports Windows.");

use tracing::*;
use windows::Win32::{
	Foundation::WPARAM,
	UI::WindowsAndMessaging::{DispatchMessageW, GetMessageW, MSG, PostQuitMessage},
};

use crate::{
	audio::{AudioSubsystem, EVENT_DEFAULT_DEVICE_SWITCHED, EVENTGROUP_AUDIO},
	tray_icon::{EVENT_LEFT_CLICK, EVENT_RIGHT_CLICK, EVENTGROUP_TRAYICON, TrayIconSubsystem},
};

mod audio;
mod tray_icon;
mod utils;

// Use this id to send messages to the main thread where the event loop is running with SendThreadMessageW
pub static MAIN_THREAD_ID: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

fn main() -> anyhow::Result<()> {
	tracing_subscriber::fmt()
		.with_max_level(tracing::Level::DEBUG)
		.init();

	let main_id = unsafe { windows::Win32::System::Threading::GetCurrentThreadId() };
	MAIN_THREAD_ID.store(main_id, std::sync::atomic::Ordering::Release);

	let mut tray_icon = TrayIconSubsystem::new()?;

	let mut audio = AudioSubsystem::new()?;
	audio.regenerate_stream()?;

	// set up a custom event loop to receive tray_icon events
	unsafe {
		let mut msg = MSG::default();
		while GetMessageW(&mut msg, None, 0, 0).into() {
			DispatchMessageW(&msg);

			match (msg.message, msg.wParam) {
				(_msg @ EVENTGROUP_AUDIO, WPARAM(_p @ EVENT_DEFAULT_DEVICE_SWITCHED)) => {
					info!("Default device switched, regenerating stream and toggling playback");
					audio.regenerate_stream()?;
					audio.toggle_playback()?;
				},

				(_msg @ EVENTGROUP_TRAYICON, WPARAM(_p @ EVENT_LEFT_CLICK)) => {
					debug!("tray icon left clicked, toggling playback");
					audio.toggle_playback()?;
					tray_icon
						.set_tooltip(Some(format!("Playback: {:#?}", audio.get_playback_state())))?;
				},
				(_msg @ EVENTGROUP_TRAYICON, WPARAM(_p @ EVENT_RIGHT_CLICK)) => {
					debug!("tray icon right clicked, posting PostQuitMessage to quit application");
					PostQuitMessage(0)
				},

				// these are received on theme change. we get multiple so like, yeah either debounce or do conditional checks
				(0x320, _) => tray_icon.sync_system_scheme()?,
				_ => continue,
			}
		}
	}

	Ok(())
}
