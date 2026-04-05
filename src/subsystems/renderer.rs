use anyhow::anyhow;

use log::*;
use pollster::FutureExt;
use std::{borrow::Cow, fs, sync::Arc};
use wgpu::*;
use winit::window::Window;

pub struct RendererSubsystem {
	instance: Instance,
	handles: Option<WGPUHandles>,
}

impl RendererSubsystem {
	/// Initialize only the bare minimum from the renderer
	pub async fn new() -> anyhow::Result<Self> {
		let instance = Instance::new(InstanceDescriptor {
			backends: Backends::DX12, // vulkan wont let us expose different draw call formats so
			backend_options: BackendOptions {
				dx12: Dx12BackendOptions {
					presentation_system: Dx12SwapchainKind::DxgiFromVisual,
					// shader_compiler: Dx12Compiler::Fxc,
					..Default::default()
				},
				..Default::default()
			},
			..InstanceDescriptor::new_without_display_handle()
		});

		Ok(Self {
			instance,
			handles: None,
		})
	}

	/// Call this when window is unhidden
	/// We do this as the renderer while initialized takes up a lot of memory.
	/// And it isnt needed all the time, so when the window is made visible, just call this
	///
	/// This will err if renderer is already initialized,
	/// it only works after [`Self::uninitialize_wgpu()`] was called
	pub fn initialize_wgpu(&mut self, window: Arc<Window>) -> anyhow::Result<()> {
		self.handles = Some(WGPUHandles::new(&self.instance, window.clone()).block_on()?);
		Ok(())
	}

	/// Call this when window is hidden
	/// Other half of [`Self::initialize_wgpu`]
	pub fn uninitialize_wgpu(&mut self) {
		drop(self.handles.take())
	}

	/// Draw a single frame,
	/// This will err if renderer wasnt initialized with [`Self::initialize_wgpu`]
	pub fn redraw(&mut self) -> anyhow::Result<()> {
		self
			.handles
			.as_mut()
			.ok_or(anyhow!(
				"WGPU State not initialized, `Self::initialize_wgpu()` needs to be called"
			))?
			.redraw()
	}

	/// Call when window size has changed
	pub fn resize(&mut self, width: u32, height: u32) -> anyhow::Result<()> {
		self
			.handles
			.as_mut()
			.ok_or(anyhow!(
				"WGPU State not initialized, `Self::initialize_wgpu()` needs to be called"
			))?
			.resize(width, height);

		Ok(())
	}

	pub fn reload_shader(&mut self) -> anyhow::Result<()> {
		self
			.handles
			.as_mut()
			.ok_or(anyhow!(
				"WGPU State not initialized, `Self::initialize_wgpu()` needs to be called"
			))?
			.reload_shader()
	}
}

// -------------------------------------------------------------------------------------

const SHADER_FILEPATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/src/shaders/shader.wgsl");

/// were just going to store the instance and device in the renderersubsystem,
/// this will be initialized/deinitialized when the window is shown/hidden
struct WGPUHandles {
	surface: Surface<'static>,
	surface_config: SurfaceConfiguration,
	render_pipeline: RenderPipeline,
	device: Device,
	queue: Queue,
}

impl WGPUHandles {
	async fn new(instance: &Instance, window: Arc<Window>) -> anyhow::Result<Self> {
		let window_size = window.inner_size();
		let surface = instance.create_surface(window).unwrap();

		let adapter = instance
			.request_adapter(&RequestAdapterOptions {
				power_preference: PowerPreference::LowPower,
				compatible_surface: None,
				force_fallback_adapter: false,
			})
			.await?;

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

		let (device, queue) = adapter
			.request_device(&DeviceDescriptor {
				label: Some("Main Device"),
				memory_hints: MemoryHints::MemoryUsage,
				..Default::default()
			})
			.await?;

		// if you get rid of the resized method this needs to be done atleast
		// once to make the surface thats configured like,,,,,,,,,, actually
		// use the config we provide it
		surface.configure(&device, &surface_config);

		let render_pipeline = Self::create_render_pipeline(&device, surface_config.format)?;

		drop(adapter);


		Ok(Self {
			surface,
			surface_config,
			device,
			queue,
			render_pipeline,
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
}
