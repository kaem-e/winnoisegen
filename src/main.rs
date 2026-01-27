#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![feature(float_minimum_maximum, portable_simd)]
#![allow(unused)]

// use cpal::{
// 	self, Host,
// 	traits::{DeviceTrait, HostTrait, StreamTrait},
// };
// use egui_winit;
// use rand;
// use egui;
use winit::{
	self,
	application::ApplicationHandler,
	event::WindowEvent,
	event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
	window::{Window, WindowId},
};

// fn main() -> Result<(), anyhow::Error> {
// 	let device = Host::default()
// 		.default_output_device()
// 		.ok_or_else(|| anyhow::Error::msg("Failed to get default output device"))?;

// 	let config = device
// 		.supported_output_configs()?
// 		.next()
// 		.ok_or_else(|| anyhow::Error::msg("No supported stream config"))?
// 		.with_max_sample_rate()
// 		.config();

// 	let stream = device.build_output_stream(
// 		&config,
// 		move |b: &mut [f32], _| {
// 			// Deinterleave mono
// 			for i in b.chunks_mut(config.channels as usize) {
// 				let s = rand::random::<f32>();
// 				i.fill(s);
// 			}

// 			// for i in a {
// 			// 	*i = rand::random::<f32>();
// 			// }
// 		},
// 		|_| {},
// 		None,
// 	)?;

// 	stream.play()?;

// 	std::thread::sleep(std::time::Duration::from_millis(2000));

// 	Ok(())
// }

fn main() -> Result<(), anyhow::Error> {
	let event_loop = EventLoop::new()?;
	event_loop.set_control_flow(ControlFlow::Wait);

	let mut app = App::default();
	event_loop.run_app(&mut app);

	Ok(())
}

#[derive(Default)]
struct App {
	egui_ctx: egui::Context,
	window: Option<Window>,
}

impl ApplicationHandler for App {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		self.window = Some(
			event_loop
				.create_window(Window::default_attributes())
				.unwrap(),
		);
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
		match event {
			WindowEvent::CloseRequested => {
				println!("The close button was pressed; stopping");
				event_loop.exit();
			},
			WindowEvent::RedrawRequested => {
				// Redraw the application.
				//
				// It's preferable for applications that do not render continuously to render in
				// this event rather than in AboutToWait, since rendering in here allows
				// the program to gracefully handle redraws requested by the OS.

				// Draw.

				// Queue a RedrawRequested event.
				//
				// You only need to call this if you've determined that you need to redraw in
				// applications which do not always need to. Applications that redraw continuously
				// can render here instead.
				self.window.as_ref().unwrap().request_redraw();
			},
			a => {
				dbg!(a);
			},
		}
	}
}
