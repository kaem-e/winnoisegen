use anyhow::anyhow;
use egui::{Context, Ui, ViewportId};
use egui_wgpu::{Renderer, RendererOptions, ScreenDescriptor};
use egui_winit::State;
use log::*;
use std::{borrow::Cow, fs, sync::Arc};
use wgpu::*;
use winit::{event_loop::EventLoopProxy, window::Window};

use crate::{app::UserEvent, utils::log_err as _};

const SHADER_FILEPATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/shader.wgsl");

pub struct RendererSubsystem {
	pub window: Arc<Window>,
	proxy: EventLoopProxy<UserEvent>,

	surface: Surface<'static>,
	surface_config: SurfaceConfiguration,
	render_pipeline: RenderPipeline,
	device: Device,
	queue: Queue,

	pub state: State,
	renderer: Renderer,
	ctx: Context,
}

impl RendererSubsystem {
	pub async fn new(window: Arc<Window>, proxy: EventLoopProxy<UserEvent>) -> anyhow::Result<Self> {
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
		let surface = instance.create_surface(window.clone()).unwrap();
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
		let surface_caps = surface.get_capabilities(&adapter);
		let format = *surface_caps
			.formats
			.iter()
			.find(|&&a| a == TextureFormat::Bgra8Unorm)
			.unwrap_or(&surface_caps.formats[0]);

		let surface_config = SurfaceConfiguration {
			usage: TextureUsages::RENDER_ATTACHMENT,
			format,
			width: window_size.width,
			height: window_size.height,
			present_mode: PresentMode::AutoVsync,
			// alpha_mode: surface_caps.alpha_modes[0],
			alpha_mode: CompositeAlphaMode::PreMultiplied,
			view_formats: vec![],
			desired_maximum_frame_latency: 2,
		};

		// // if you get rid of the resized method this needs to be done atleast
		// // once to make the surface thats configured like,,,,,,,,,, actually
		// // use the config we provide it
		surface.configure(&device, &surface_config);

		let render_pipeline = Self::create_render_pipeline(&device, surface_config.format)?;

		let ctx = Context::default();
		let state = State::new(
			ctx.clone(),
			ViewportId::ROOT,
			window.as_ref(),
			Some(window.scale_factor() as f32),
			None,
			None,
		);
		let renderer = Renderer::new(&device, format, RendererOptions::default());

		Ok(Self {
			window,
			proxy,
			surface,
			surface_config,
			device,
			queue,
			render_pipeline,

			state,
			renderer,
			ctx,
		})
	}

	fn create_render_pipeline(
		device: &Device,
		format: TextureFormat,
	) -> anyhow::Result<RenderPipeline> {
		let shader_module = device.create_shader_module(ShaderModuleDescriptor {
			label: Some("ultra glossy balls"),
			source: ShaderSource::Wgsl(Cow::Owned(fs::read_to_string(SHADER_FILEPATH)?)),
		});
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
					format,
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

		Ok(render_pipeline)
	}

	// single frame
	pub fn redraw(&mut self) -> anyhow::Result<()> {
		let window = self.window.as_ref();

		// a single surface_texutre needs to be consistent and be the one used for all the draw calls for one frame
		let surface_texture = match self.surface.get_current_texture() {
			CurrentSurfaceTexture::Success(s) | CurrentSurfaceTexture::Suboptimal(s) => Ok(s),
			CurrentSurfaceTexture::Outdated => {
				self.surface.configure(&self.device, &self.surface_config);
				return Ok(()); // Skip this frame
			},
			x => Err(anyhow!("balls {:?}", x)),
		}?;
		let view = surface_texture
			.texture
			.create_view(&TextureViewDescriptor::default());

		let mut encoder = self
			.device
			.create_command_encoder(&CommandEncoderDescriptor {
				label: Some("Main Render Pass Encoder"),
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

		// egui
		let raw_input = self.state.take_egui_input(window);

		let full_output = self.ctx.run_ui(raw_input, |ui| {
			Self::egui_ui(ui, &self.proxy);
		});

		self
			.state
			.handle_platform_output(window, full_output.platform_output);
		let egui_ui_primitives = self
			.ctx
			.tessellate(full_output.shapes, full_output.pixels_per_point);

		// Update GPU buffers and textures for Egui
		for (texture_id, image_delta) in &full_output.textures_delta.set {
			self
				.renderer
				.update_texture(&self.device, &self.queue, *texture_id, image_delta);
		}
		let screen_descriptor = ScreenDescriptor {
			size_in_pixels: [self.surface_config.width, self.surface_config.height],
			pixels_per_point: window.scale_factor() as f32,
		};

		self.renderer.update_buffers(
			&self.device,
			&self.queue,
			&mut encoder,
			&egui_ui_primitives,
			&screen_descriptor,
		);

		let egui_render_pass = encoder.begin_render_pass(&RenderPassDescriptor {
			label: Some("EGUI Render Pass"),
			color_attachments: &[Some(RenderPassColorAttachment {
				view: &view,
				resolve_target: None,
				ops: Operations {
					load: LoadOp::Load, // reuse prev pass's output
					store: StoreOp::Store,
				},
				depth_slice: None,
			})],
			depth_stencil_attachment: None,
			multiview_mask: None,
			timestamp_writes: None,
			occlusion_query_set: None,
		});

		self.renderer.render(
			&mut egui_render_pass.forget_lifetime(),
			&egui_ui_primitives,
			&screen_descriptor,
		);
		// drop(egui_render_pass); // not needed. egui_wgpu's renderer will drop this for us

		self.queue.submit(std::iter::once(encoder.finish()));
		surface_texture.present();

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

	pub fn reload_shader(&mut self) -> anyhow::Result<()> {
		self.render_pipeline =
			Self::create_render_pipeline(&self.device, self.surface_config.format)?;

		info!("Shader Reloaded");
		Ok(())
	}

	fn egui_ui(ui: &mut Ui, proxy: &EventLoopProxy<UserEvent>) {
		use egui::*;

		Window::new("Manual Setup").show(ui, |ui| {
			ui.label("Successfully wired winit + wgpu + egui manually!");
			ui.horizontal(|ui| {
				if ui.button("Exit").clicked() {
					proxy
						.send_event(UserEvent::UIEvent(GUIEvent::CloseRequested))
						.log_err();
				}
				if ui.button("Play/Pause").clicked() {
					proxy
						.send_event(UserEvent::UIEvent(GUIEvent::TogglePlayback))
						.log_err();
				}
			})
		});
	}
}

#[derive(Debug)]
pub enum GUIEvent {
	CloseRequested,
	TogglePlayback,
}
