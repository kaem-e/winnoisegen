#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![allow(unused)]
#![feature(float_minimum_maximum, portable_simd)]

use anyhow;
use cpal::{
	self, Host, Stream,
	traits::{DeviceTrait, HostTrait, StreamTrait},
};
use egui;
use egui_wgpu::{self, Renderer, RendererOptions};
use egui_winit::{self, State};
use env_logger;
use pollster::FutureExt as _;
use std::{path::Path, sync::Arc};
use tray_icon::{
	self, Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconAttributes, TrayIconEvent,
	menu::MenuEvent,
};
use wgpu::{self, *};
use window_vibrancy::apply_mica;
use winit::{
	application::ApplicationHandler,
	dpi::{LogicalPosition, LogicalSize},
	event::{ElementState, KeyEvent, WindowEvent},
	event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
	keyboard::{self, KeyCode, PhysicalKey},
	platform::windows::{CornerPreference, WindowAttributesExtWindows},
	window::{Theme, Window, WindowAttributes, WindowId, WindowLevel},
};

fn main() -> Result<(), anyhow::Error> {
	env_logger::init();

	// set up cpal stream
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

	// tray icon
	let icon_dark = {
		let image = image::open(Path::new(concat!(
			env!("CARGO_MANIFEST_DIR"),
			"/assets/Icon.png"
		)))?
		.into_rgba8();

		let (width, height) = image.dimensions();
		let rgba = image.into_raw();
		Icon::from_rgba(rgba, width, height).expect("Failed to open icon")
	};
	let icon_light = {
		let image = image::open(Path::new(concat!(
			env!("CARGO_MANIFEST_DIR"),
			"/assets/Icon-light.png"
		)))?
		.into_rgba8();

		let (width, height) = image.dimensions();
		let rgba = image.into_raw();
		Icon::from_rgba(rgba, width, height).expect("Failed to open icon")
	};

	let tray_icon = TrayIcon::new(TrayIconAttributes {
		tooltip: Some(String::from("penis balls tooltip")),
		title: Some(String::from("ambience")),
		..Default::default()
	})?;

	let event_loop = EventLoop::<UserEvent>::with_user_event().build()?;
	event_loop.set_control_flow(ControlFlow::Wait);

	// give event loop proxy to tray-icon and menu items within tray-icon
	let p = event_loop.create_proxy();
	tray_icon::TrayIconEvent::set_event_handler(Some(move |event| {
		p.send_event(UserEvent::TrayIconEvent(event));
	}));
	let p = event_loop.create_proxy();
	tray_icon::menu::MenuEvent::set_event_handler(Some(move |event| {
		p.send_event(UserEvent::MenuEvent(event));
	}));

	let mut app = App::new(stream, tray_icon, (icon_dark, icon_light));
	event_loop.run_app(&mut app)?;

	Ok(())
}

#[derive(Debug)]
enum UserEvent {
	TrayIconEvent(TrayIconEvent),
	MenuEvent(MenuEvent),
}

struct App {
	cpal_state: (Stream, bool),
	tray_icon: TrayIcon,
	tray_icons: (Icon, Icon),
	window: Option<Window>,
	wgpu_state: Option<WGPUState>,
	egui_state: Option<EGUIState>,
}

struct WGPUState {
	surface: Surface<'static>,
	device: Device,
	queue: Queue,
	config: SurfaceConfiguration,

	window: Arc<Window>,
	texture_format: TextureFormat,
}

struct EGUIState {
	state: State,
	renderer: Renderer,
}

impl App {
	pub fn new(cpal_stream: Stream, tray_icon: TrayIcon, tray_icons: (Icon, Icon)) -> Self {
		Self {
			cpal_state: (cpal_stream, false),

			tray_icon,
			tray_icons,

			wgpu_state: None,
			egui_state: None,

			window: None,
		}
	}
}

const WINDOW_WIDTH: f64 = 180.0;
const WINDOW_HEIGHT: f64 = 250.0;
const XOFFSET: f64 = 10.0;
const YOFFSET: f64 = 60.0;

