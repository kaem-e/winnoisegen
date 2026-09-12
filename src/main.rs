#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![feature(portable_simd)]

#[cfg(not(target_os = "windows"))]
compile_error!("This application only supports Windows.");

use crate::{
	audio::{AudioSubsystem, PlaybackState},
	platform::{AppEvent, EventLoop, get_current_theme},
	tray_icon::TrayIconSubsystem,
};
use tracing::*;

mod audio;
mod platform;
mod tray_icon;
mod utils;

fn main() -> anyhow::Result<()> {
	tracing_subscriber::fmt()
		.with_max_level(tracing::Level::DEBUG)
		.init();

	let event_loop = EventLoop::new();

	let mut tray_icon = TrayIconSubsystem::new(event_loop.create_event_proxy())?;
	let mut audio = AudioSubsystem::new(event_loop.create_event_proxy())?;

	let _span = span!(Level::INFO, "Event Loop");
	while let Some(result) = event_loop.pump() {
		let Ok(event) = result else { continue };

		match event {
			AppEvent::PlaybackToggle => {
				info!("Toggling Playback");
				audio.toggle_playback()?;
				match audio.get_playback_state()? {
					PlaybackState::Playing => tray_icon.set_tooltip("Playing")?,
					PlaybackState::Paused => tray_icon.set_tooltip("Paused")?,
				}
			},
			AppEvent::AudioDeviceSwitched => {
				info!("Default device switched, regenerating stream and toggling playback");
				audio.regenerate_stream()?;
				audio.toggle_playback()?;
			},

			AppEvent::SystemThemeChanged => {
				let t = get_current_theme()?;
				info!("System theme changed: {:#?}", t);
				tray_icon.set_theme(t)?
			},

			AppEvent::QuitApplication => {
				info!("Quit application");
				break;
			},
		}
	}
	drop(_span);

	Ok(())
}
