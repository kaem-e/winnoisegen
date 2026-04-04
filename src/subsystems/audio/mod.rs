use cpal::{
	self, Host, Stream,
	traits::{DeviceTrait, HostTrait, StreamTrait},
};
use log::error;
use qoaudio::{QoaDecoder, QoaItem};
use ringbuf::{
	StaticRb,
	traits::{Consumer, Observer, Producer, Split},
};
use std::thread;

enum PlaybackState {
	Playing,
	Paused,
}

pub struct AudioSubsystem {
	stream: Stream,
	playback_state: PlaybackState,
}

#[rustfmt::skip]
static QOA_BINARY_BLOB: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/rain.qoa"));
const RINGBUF_CAPACITY: usize = 40_000; // enough for 1 second (2 x 24_000 as stereo interleaved data)

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

		// spawn thread for our poa audio file decoder
		let decoder_handle = thread::spawn(move || {
			loop {
				let mut decoder = match QoaDecoder::new(QOA_BINARY_BLOB) {
					Ok(d) => d,
					Err(e) => {
						error!("Failed to initialize opus decoder: {:#?}", e);
						panic!()
					},
				}
				.filter_map(|a| match a {
					Ok(QoaItem::Sample(a)) => Some(a as f32 / i16::MAX as f32),
					Ok(QoaItem::FrameHeader(_)) => None,
					Err(e) => {
						error!("Error while decoding qoa frame: {:?}", e);
						None
					},
				})
				.peekable();

				while decoder.peek().is_some() {
					while prod.vacant_len() < 50 {
						thread::park();
					}

					prod.push_iter(&mut decoder);
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
				cons.pop_slice(b);

				// if were running low on writeable samples, wake the thread up
				if cons.occupied_len() < (b.len() * 2) {
					decoder_handle.thread().unpark();
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