impl ApplicationHandler<UserEvent> for App {
	fn resumed(&mut self, event_loop: &ActiveEventLoop) {
		let monitor_size = {
			let monitor_handle = event_loop.primary_monitor().unwrap();
			let monitor_size = monitor_handle.scale_factor();
			monitor_handle.size().to_logical::<f64>(monitor_size)
		};

		let window_attributes = WindowAttributes::default()
			.with_corner_preference(CornerPreference::Round)
			.with_window_level(WindowLevel::AlwaysOnTop)
			.with_undecorated_shadow(true)
			.with_drag_and_drop(false)
			.with_skip_taskbar(true)
			.with_decorations(false)
			.with_transparent(true)
			.with_title("ambience")
			.with_transparent(true)
			.with_resizable(false)
			.with_visible(false)
			.with_active(false)
			.with_position(LogicalPosition::new(
				monitor_size.width - (WINDOW_WIDTH + XOFFSET),
				monitor_size.height - (WINDOW_HEIGHT + YOFFSET),
			));

		let window = event_loop.create_window(window_attributes).unwrap();
		//
		match window.theme() {
			Some(Theme::Light) => self.tray_icon.set_icon(Some(self.tray_icons.0.clone())),
			_ => self.tray_icon.set_icon(Some(self.tray_icons.1.clone())),
		};

		// self.wgpu_state = Some(WGPUState::new(Arc::new(window)).block_on());
		self.wgpu_state = None;
		self.window = Some(window)
	}

	fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
		let window = match self.window.as_ref() {
			Some(a) => a,
			None => return,
		};

		match event {
			#[rustfmt::skip]
			UserEvent::TrayIconEvent(TrayIconEvent::Click {
				button_state: MouseButtonState::Up,
				button: MouseButton::Left, ..
			}) => {
				match window.is_visible() {
					Some(true) | None => window.set_visible(false),
					Some(false) => {
						window.set_visible(true); // window needs to be visible before we clear it
						window.request_inner_size(LogicalSize::new(0.0, 0.0));
						apply_mica(&window, None).unwrap();
						window.request_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));
					},
				};
			},

			// toggle playback on right mouse
			#[rustfmt::skip]
			UserEvent::TrayIconEvent(TrayIconEvent::Click {
				button_state: MouseButtonState::Up,
				button: MouseButton::Right, ..
			}) => toggle_playback(&mut self.cpal_state),

			_e => {},
		}
	}

	fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
		match event {
			// close window with close event or by pressing escape
			#[rustfmt::skip]
			WindowEvent::KeyboardInput {
				event: KeyEvent {
					physical_key: PhysicalKey::Code(KeyCode::Escape),
					state: ElementState::Pressed, ..
				}, ..
			} => {
				self.window.as_ref().unwrap().set_visible(false);
			},

			// close app when we actually close the window with alt+f4
			WindowEvent::CloseRequested => event_loop.exit(),

			// spacebar to toggle play/pause
			#[rustfmt::skip]
			WindowEvent::KeyboardInput {
				event: KeyEvent {
					physical_key: PhysicalKey::Code(KeyCode::Space),
					state: ElementState::Pressed, ..
				}, ..
			} => toggle_playback(&mut self.cpal_state),

			// change tray icon when we receive a system theme switch
			WindowEvent::ThemeChanged(a) => match a {
				Theme::Light => self
					.tray_icon
					.set_icon(Some(self.tray_icons.0.clone())) // use dark icon in light mode
					.unwrap(),
				Theme::Dark => self
					.tray_icon
					.set_icon(Some(self.tray_icons.1.clone())) // use light icon in dark mode
					.unwrap(),
			},

			// WindowEvent::Resized(s) => state.resize(s.width, s.height),
			WindowEvent::RedrawRequested => {
				self.window.as_ref().unwrap().request_redraw();
				// 	match state.render() {
				// 		Ok(_) => {},
				// 		Err(e) => log::error!("Unable to render: {:#?}", e),
				// 	};
			},
			_ => {},
		}
	}
}

fn toggle_playback(cpal_state: &mut (Stream, bool)) {
	match dbg!(cpal_state.1) {
		true => cpal_state.0.pause().unwrap(),
		false => cpal_state.0.play().unwrap(),
	};
	cpal_state.1 = !cpal_state.1;
}

// impl WGPUState {
// 	pub async fn new(window: Arc<Window>) -> Self {
// 		// BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU
// 		let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
// 			backends: wgpu::Backends::PRIMARY,
// 			..wgpu::InstanceDescriptor::new_without_display_handle()
// 		});

