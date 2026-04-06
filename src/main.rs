#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![feature(portable_simd)]
#![allow(unused)]

use ::tray_icon::TrayIconEvent;
use log::*;
use windows::Win32::UI::WindowsAndMessaging::{
	DispatchMessageA, DispatchMessageW, GetMessageA, GetMessageW, MSG, PM_REMOVE, PeekMessageW,
	TranslateMessage, WaitMessage,
};

use crate::tray_icon::TrayIconSubsystem;

mod audio;
mod tray_icon;
mod utils;

fn main() -> anyhow::Result<()> {
	env_logger::builder()
		.filter_level(log::LevelFilter::Info)
		.init();

	let tray_icon = TrayIconSubsystem::new()?;

	unsafe {
		let mut msg = MSG::default();
		'main: loop {
			// 1. Process ALL pending Windows messages
			while PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE).into() {
				TranslateMessage(&msg);
				DispatchMessageW(&msg);
			}

			for event in TrayIconEvent::receiver().try_iter() {
				match event {
					TrayIconEvent::Click {
						button,
						button_state,
						..
					} => {
						dbg_log!(button, button_state);
					},
					_ => {},
				}
			}

			// 3. Prevent 100% CPU usage
			// If there were no messages and no events, wait for the next system event
			// Or simply use: std::thread::sleep(std::time::Duration::from_millis(5));
			WaitMessage().ok();
		}
	}

	// drop(tray_icon);
	// Ok(())
}

// unsafe {
// 	let mut msg = MSG::default();
// 	while GetMessageW(&mut msg, None, 0, 0).into() {
// 		// We skip TranslateMessage because we don't care about text input/WM_CHAR
// 		DispatchMessageW(&msg);

// 		// Check our tray events
// 		for event in TrayIconEvent::receiver().try_iter() {
// 			match event {
// 				TrayIconEvent::Click {
// 					position,
// 					rect,
// 					button,
// 					button_state,
// 					..
// 				} => {
// 					dbg_log!(position, rect, button, button_state);
// 				},
// 				TrayIconEvent::DoubleClick {
// 					position,
// 					rect,
// 					button,
// 					..
// 				} => {
// 					dbg_log!(position, rect, button);
// 				},
// 				_ => {},
// 			};
// 			// dbg_log!("event received");
// 		}
// 		// WaitMessage().ok();
// 	}
// }
