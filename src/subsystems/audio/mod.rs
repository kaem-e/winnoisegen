use cpal::{
	self, Host, Stream,
	traits::{DeviceTrait, HostTrait, StreamTrait},
};
use log::{error, info};
use opus::{Channels, Decoder};
use ringbuf::{
	StaticRb,
	traits::{Consumer, Observer, Producer, Split},
};
use std::thread;

mod opus_atlas;
use crate::subsystems::audio::opus_atlas::OpusAtlas;

enum PlaybackState {
	Playing,
	Paused,
}

pub struct AudioSubsystem {
	stream: Stream,
	playback_state: PlaybackState,
}

// Why 11,520?
// The calculation for a "worst-case" single packet at 48kHz stereo is:
// - Max duration: 120ms
// - Samples per ms: 48
// - Channels: 2 (Stereo)
// > Total: 120 * 48 * 2 = 11,520
//
// Im allocating more than that just to make sure we have enough
const RINGBUF_CAPACITY: usize = 12_000 * 2;

#[allow(unused)]
static ATLAS: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/rain.atlas"));

impl AudioSubsystem {
	pub fn new() -> anyhow::Result<Self> {
		// create ringbuf to give to both opus decoder thread and audio thread
		let ringbuffer = StaticRb::<f32, RINGBUF_CAPACITY>::default();
		let (mut prod, mut cons) = ringbuffer.split();

		// init cpal's device and config to create a stream later
		let device = Host::default()
			.default_output_device()
			.ok_or_else(|| anyhow::Error::msg("Failed to get default output device"))?;
		let config = device
			.supported_output_configs()?
			.find_map(|a| {
				if a.max_sample_rate() == 48000
					&& a.sample_format() == cpal::SampleFormat::F32
					&& a.channels() == 2
				{
					Some(a.with_max_sample_rate().config())
				} else {
					None
				}
			})
			.ok_or_else(|| anyhow::Error::msg("No supported stream config"))?;

		// spawn thread for our opus packets decoder
		let opus_handle = thread::spawn(move || {
			let mut decoder = match Decoder::new(config.sample_rate, Channels::Stereo) {
				Ok(d) => d,
				Err(e) => {
					error!("Failed to initialize opus decoder: {:#?}", e);
					panic!()
				},
			};
			let mut pcm_out = [0.0f32; 5760 * 10]; // Max opus frame size (120ms)

			loop {
				while prod.vacant_len() < pcm_out.len() / 10 {
					info!("Parking the thread");
					thread::park();
				}

				for x in OpusAtlas::load(ATLAS).iter() {
					match decoder.decode_float(x, &mut pcm_out, false) {
						// IMPORTANT:
						//
						// decode_float returns how many samples it decoded for a single channel.
						//
						// Since our audio source is a stereo signal (2 channels),
						// this means that the `samples_decoded` number will be 1/2 of
						// the number of samples it actually decoded.
						//
						// So we return from the beginning of the array the decoder wrote to,
						// to 2 * the num of samples the decode_float method returns
						Ok(num_samples_decoded) => {
							prod.push_slice(&pcm_out[..num_samples_decoded * 2]);
						},
						Err(e) => error!("Opus decoder failed to decode packet: {:#?}", e),
					};
				}
			}
		});

		let stream = device.build_output_stream(
			&config,
			move |b: &mut [f32], _| {
				// // Deinterleave mono
				// for i in b.chunks_mut(config.channels as usize) {
				// 	i.fill(rand::random::<f32>());
				// }

				// for i in b {
				// 	*i = rand::random::<f32>();
				// }

				// write samples from the ringbuf to the buffer slice
				info!("popping from the ring buffer");
				cons.pop_slice(b);

				// if were running low on writeable samples, wake the thread up
				if cons.occupied_len() < (b.len() * 2) {
					opus_handle.thread().unpark();
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

	/// Toggles playback of cpal stream between Playing and Paused
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
