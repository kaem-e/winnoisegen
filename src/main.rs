#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![feature(portable_simd)]
// #![allow(unused)]

mod app;
mod subsystems;
mod utils;

use log::LevelFilter;
use winit::event_loop::{ControlFlow, EventLoop};

use crate::app::{App, UserEvent};

fn main() -> anyhow::Result<()> {
	env_logger::builder().filter_level(LevelFilter::Info).init();

	let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
	event_loop.set_control_flow(ControlFlow::Wait);

	let proxy = event_loop.create_proxy();

	let mut app = App::new_uninitialized(proxy)?;
	event_loop.run_app(&mut app)?;

	Ok(())
}
