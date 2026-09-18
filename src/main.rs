#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![feature(portable_simd)]

use crate::{
	audio::{AudioSubsystem, PlaybackState},
	platform::{AppEvent, EventLoop, get_current_theme},
	tray_icon::TrayIconSubsystem,
};
use tracing::*;
use tracing_subscriber::EnvFilter;

mod audio;
mod platform;
mod tray_icon;
mod utils;

fn main() -> anyhow::Result<()> {
	tracing_subscriber::fmt()
		.with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
		.init();

	let event_loop = EventLoop::new();

	let mut tray_icon = TrayIconSubsystem::new(event_loop.create_event_proxy())?;
	let mut audio = AudioSubsystem::new(event_loop.create_event_proxy())?;

	let _span = span!(Level::INFO, "Event Loop");
	loop {
		match event_loop.pump() {
			Ok(AppEvent::PlaybackToggle) => {
				info!("Toggling Playback");
				audio.toggle_playback()?;
				match audio.get_playback_state()? {
					PlaybackState::Playing => tray_icon.set_tooltip("Playing")?,
					PlaybackState::Paused => tray_icon.set_tooltip("Paused")?,
				}
			},
			Ok(AppEvent::AudioDeviceSwitched) => {
				info!("Default device switched, regenerating stream and toggling playback");
				audio.regenerate_stream()?;
				audio.toggle_playback()?;
			},

			Ok(AppEvent::SystemThemeChanged) => {
				let t = get_current_theme()?;
				info!("System theme changed: {:#?}", t);
				tray_icon.set_theme(t)?
			},

			Ok(AppEvent::QuitApplication) => {
				info!("Quit application");
				break;
			},

			_ => {},
		}
	}
	drop(_span);

	Ok(())
}
