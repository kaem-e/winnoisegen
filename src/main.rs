#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]
#![feature(float_minimum_maximum)]

use cpal::{
	self, Host,
	traits::{DeviceTrait, HostTrait, StreamTrait},
};
use rand;

fn main() -> Result<(), anyhow::Error> {
	let device = Host::default()
		.default_output_device()
		.ok_or_else(|| anyhow::Error::msg("Failed to get default output device"))?;

	let config = device
		.supported_output_configs()?
		.next()
		.ok_or_else(|| anyhow::Error::msg("No supported stream config"))?
		.with_max_sample_rate()
		.config();


	let stream = device.build_output_stream(
		&config,
		move |b: &mut [f32], _| {
			// Deinterleave mono
			for i in b.chunks_mut(config.channels as usize) {
				let s = rand::random::<f32>();
				i.fill(s);
			}

			// for i in a {
			// 	*i = rand::random::<f32>();
			// }
		},
		|_| {},
		None,
	)?;

	stream.play()?;

	std::thread::sleep(std::time::Duration::from_millis(2000));

	Ok(())
}
