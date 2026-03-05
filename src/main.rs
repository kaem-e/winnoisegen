#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![feature(float_minimum_maximum, portable_simd)]
#![allow(unused)]

use cpal::{
	self, Host, Stream,
	traits::{DeviceTrait, HostTrait, StreamTrait},
};
use egui::{self, Context};
use egui_wgpu;
use egui_winit;
use pollster::FutureExt as _;
use std::sync::Arc;
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
	env_logger::init();

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

// ----------------------------- winit shit -----------------------------

struct App {
	cpal: (Stream, bool),
	wgpu_state: Option<WGPUState>,
	egui_state: Option<EGUIState>,
}

impl App {
	pub fn new(cpal_stream: Stream) -> Self {
		Self {
			cpal: (cpal_stream, false),
			wgpu_state: None,
			egui_state: None,
		}
	}
}

impl ApplicationHandler for App {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		let window = event_loop
			.create_window(
				Window::default_attributes()
					.with_min_inner_size(LogicalSize::new(400.0, 200.0))
					.with_max_inner_size(LogicalSize::new(800.0, 500.0))
					.with_title("Penis Balls Sex"),
			)
			.unwrap();

		// we are not on web so we just use pollster to calculate the async new fn
		// in place
		self.wgpu_state = Some(WGPUState::new(Arc::new(window)).block_on());
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
		let state = match &mut self.wgpu_state {
			Some(s) => s,
			None => return,
		};

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

			WindowEvent::Resized(s) => state.resize(s.width, s.height),

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
				// self.window.as_ref().unwrap().request_redraw();

				match state.render() {
					Ok(_) => {},
					Err(e) => log::error!("Unable to render: {:#?}", e),
				};
			},
			_ => {},
		}
	}
}

// ----------------------------- WGPU shit -----------------------------

struct WGPUState {
	surface: wgpu::Surface<'static>,
	device: wgpu::Device,
	queue: wgpu::Queue,
	config: wgpu::SurfaceConfiguration,

	window: Arc<Window>,
}

impl WGPUState {
	pub async fn new(window: Arc<Window>) -> Self {
		// BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
		let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
			backends: wgpu::Backends::PRIMARY,
			..wgpu::InstanceDescriptor::new_without_display_handle()
		});

		let surface = instance.create_surface(window.clone()).unwrap();

		// The adapter is a handle for our actual graphics card. You can use this
		// to get information about the graphics card, such as its name and what
		// backend the adapter uses
		let adapter = instance
			.request_adapter(&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::LowPower,
				compatible_surface: Some(&surface),
				force_fallback_adapter: false,
			})
			.await
			.unwrap();

		let (device, queue) = adapter
			.request_device(&wgpu::DeviceDescriptor {
				label: Some("Main Device"),
				..Default::default()
			})
			.await
			.unwrap();

		let surface_caps = surface.get_capabilities(&adapter);
		let size = window.inner_size();
		let config = wgpu::SurfaceConfiguration {
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
			format: surface_caps
				.formats
				.iter()
				.find(|f| {
					// Shader code in this tutorial assumes an sRGB surface texture. Using a different
					// one will result in all the colors coming out darker. If you want to support non
					// sRGB surfaces, you'll need to account for that when drawing to the frame.
					f.is_srgb()
				})
				.copied()
				.unwrap_or(surface_caps.formats[0]),
			width: size.width,
			height: size.height,
			present_mode: wgpu::PresentMode::Fifo,
			alpha_mode: surface_caps.alpha_modes[0],
			view_formats: vec![],
			desired_maximum_frame_latency: 2,
		};

		Self {
			surface,
			device,
			queue,
			config,
			window,
		}
	}

	pub fn resize(&mut self, width: u32, height: u32) {
		// wgpu crashes if the window size is less than 1x1
		if width > 0 && height > 0 {
			self.config.width = width;
			self.config.height = height;
			self.surface.configure(&self.device, &self.config);
		}
	}

	pub fn render(&mut self) -> anyhow::Result<()> {
		self.window.request_redraw();

		let output = match self.surface.get_current_texture() {
			wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
			wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
			wgpu::CurrentSurfaceTexture::Outdated => {
				self.surface.configure(&self.device, &self.config);
				return Ok(()); // Skip this frame
			},
			_ => panic!("Surface error!"),
		};

		let view = output
			.texture
			.create_view(&wgpu::TextureViewDescriptor::default());

		let mut encoder = self
			.device
			.create_command_encoder(&wgpu::CommandEncoderDescriptor {
				label: Some("Main Render Encoder"),
			});

		let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("Render Pass"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				depth_slice: None,
				view: &view,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Clear(wgpu::Color {
						r: 0.9,
						g: 0.2,
						b: 0.5,
						a: 1.0,
					}),
					store: wgpu::StoreOp::Store,
				},
			})],
			depth_stencil_attachment: None,
			occlusion_query_set: None,
			timestamp_writes: None,
			multiview_mask: None,
		});
		drop(render_pass);

		// submit will accept anything that implements IntoIter
		self.queue.submit(std::iter::once(encoder.finish()));
		output.present();

		Ok(())
	}
}

// ----------------------------- EGUI shit -----------------------------

struct EGUIState {
	ctx: Context,
	state: egui_winit::State,
	renderer: egui_wgpu::Renderer,
}
