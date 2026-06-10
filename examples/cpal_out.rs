use anyhow::Context;
use cpal::{
	Host, SampleFormat,
	traits::{DeviceTrait, HostTrait, StreamTrait},
};

fn main() -> anyhow::Result<()> {
	let device = Host::default()
		.default_output_device()
		.context("Failed to get default output device")?;

	let config = &device
		.supported_output_configs()
		.context("Failed to get default output configs for device")?
		.find(|c| {
			c.sample_format() == SampleFormat::F32
				&& c.channels() == 2
				&& (c.min_sample_rate()..=c.max_sample_rate()).contains(&48000u32)
		})
		.context("Failed to find suitable config for device output configs")?
		.try_with_sample_rate(48000)
		.context("Failed to get config with sample rate")?
		.config();

	let mut sine_osc_phase: f32 = 0.0;
	let mut sine_osc = move || -> f32 {
		use std::f32::consts::PI;
		const PHASE_INC: f32 = (2.0 * PI / 48000.0) * 440.0;
		let result = sine_osc_phase.sin();
		sine_osc_phase = (sine_osc_phase + PHASE_INC) % (2.0 * PI);
		result
	};
	let stream_handle = device
		.build_output_stream(
			config,
			move |frame: &mut [f32], _| {
				for s in frame.chunks_exact_mut(2) {
					let v = sine_osc();
					*unsafe { s.get_unchecked_mut(0) } = v;
					*unsafe { s.get_unchecked_mut(1) } = v;
				}
			},
			|_| (),
			None,
		)
		.context("Failed to build output stream")?;

	stream_handle.play()?;
	std::thread::sleep(std::time::Duration::from_secs(1));
	stream_handle.pause()?;
	println!("pebis balls");
	Ok(())
}
