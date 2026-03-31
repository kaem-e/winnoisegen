use cpal::{
	self, Host, Stream,
	traits::{DeviceTrait, HostTrait, StreamTrait},
};


enum PlaybackState {
	Playing,
	Paused,
}

pub struct AudioSubsystem {
	stream: Stream,
	playback_state: PlaybackState,
}

impl AudioSubsystem {
	pub fn new() -> anyhow::Result<Self> {
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

		Ok(Self {
			stream,
			playback_state: PlaybackState::Paused, // stream is paused by default
		})
	}

	// Toggles playback of cpal stream
	pub fn toggle_playback(&mut self) -> anyhow::Result<()> {
		let Self {
			stream,
			playback_state,
		} = self;

		match playback_state {
			PlaybackState::Playing => {
				stream.pause()?;
				*playback_state = PlaybackState::Paused;
			},
			PlaybackState::Paused => {
				stream.play()?;
				*playback_state = PlaybackState::Playing;
			},
		};

		Ok(())
	}
}
