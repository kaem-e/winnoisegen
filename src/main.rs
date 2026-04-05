#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![feature(portable_simd)]
// #![allow(unused)]

use tracing_subscriber::{filter::LevelFilter, fmt, prelude::*};
use winit::event_loop::{ControlFlow, EventLoop};

mod app;
mod subsystems;
mod utils;

use crate::app::{App, UserEvent};

fn main() -> anyhow::Result<()> {
	let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());

	tracing_subscriber::registry()
		.with(fmt::layer().with_writer(non_blocking))
		.with(LevelFilter::INFO) // This replaces your .filter_level(LevelFilter::Info)
		.init();

	tracing::info!("This will show up!");
	tracing::debug!("This will be filtered out.");

	let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
	event_loop.set_control_flow(ControlFlow::Wait);

	let proxy = event_loop.create_proxy();

	let mut app = App::new_uninitialized(proxy)?;
	event_loop.run_app(&mut app)?;

	Ok(())
}
