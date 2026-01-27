#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![feature(float_minimum_maximum, portable_simd, if_let_guard)]

use cpal::{
	self, Host, Stream,
	traits::{DeviceTrait, HostTrait, StreamTrait},
};
use winit::{
	self,
	application::ApplicationHandler,
	dpi::LogicalSize,
	event::{ElementState, KeyEvent, WindowEvent},
	event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
	keyboard::{KeyCode, PhysicalKey},
	window::{Window, WindowId},
};

fn main() -> Result<(), anyhow::Error> {
	let device = Host::default()
		.default_output_device()
		.ok_or_else(|| anyhow::Error::msg("Failed to get default output device"))?;

	let config = device
		.supported_output_configs()?
		.find_map(|a| {
			if a.max_sample_rate() == 48000 && cpal::SampleFormat::F32 == a.sample_format() {
				Some(a.with_max_sample_rate().config())
			} else {
				None
			}
		})
		.ok_or_else(|| anyhow::Error::msg("No supported stream config"))?;

	let stream = device.build_output_stream(
		&config,
		move |b: &mut [f32], _| {
			// // Deinterleave mono
			// for i in b.chunks_mut(config.channels as usize) {
			// 	i.fill(rand::random::<f32>());
			// }

			for i in b {
				*i = rand::random::<f32>();
			}
		},
		|_| {},
		None,
	)?;

	let event_loop = EventLoop::new()?;
	event_loop.set_control_flow(ControlFlow::Wait);

	let mut app = App::new(stream);
	event_loop.run_app(&mut app)?;

	Ok(())
}

struct App {
	// egui_ctx: egui::Context,
	cpal: (Stream, bool),
	window: Option<Window>,
}

impl App {
	pub fn new(cpal_stream: Stream) -> Self {
		Self {
			cpal: (cpal_stream, false),
			window: None,
		}
	}
}

impl ApplicationHandler for App {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		self.window = event_loop
			.create_window(
				Window::default_attributes()
					.with_min_inner_size(LogicalSize::new(400.0, 200.0))
					.with_max_inner_size(LogicalSize::new(800.0, 500.0))
					.with_title("Penis Balls Sex"),
			)
			.ok();
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
		match event {
			// close window with close event or by pressing escape
			WindowEvent::CloseRequested
			| WindowEvent::KeyboardInput {
				event:
					KeyEvent {
						physical_key: PhysicalKey::Code(KeyCode::Escape),
						state: ElementState::Pressed,
						..
					},
				..
			} => {
				event_loop.exit();
			},

			// spacebar to toggle play/pause
			WindowEvent::KeyboardInput {
				event:
					KeyEvent {
						physical_key: PhysicalKey::Code(KeyCode::Space),
						state: ElementState::Pressed,
						..
					},
				..
			} => {
				match dbg!(self.cpal.1) {
					true => self.cpal.0.pause().unwrap(),
					false => self.cpal.0.play().unwrap(),
				};
				self.cpal.1 = !self.cpal.1;
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
			_ => {},
		}
	}
}
