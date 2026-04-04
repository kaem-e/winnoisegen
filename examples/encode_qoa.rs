use std::fs;

use hound::{self, WavReader};
use qoaudio::{QoaDesc, encode_all};

fn main() {
	println!("Penis");

	// // QOA expects i16 samples
	let mut reader = WavReader::open("assets/output.wav").expect("Where is output.wav?");
	let pcm_samples: Vec<i16> = reader.samples::<i16>().map(|s| s.unwrap()).collect();

	let spec = reader.spec();
	let bytes = encode_all(
		&pcm_samples,
		&QoaDesc {
			channels: spec.channels as u8,
			sample_rate: spec.sample_rate,
			// QOA expects total samples *per channel*
			samples: (pcm_samples.len() / spec.channels as usize) as u32,
		},
	)
	.expect("Encoding failed");

	fs::write(OUT_PATH, bytes).expect("Failed to write file");
	println!("Success! rain.qoa created.");
}

const OUT_PATH: &str = "assets/rain.qoa";