// 		let surface = instance.create_surface(window.clone()).unwrap();

// 		// The adapter is a handle for our actual graphics card. You can use this
// 		// to get information about the graphics card, such as its name and what
// 		// backend the adapter uses
// 		let adapter = instance
// 			.request_adapter(&wgpu::RequestAdapterOptions {
// 				power_preference: wgpu::PowerPreference::LowPower,
// 				compatible_surface: Some(&surface),
// 				force_fallback_adapter: false,
// 			})
// 			.await
// 			.unwrap();

// 		let (device, queue) = adapter
// 			.request_device(&wgpu::DeviceDescriptor {
// 				label: Some("Main Device"),
// 				..Default::default()
// 			})
// 			.await
// 			.unwrap();

// 		let surface_caps = surface.get_capabilities(&adapter);
// 		let size = window.inner_size();
// 		let config = wgpu::SurfaceConfiguration {
// 			usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
// 			format: surface_caps
// 				.formats
// 				.iter()
// 				.find(|f| {
// 					// Shader code in this tutorial assumes an sRGB surface texture. Using a different
// 					// one will result in all the colors coming out darker. If you want to support non
// 					// sRGB surfaces, you'll need to account for that when drawing to the frame.
// 					f.is_srgb()
// 				})
// 				.copied()
// 				.unwrap_or(surface_caps.formats[0]),
// 			width: size.width,
// 			height: size.height,
// 			present_mode: wgpu::PresentMode::Fifo,
// 			alpha_mode: surface_caps.alpha_modes[0],
// 			view_formats: vec![],
// 			desired_maximum_frame_latency: 2,
// 		};

// 		Self {
// 			surface,
// 			device,
// 			queue,
// 			config,
// 			window,
// 			texture_format: todo!(),
// 		}
// 	}

// 	pub fn resize(&mut self, width: u32, height: u32) {
// 		// wgpu crashes if the window size is less than 1x1
// 		if width > 0 && height > 0 {
// 			self.config.width = width;
// 			self.config.height = height;
// 			self.surface.configure(&self.device, &self.config);
// 		}
// 	}

// 	pub fn render(&mut self) -> anyhow::Result<()> {
// 		self.window.request_redraw();

// 		let output = match self.surface.get_current_texture() {
// 			wgpu::CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
// 			wgpu::CurrentSurfaceTexture::Suboptimal(surface_texture) => surface_texture,
// 			wgpu::CurrentSurfaceTexture::Outdated => {
// 				self.surface.configure(&self.device, &self.config);
// 				return Ok(()); // Skip this frame
// 			},
// 			_ => panic!("Surface error!"),
// 		};

// 		let view = output
// 			.texture
// 			.create_view(&wgpu::TextureViewDescriptor::default());

// 		let mut encoder = self
// 			.device
// 			.create_command_encoder(&wgpu::CommandEncoderDescriptor {
// 				label: Some("Main Render Encoder"),
// 			});

// 		let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
// 			label: Some("Render Pass"),
// 			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
// 				depth_slice: None,
// 				view: &view,
// 				resolve_target: None,
// 				ops: wgpu::Operations {
// 					load: wgpu::LoadOp::Clear(wgpu::Color {
// 						r: 0.9,
// 						g: 0.2,
// 						b: 0.5,
// 						a: 1.0,
// 					}),
// 					store: wgpu::StoreOp::Store,
// 				},
// 			})],
// 			depth_stencil_attachment: None,
// 			occlusion_query_set: None,
// 			timestamp_writes: None,
// 			multiview_mask: None,
// 		});
// 		drop(render_pass);

// 		// submit will accept anything that implements IntoIter
// 		self.queue.submit(std::iter::once(encoder.finish()));
// 		output.present();

// 		Ok(())
// 	}
// }

// impl EGUIState {
// 	fn new(window: &Window, wgpu_state: &WGPUState) -> Self {
// 		let state = State::new(
// 			egui::Context::default(),
// 			egui::ViewportId::ROOT,
// 			window,
// 			Some(window.scale_factor() as f32),
// 			None,
// 			None,
// 		);
// 		let renderer = Renderer::new(
// 			&wgpu_state.device,
// 			wgpu_state.texture_format,
// 			RendererOptions::default(),
// 		);

// 		Self { state, renderer }
// 	}
// }
