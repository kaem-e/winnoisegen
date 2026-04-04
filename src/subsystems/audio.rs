use cpal::{
	self, Host, Stream,
	traits::{DeviceTrait, HostTrait, StreamTrait},
};
use log::error;
use qoaudio::{QoaDecoder, QoaItem};
use ringbuf::{
	HeapRb,
	traits::{Consumer, Observer, Producer, Split},
};
use std::{simd::prelude::*, thread, time::Duration};

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
const RINGBUF_CAPACITY: usize = 100_000; // enough for 1 second ≈(2 x 48_000 as stereo interleaved data)

impl AudioSubsystem {
	pub fn new() -> anyhow::Result<Self> {
		// create ringbuf to give to both opus decoder thread and audio thread
		let ringbuffer = HeapRb::<f32>::new(RINGBUF_CAPACITY);
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

		let _ = thread::spawn(move || {
			const BATCH_SIZE: usize = 1024;
			// Multiplication is much faster than division in SIMD operations
			const I16_MAX_INV: f32 = 1.0 / i16::MAX as f32;

			// Pre-allocate buffers outside the loop to avoid memory allocations in the hot path
			let mut i16_buf = vec![0i16; BATCH_SIZE];
			let mut f32_buf = vec![0.0f32; BATCH_SIZE];

			loop {
				let mut decoder = match QoaDecoder::new(QOA_BINARY_BLOB) {
					Ok(d) => d,
					Err(e) => {
						// Fixed the copy-paste log (said opus, actually qoa)
						error!("Failed to initialize qoa decoder: {:#?}", e);
						panic!()
					},
				};

				let mut eof = false;

				while !eof {
					let mut samples_collected = 0;

					// 1. Grouped Read: Fill the intermediate i16 buffer
					while samples_collected < BATCH_SIZE {
						match decoder.next() {
							Some(Ok(QoaItem::Sample(s))) => {
								i16_buf[samples_collected] = s;
								samples_collected += 1;
							},
							Some(Ok(QoaItem::FrameHeader(_))) => continue,
							Some(Err(e)) => {
								error!("Error while decoding qoa frame: {:?}", e);
								break; // Try to push what we have, then probably restart stream
							},
							None => {
								eof = true;
								break;
							},
						}
					}

					if samples_collected == 0 {
						break; // Hit EOF right away, restart the decoder loop
					}

					let i16_slice = &i16_buf[..samples_collected];
					let f32_slice = &mut f32_buf[..samples_collected];

					// Inside the loop:
					let (i16_chunks, i16_rem) = i16_slice.as_chunks::<8>();
					let (f32_chunks, f32_rem) = f32_slice.as_chunks_mut::<8>();

					let multiplier = f32x8::splat(I16_MAX_INV);

					// Map SIMD chunks
					for (out_chunk, in_chunk) in f32_chunks.iter_mut().zip(i16_chunks.iter()) {
						let in_simd = i16x8::from_array(*in_chunk);
						// Upcast to i32 to allow f32 casting
						let f32_simd = in_simd.cast::<i32>().cast::<f32>();
						let result = f32_simd * multiplier;
						*out_chunk = result.to_array();
					}

					// Clean up any remaining samples at the end of the slice (if collected < 1024)
					for (out_f, &in_i) in f32_rem.iter_mut().zip(i16_rem.iter()) {
						*out_f = (in_i as f32) * I16_MAX_INV;
					}

					// 3. Ringbuffer Sleep Wait
					// Wait until we have exact space for our batched slice.
					while prod.vacant_len() < samples_collected {
						thread::sleep(Duration::from_millis(1)); // 1ms is much safer than 10ms
					}

					// 4. Grouped Write
					prod.push_slice(f32_slice);
				}
			}
		});

		let stream = device.build_output_stream(
			&config,
			move |b: &mut [f32], _| {
				// write samples from the ringbuf to the buffer slice
				cons.pop_slice(b);
				// if cons.is_empty() {
				// 	b.fill(0.0);
				// }
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
