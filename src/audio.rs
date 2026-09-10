use crate::audio::consumer_access::UnsafeConsumerExclusiveAccess as _;
use anyhow::Context as _;
use cpal::{
	self, ErrorKind, Host, SampleFormat, Stream,
	traits::{DeviceTrait, HostTrait, StreamTrait},
};
use ringbuf::{
	HeapRb,
	traits::{Observer, Producer, Split},
};
use std::{simd::prelude::*, thread, time::Duration};
use tracing::*;

/// `msg` range on the windows message type that the tray icon sends its events to
pub const EVENTGROUP_AUDIO: u32 = windows::Win32::UI::WindowsAndMessaging::WM_APP + 2;
/// WPARAM value corresponding to a default device switched event
pub const EVENT_DEFAULT_DEVICE_SWITCHED: usize = 0;

/// Manually enumerated enum representing the current playback state of the audio subsystem
#[derive(Debug, Clone)]
pub enum PlaybackState {
	Playing,
	Paused,
}

struct StreamState {
	stream: Stream,
	playback_state: PlaybackState,
}

type Cons = ringbuf::HeapCons<f32>;

/// Subsystem that interfaces with the entire audio system.
/// This is a largely independent system,
pub struct AudioSubsystem {
	consumer: Cons,
	cpal_stream: Option<StreamState>,
}

#[rustfmt::skip]
static QOA_BINARY_BLOB: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/rain.qoa"));
const RINGBUF_CAPACITY: usize = 100_000; // enough for 1 second ≈(2 x 48_000 as stereo interleaved data)

impl AudioSubsystem {
	pub fn new() -> anyhow::Result<Self> {
		// create ringbuf to give to both opus decoder thread and audio thread
		let ringbuffer = HeapRb::<f32>::new(RINGBUF_CAPACITY);
		let (prod, cons) = ringbuffer.split();

		// spawn thread for our poa audio file decoder
		// this sends decoded samples to the ringbuffer
		let _ = thread::spawn(move || {
			let _guard = span!(Level::DEBUG, "decoder_thread");
			decoder_thread(prod)
		});

		Ok(Self {
			consumer: cons,
			cpal_stream: None,
		})
	}

	pub fn regenerate_stream(&mut self) -> anyhow::Result<()> {
		debug!("regenerating cpal stream");

		// SAFETY:
		// Cpal implements Drop on its stream object that joins the audio thread
		// it spawns, blocking until the thread does join.
		//
		// By explicitly dropping the stream here, we guaratee the previous
		// ConsumerAccess is dropped before we re-initialize it, i.e. no thread
		// can use it to access the consumer.
		//
		// This upholds the invariant for the spsc consumer that only one thread
		// can access it at a time, thus creating this is always safe after
		// dropping the cpal stream
		//
		// for wasapi: https://github.com/RustAudio/cpal/blob/e1612d5d98152f8dc2a62e1b51ef7cbf4f7f26b7/src/host/wasapi/stream.rs#L479-L495
		// TODO: check if this upholds for other hosts, ive only verified windows
		drop(self.cpal_stream.take());
		let mut access = unsafe { self.consumer.unsafe_get_exclusive_access() };

		let device = Host::default()
			.default_output_device()
			.context("Failed to get default output device")?;

		let config = device
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

		let stream = device.build_output_stream(
			config,
			move |frame: &mut [f32], _| {
				// write samples from the ringbuf to the buffer slice
				access.fill_frame(frame);
			},
			|e| {
				match e.kind() {
					// Small errors that can recover/are non-critical so we just continue and issue a warning
					ErrorKind::Xrun | ErrorKind::RealtimeDenied | ErrorKind::DeviceBusy => {
						warn!("Non-critical error: {:?}", e);
					},

					// Error that indicates invalidation, but can be recovered from by rebuilding the stream
					ErrorKind::DeviceChanged
					| ErrorKind::DeviceNotAvailable
					| ErrorKind::InvalidInput
					| ErrorKind::StreamInvalidated
					| ErrorKind::UnsupportedConfig => {
						warn!("Stream invalidation error: {e:?}. Posting message to rebuild stream.");
						let thread_id = crate::MAIN_THREAD_ID.load(std::sync::atomic::Ordering::Acquire);

						use windows::Win32::Foundation::{LPARAM, WPARAM};
						use windows::Win32::UI::WindowsAndMessaging::PostThreadMessageW;

						match unsafe {
							PostThreadMessageW(thread_id, EVENTGROUP_AUDIO, WPARAM(0), LPARAM(0))
						} {
							Ok(()) => {},
							Err(e) => error!("Failed pushing to Win32 Queue: {:#?}", e),
						};
					},

					// errors that theres no way to recover from so we just exit the application
					ErrorKind::HostUnavailable
					| ErrorKind::PermissionDenied
					| ErrorKind::ResourceExhausted
					| ErrorKind::BackendError
					| ErrorKind::UnsupportedOperation
					| ErrorKind::Other => {
						error!("Fatal error: {:?}", e);
						unsafe { windows::Win32::UI::WindowsAndMessaging::PostQuitMessage(2) }
					},

					// Catch all that just exists the app for a yet nonidentified error. we just exit the process outright
					_ => {
						error!("Unknown error: {:?}", e);
						std::process::exit(1)
					},
				}
			},
			None,
		)?;

		self.cpal_stream = Some(StreamState {
			stream,
			playback_state: PlaybackState::Paused, // cpal stream is paused by default
		});
		Ok(())
	}

