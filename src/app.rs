use anyhow::anyhow;
use egui_winit::EventResponse;
use log::*;
use notify::{Event, EventKind};
use notify::{RecursiveMode, Watcher};
use pollster::FutureExt as _;
use std::{path::Path, sync::Arc};
use tray_icon::{MouseButton, MouseButtonState, TrayIconEvent};
use window_vibrancy::*;
use winit::{
	dpi::LogicalPosition,
	event::*,
	event_loop::{ActiveEventLoop, EventLoopProxy},
	keyboard::{KeyCode, PhysicalKey},
	platform::windows::{CornerPreference, WindowAttributesExtWindows as _},
	window::{WindowAttributes, WindowId},
};

use crate::subsystems::{
	audio::AudioSubsystem,
	renderer::{GUIEvent, RendererSubsystem},
	tray_icon::TrayIconSubsystem,
	watcher::WatcherSubsystem,
};

#[allow(unused)]
#[derive(Debug)]
pub enum UserEvent {
	TrayIconEvent(TrayIconEvent),
	ShaderFileChanged(Event),
	UIEvent(GUIEvent),
	WinitWindowEvent(WindowEvent, WindowId),
	WinitDeviceEvent(DeviceEvent, DeviceId),
}

pub struct App {
	audio: AudioSubsystem,
	tray_icon: TrayIconSubsystem,
	file_watcher: WatcherSubsystem,
	renderer: Option<RendererSubsystem>,
	proxy: EventLoopProxy<UserEvent>,
}

const SHADERS_DIRECTORY: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders");

const WINDOW_WIDTH: f64 = 180.0;
const WINDOW_HEIGHT: f64 = 250.0;
const XOFFSET: f64 = 10.0;
const YOFFSET: f64 = 60.0;

impl App {
	pub fn new_uninitialized(proxy: EventLoopProxy<UserEvent>) -> anyhow::Result<App> {
		Ok(App {
			audio: AudioSubsystem::new()?,
			tray_icon: TrayIconSubsystem::new(proxy.clone())?,
			file_watcher: WatcherSubsystem::new(proxy.clone())?,
			renderer: None,
			proxy,
		})
	}

	pub fn initialize(&mut self, event_loop: &ActiveEventLoop) -> anyhow::Result<()> {
		// Graphics have already been initialized
		if self.renderer.is_some() {
			return Ok(());
		}

		let monitor_size = {
			let monitor_handle = event_loop
				.primary_monitor()
				.ok_or(anyhow!("No Primary Monitory Found"))?;
			let monitor_size = monitor_handle.scale_factor();
			monitor_handle.size().to_logical::<f64>(monitor_size)
		};

		let window_attributes = WindowAttributes::default()
			.with_inner_size(winit::dpi::LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT))
			.with_corner_preference(CornerPreference::Round)
			.with_window_level(winit::window::WindowLevel::AlwaysOnTop)
			.with_no_redirection_bitmap(true)
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
		self.tray_icon.set_theme(window.theme().unwrap())?;

		apply_tabbed(&window, None).unwrap();
		self.renderer = Some(RendererSubsystem::new(window, self.proxy.clone()).block_on()?);

		Ok(())
	}

	pub fn handle_event(
		&mut self,
		event_loop: &ActiveEventLoop,
		event: UserEvent,
	) -> anyhow::Result<()> {
		// destructure everything for convenient use
		let App {
			renderer: Option::Some(renderer),
			audio,
			file_watcher: WatcherSubsystem(watcher),
			..
		} = self
		else {
			return Ok(());
		};
		let window = renderer.window.as_ref();

		// Pipe any WindowEvent we receive into egui's input handling first and foremost
		if let UserEvent::WinitWindowEvent(ref e, _) = event {
			let EventResponse { consumed, repaint } = renderer.state.on_window_event(window, e);
			if consumed {
				return Ok(());
			}
			if repaint {
				window.request_redraw();
			}
		}

		match event {
			// Toggle Playback on spacebar press within the app or on tray icon right click
			UserEvent::TrayIconEvent(TrayIconEvent::Click {
				button_state: MouseButtonState::Up,
				button: MouseButton::Right,
				..
			})
			| UserEvent::UIEvent(GUIEvent::TogglePlayback)
			| UserEvent::WinitWindowEvent(
				WindowEvent::KeyboardInput {
					event:
						KeyEvent {
							physical_key: PhysicalKey::Code(KeyCode::Space),
							state: ElementState::Pressed,
							..
						},
					..
				},
				..,
			) => audio.toggle_playback()?,

			// Show Window on Tray icon left click
			UserEvent::TrayIconEvent(TrayIconEvent::Click {
				button_state: MouseButtonState::Up,
				button: MouseButton::Left,
				..
			}) => {
				match window.is_visible() {
					Some(true) | None => {
						window.set_visible(false);
						watcher.unwatch(Path::new(SHADERS_DIRECTORY))?;
						info!("unwatch watcher for {}", SHADERS_DIRECTORY);
					},
					Some(false) => {
						window.set_visible(true);
						// let _ = window.request_inner_size(LogicalSize::new(0, 0));
						// let _ = window.request_inner_size(LogicalSize::new(WINDOW_WIDTH, WINDOW_HEIGHT));

						watcher.watch(Path::new(SHADERS_DIRECTORY), RecursiveMode::NonRecursive)?;
						info!("set up watcher for {}", SHADERS_DIRECTORY);

						window.focus_window();
						window.request_redraw();
					},
				};
			},

			// Call shader update logic within renderer
			UserEvent::ShaderFileChanged(Event {
				kind: EventKind::Modify(_),
				..
			}) => renderer.reload_shader()?,

			// Resize graphics when window size is changed
			//
			// this isnt strictily nesessary for *this* app in particular since,
			// yknow, its just a tray icon, but this is smth you need to do for
			// wgpu normally
			#[rustfmt::skip]
			UserEvent::WinitWindowEvent(WindowEvent::Resized(s), _) => renderer.resize(s.width, s.height),

			// Close winit's event loop when we receive request to do so
			UserEvent::WinitWindowEvent(WindowEvent::CloseRequested, _)
			| UserEvent::UIEvent(GUIEvent::CloseRequested)
			| UserEvent::WinitWindowEvent(
				WindowEvent::KeyboardInput {
					event:
						KeyEvent {
							physical_key: PhysicalKey::Code(KeyCode::Escape),
							state: ElementState::Pressed,
							..
						},
					..
				},
				_,
			) => event_loop.exit(),

			UserEvent::WinitWindowEvent(WindowEvent::RedrawRequested, _)
			| UserEvent::WinitWindowEvent(WindowEvent::CursorMoved { .. }, _) => self.redraw(),

			_ => {},
		}

		Ok(())
	}

	pub fn redraw(&mut self) {
		let renderer = self.renderer.as_mut().unwrap();

		let s = renderer.window.inner_size();
		if s.width > 0 && s.height > 0 {
			match renderer.redraw() {
				Ok(_) => {},
				Err(e) => error!("Renderer Error: {:#?}", e),
			};
		}

		renderer.window.request_redraw();
	}
}
