#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![allow(unused)]
#![feature(float_minimum_maximum, portable_simd)]

use anyhow;
use cpal::{
	self, Host, Stream,
	traits::{DeviceTrait, HostTrait, StreamTrait},
};
// use egui;
// use egui_wgpu::{self, Renderer, RendererOptions};
// use egui_winit::{self, State};
use log::{
	Level, LevelFilter, debug as log_debug, error as log_error, info as log_info, log,
	trace as log_trace,
};
use env_logger;
use pollster::FutureExt as _;
use std::{path::Path, sync::Arc};
use tray_icon::{
	self, Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconAttributes, TrayIconEvent,
	menu::MenuEvent,
};
use wgpu::*;
use window_vibrancy::{apply_mica, apply_tabbed};
use winit::{
	application::ApplicationHandler,
	dpi::{LogicalPosition, LogicalSize},
	event::{ElementState, KeyEvent, WindowEvent},
	event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
	keyboard::{KeyCode, PhysicalKey},
	platform::windows::{CornerPreference, WindowAttributesExtWindows},
	window::{Theme, Window, WindowAttributes, WindowId, WindowLevel},
};

fn main() -> Result<(), anyhow::Error> {
	env_logger::builder().filter_level(LevelFilter::Info).init();

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
	window: Option<Arc<Window>>,
	wgpu_state: Option<WGPUState>,
	// egui_state: Option<EGUIState>,
}

struct WGPUState {
	surface: Surface<'static>,
	surface_config: SurfaceConfiguration,
	device: Device,
	queue: Queue,

	render_pipeline: RenderPipeline,
}

// struct EGUIState {
// 	state: State,
// 	renderer: Renderer,
// }

impl App {
	pub fn new(cpal_stream: Stream, tray_icon: TrayIcon, tray_icons: (Icon, Icon)) -> Self {
		Self {
			cpal_state: (cpal_stream, false),

			tray_icon,
			tray_icons,

			wgpu_state: None,
			// egui_state: None,
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
		if self.window.is_none() || self.wgpu_state.is_none() {
			let monitor_size = {
				let monitor_handle = event_loop.primary_monitor().unwrap();
				let monitor_size = monitor_handle.scale_factor();
				monitor_handle.size().to_logical::<f64>(monitor_size)
			};

			let window_attributes = WindowAttributes::default()
				.with_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
				.with_corner_preference(CornerPreference::Round)
				.with_window_level(WindowLevel::AlwaysOnTop)
				.with_undecorated_shadow(true)
				.with_drag_and_drop(false)
				.with_skip_taskbar(true)
				.with_decorations(false)
				.with_transparent(true)
				.with_title("ambience")
				.with_resizable(false)
				.with_visible(false)
				.with_active(false)
				.with_position(LogicalPosition::new(
					monitor_size.width - (WINDOW_WIDTH + XOFFSET),
					monitor_size.height - (WINDOW_HEIGHT + YOFFSET),
				));

			let window = Arc::new(event_loop.create_window(window_attributes).unwrap());
			apply_tabbed(&window, None).unwrap();

			match window.theme() {
				Some(Theme::Light) => self.tray_icon.set_icon(Some(self.tray_icons.0.clone())),
				_ => self.tray_icon.set_icon(Some(self.tray_icons.1.clone())),
			};

			self.wgpu_state = Some(WGPUState::new(window.clone()).block_on());
			self.window = Some(window)
		} else {
			log_info!("app resumed and state already present");
		}
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

						// winit initializes a window with the stupid windows painter.
						// in order to bypass this we either can do some unsafe
						// shenanegans, or just resize the window to 0 and then to our
						// size to clear it
						window.request_inner_size(LogicalSize::new(0, 0));
						window.request_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));

						window.focus_window();
						window.request_redraw();
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
		let wgpu_state = self.wgpu_state.as_mut().unwrap();
		let window = self.window.as_ref().unwrap();

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

			WindowEvent::Resized(s) => wgpu_state.resize(s.width, s.height),