	/// Toggles playback of cpal stream between Playing and Paused
	pub fn toggle_playback(&mut self) -> anyhow::Result<()> {
		let _guard = span!(Level::DEBUG, "Playback Toggle");

		let Some(state) = &mut self.cpal_stream else {
			return Err(anyhow::anyhow!("No stream to toggle playback on"));
		};

		match state.playback_state {
			PlaybackState::Playing => {
				state.stream.pause()?;
				state.playback_state = PlaybackState::Paused;
			},
			PlaybackState::Paused => {
				state.stream.play()?;
				state.playback_state = PlaybackState::Playing;
			},
		}

		debug!(
			"Toggled Playback: playback_state = {:?}",
			state.playback_state
		);

		Ok(())
	}

	/// Retrive the current playback state
	///
	/// See comment at definition if you want to know why the `playback_state` field isnt just pub
	pub fn get_playback_state(&self) -> anyhow::Result<PlaybackState> {
		// I wrote a getter for this instead of just making the field itself pub
		// because the playback state needs to stay in sync with the cpal stream,
		// making it pub can mean anyone using the library can just change it
		// breaking the logic. so yeah we just keep it private and return copies if
		// someone needs them

		if let Some(state) = &self.cpal_stream {
			Ok(state.playback_state.clone())
		} else {
			anyhow::bail!("No stream to get playback state from")
		}
	}
}

/// Function that creates our decoder thread logic. call this in [`std::thread::spawn`]
///
/// Example:
/// ```rust
/// let handle = thread::spawn(move || decoder_thread(prod));
/// ```
fn decoder_thread(mut prod: ringbuf::HeapProd<f32>) {
	use qoaudio::{QoaDecoder, QoaItem};

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
					Some(Ok(QoaItem::FrameHeader(_h))) => {
						trace!("Frame header read: {_h:?}");
					},
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

mod consumer_access {
	use crate::audio::Cons;
	use ringbuf::traits::Consumer as _;
	use tracing::*;

	#[cfg(debug_assertions)]
	use std::sync::atomic::AtomicBool;
	#[cfg(debug_assertions)]
	static CONSUMER_ACCESS_POINTER_ACTIVE: AtomicBool = AtomicBool::new(false);

	pub struct ExclusiveConsumerAccess {
		_ptr: *mut Cons,
	}

	unsafe impl Send for ExclusiveConsumerAccess {}

	impl ExclusiveConsumerAccess {
		unsafe fn new(consumer: &mut Cons) -> Self {
			#[cfg(debug_assertions)]
			use std::sync::atomic::Ordering;

			// Validate that no other access is currently in progress.
			// Only does this verification on debug builds to let release builds be fast
			#[cfg(debug_assertions)]
			assert_eq!(
				CONSUMER_ACCESS_POINTER_ACTIVE.load(Ordering::Acquire),
				false
			);

			// Mark that access is in progress.
			#[cfg(debug_assertions)]
			CONSUMER_ACCESS_POINTER_ACTIVE.store(true, Ordering::Release);

			Self {
				_ptr: consumer as *mut _,
			}
		}

		pub fn fill_frame(&mut self, frame: &mut [f32]) {
			// SAFETY:
			//
			// The invariants established by `create_access()` guarantee
			// that this pointer is valid and exclusively accessed.
			let prod = unsafe { &mut (*self._ptr) };

			let n = prod.pop_slice(frame);
			if n != frame.len() {
				warn!("Ringbuffer Underflow, filling remaining frame with silence",);
				debug!("expected {} elements, got {}", frame.len(), n);

				frame[n..].fill(0.0);
			}
		}
	}

	#[cfg(debug_assertions)]
	impl Drop for ExclusiveConsumerAccess {
		fn drop(&mut self) {
			use std::sync::atomic::Ordering;

			tracing::trace!("dropping ConsumerAccess");

			// Mark that access is no longer in progress.
			CONSUMER_ACCESS_POINTER_ACTIVE.store(false, Ordering::Release);
		}
	}

	pub trait UnsafeConsumerExclusiveAccess {
		/// Creates temporary exclusive access to `consumer`.
		///
		/// # Safety
		///
		/// The caller must guarantee that:
		///
		/// 1. `consumer` remains alive while this access exists.
		/// 2. No other access to `consumer` occurs while this access exists.
		unsafe fn unsafe_get_exclusive_access(&mut self) -> ExclusiveConsumerAccess;
	}

	impl UnsafeConsumerExclusiveAccess for Cons {
		unsafe fn unsafe_get_exclusive_access(&mut self) -> ExclusiveConsumerAccess {
			unsafe { ExclusiveConsumerAccess::new(self) }
		}
	}
}
