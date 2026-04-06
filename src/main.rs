#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![feature(portable_simd)]
// #![allow(unused)]

mod audio;
mod tray_icon;
mod utils;

fn main() -> anyhow::Result<()> {
	env_logger::builder()
		.filter_level(log::LevelFilter::Info)
		.init();

	Ok(())
}
