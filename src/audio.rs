use anyhow::Context as _;
use cpal::{
	self, Host, SampleFormat, Stream,
	traits::{DeviceTrait, HostTrait, StreamTrait},
};
use log::*;
use qoaudio::{QoaDecoder, QoaItem};
use ringbuf::{
	HeapRb,
	traits::{Consumer, Observer, Producer, Split},
};
use std::{simd::prelude::*, thread, time::Duration};

/// Manually enumerated enum representing the current playback state of the audio subsystem
// this is only really loosely synced with the cpal stream itself. idk it seems ot be working fine so far byt yeah
#[derive(Debug, Clone)]
pub enum PlaybackState {
	Playing,
	Paused,
}

/// Subsystem that interfaces with the entire audio system.
/// This is a largely independent system,
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
		let (prod, mut cons) = ringbuffer.split();

		// init cpal's device and config to create a stream later
		let device = Host::default()
			.default_output_device()
			.context("Failed to get default output device")?;

		let config = &device
			.supported_output_configs()
			.context("Failed to get default output configs for device")?
			.find(|c| {
				c.sample_format() == SampleFormat::F32
					&& c.channels() == 2
					&& (c.min_sample_rate()..=c.max_sample_rate()).contains(&44100u32)
			})
			.context("Failed to find suitable config for device output configs")?
			.try_with_sample_rate(44100)
			.context("Failed to get config with sample rate")?
			.config();

		// spawn thread for our poa audio file decoder
		// this sends decoded samples to the ringbuffer when needed
		let _ = thread::spawn(move || decoder_thread(prod));

		// let mut sine_osc_phase: f32 = 0.0;
		// let mut sine_osc = move || -> f32 {
		//     use std::f32::consts::PI;
		//     const PHASE_INC: f32 = (2.0 * PI / 44100.0) * 440.0;
		//     let result = sine_osc_phase.sin();
		//     sine_osc_phase = (sine_osc_phase + PHASE_INC) % (2.0 * PI);
		//     result
		// };

		let stream = device.build_output_stream(
			&config,
			move |frame: &mut [f32], _| {
				// write samples from the ringbuf to the buffer slice
				cons.pop_slice(frame);

				// if cons.is_empty() {
				// 	b.fill(0.0);
				// }
			},
			|_| {},
			None,
		)?;

		Ok(Self {
			stream,
			playback_state: PlaybackState::Paused, // cpal stream is paused by default
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
		}

		Ok(())
	}

	/// Retrive the current playback state
	///
	/// See comment at definition if you want to know why the `playback_state` field isnt just pub
	pub fn get_playback_state(&self) -> PlaybackState {
		// I wrote a getter for this instead of just making the field itself pub
		// because the playback state needs to stay in sync with the cpal stream,
		// making it pub can mean anyone using the library can just change it
		// breaking the logic. so yeah we just keep it private and return copies if
		// someone needs them
		self.playback_state.clone()
	}
}

/// Function that creates our decoder thread logic. call this in [`std::thread::spawn`]
///
/// Example:
/// ```rust
/// let handle = thread::spawn(move || decoder_thread(prod));
/// ```
fn decoder_thread(mut prod: ringbuf::HeapProd<f32>) {
	const BATCH_SIZE: usize = 1024;
	const I16_MAX_INV: f32 = 1.0 / i16::MAX as f32; // Multiplication is much faster than division in SIMD operations

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
}