			WindowEvent::RedrawRequested => {
				if {
					let w = window.inner_size();
					w.width > 0 && w.height > 0 && window.is_visible().unwrap()
				} {
					match wgpu_state.render() {
						Ok(_) => {},
						Err(e) => log_error!("Unable to render: {:#?}", e),
					};
				}
				// self.window.as_ref().unwrap().request_redraw();
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

impl WGPUState {
	pub async fn new(window: Arc<Window>) -> Self {
		let instance = Instance::new(InstanceDescriptor {
			backends: Backends::DX12, // vulkan wont let us expose different draw call formats so
			backend_options: BackendOptions {
				dx12: Dx12BackendOptions {
					presentation_system: Dx12SwapchainKind::DxgiFromVisual,
					..Default::default()
				},
				..Default::default()
			},
			..InstanceDescriptor::new_without_display_handle()
		});

		let window_size = window.inner_size();
		let surface = instance.create_surface(window).unwrap();
		let adapter = instance
			.request_adapter(&RequestAdapterOptions {
				power_preference: PowerPreference::HighPerformance,
				compatible_surface: Some(&surface),
				force_fallback_adapter: false,
			})
			.await
			.unwrap();

		let (device, queue) = adapter
			.request_device(&DeviceDescriptor {
				label: Some("Main Device"),
				..Default::default()
			})
			.await
			.unwrap();

		let surface_caps = surface.get_capabilities(&adapter);
		// [src\main.rs:338:22] surface.get_capabilities(&adapter) = SurfaceCapabilities {
		//     formats: [
		//         Bgra8UnormSrgb,
		//         Rgba8UnormSrgb,
		//         Bgra8Unorm,
		//         Rgba8Unorm,
		//         Rgb10a2Unorm,
		//         Rgba16Float,
		//     ],
		//     present_modes: [
		//         Mailbox,
		//         Fifo,
		//         Immediate,
		//     ],
		//     alpha_modes: [
		//         Auto,
		//         Inherit,
		//         Opaque,
		//         PostMultiplied,
		//         PreMultiplied,
		//     ],
		//     usages: TextureUsages(
		//         COPY_SRC | COPY_DST | RENDER_ATTACHMENT,
		//     ),
		// }

		let surface_config = SurfaceConfiguration {
			usage: TextureUsages::RENDER_ATTACHMENT,
			// format: surface_caps.formats[0],
			format: TextureFormat::Rgba16Float,
			width: window_size.width,
			height: window_size.height,
			present_mode: PresentMode::Fifo,
			// alpha_mode: surface_caps.alpha_modes[0],
			alpha_mode: CompositeAlphaMode::PreMultiplied,
			view_formats: vec![],
			desired_maximum_frame_latency: 2,
		};

		// // if you get rid of the resized method this needs to be done atleast
		// // once to make the surface thats configured like,,,,,,,,,, actually
		// // use the config we provide it
		// surface.configure(&device, &config);

		let shader_module = device.create_shader_module(include_wgsl!("shader.wgsl"));
		let render_pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
			label: Some("super penis balls triangle pipeline"),
			layout: None,
			vertex: VertexState {
				module: &shader_module,
				entry_point: None,
				compilation_options: PipelineCompilationOptions::default(),
				buffers: &[],
			},
			fragment: Some(FragmentState {
				module: &shader_module,
				entry_point: None,
				compilation_options: PipelineCompilationOptions::default(),
				targets: &[Some(ColorTargetState {
					format: surface_config.format,
					blend: Some(BlendState::ALPHA_BLENDING),
					write_mask: ColorWrites::ALL,
				})],
			}),

			multisample: MultisampleState::default(),
			primitive: PrimitiveState::default(),
			depth_stencil: None,
			multiview_mask: None,
			cache: None,
		});

		Self {
			surface,
			surface_config,
			device,
			queue,

			render_pipeline,
		}
	}

	pub fn render(&mut self) -> anyhow::Result<()> {
		let output = match self.surface.get_current_texture() {
			CurrentSurfaceTexture::Success(s) | CurrentSurfaceTexture::Suboptimal(s) => s,
			CurrentSurfaceTexture::Outdated => {
				self.surface.configure(&self.device, &self.surface_config);
				return Ok(()); // Skip this frame
			},
			_ => panic!("Surface error!"),
		};

		let view = output
			.texture
			.create_view(&TextureViewDescriptor::default());

		let mut encoder = self
			.device
			.create_command_encoder(&CommandEncoderDescriptor {
				label: Some("Main Render Encoder"),
			});

		let mut render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
			label: Some("Render Pass"),
			color_attachments: &[Some(RenderPassColorAttachment {
				depth_slice: None,
				view: &view,
				resolve_target: None,
				ops: Operations {
					load: LoadOp::Clear(
						// Color {
						// 	r: 0.6,
						// 	g: 0.02,
						// 	b: 0.35,
						// 	a: 0.2,
						// },
						Color::TRANSPARENT,
					),
					store: StoreOp::Store,
				},
			})],
			depth_stencil_attachment: None,
			occlusion_query_set: None,
			timestamp_writes: None,
			multiview_mask: None,
		});

		render_pass.set_pipeline(&self.render_pipeline);
		render_pass.draw(0..6, 0..1);

		drop(render_pass);

		// submit will accept anything that implements IntoIter
		self.queue.submit(std::iter::once(encoder.finish()));
		output.present();

		Ok(())
	}

	pub fn resize(&mut self, width: u32, height: u32) {
		// wgpu crashes if the window size is less than 1x1
		if width > 0 && height > 0 {
			self.surface_config.width = width;
			self.surface_config.height = height;
			self.surface.configure(&self.device, &self.surface_config);
		}
	}
}

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
