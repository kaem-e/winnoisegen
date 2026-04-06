#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![feature(portable_simd)]
#![allow(unused)]

use ::tray_icon::TrayIconEvent;
use log::*;
use windows::Win32::{
	Foundation::WPARAM,
	UI::WindowsAndMessaging::{
		DispatchMessageA, DispatchMessageW, GetMessageA, GetMessageW, MSG, PM_REMOVE, PeekMessageW, PostQuitMessage, TranslateMessage, WM_APP, WaitMessage
	},
};

use crate::{
	audio::AudioSubsystem,
	tray_icon::{TRAY_ICON_EVENT, TrayIconSubsystem},
};

mod audio;
mod tray_icon;
mod utils;

fn main() -> anyhow::Result<()> {
	env_logger::builder()
		.filter_level(log::LevelFilter::Info)
		.init();

	let tray_icon = TrayIconSubsystem::new()?;
	let mut audio = AudioSubsystem::new()?;

	unsafe {
		let mut msg = MSG::default();
		while GetMessageW(&mut msg, None, 0, 0).into() {
			TranslateMessage(&msg);
			DispatchMessageW(&msg);

			match (msg.message, msg.wParam, msg.hwnd) {
				(msg @ TRAY_ICON_EVENT, WPARAM(p @ 0), _) => {
					info!("Left click");
					audio.toggle_playback()?
				},
				(msg @ TRAY_ICON_EVENT, WPARAM(p @ 1), _) => {
					info!("Right click");
					PostQuitMessage(0);
					// return Ok(());
				},
				_ => continue,
			}
		}
	}

	drop(tray_icon);
	drop(audio);
	Ok(())
}
