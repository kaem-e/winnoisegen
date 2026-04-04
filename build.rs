use ogg::PacketReader;
use std::{
	env,
	fs::{self, File},
	io::{Cursor, Write},
	path::Path,
};

const INPUT_PATH: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/output.opus");

fn main() {
	// Trigger only on input change
	// println!("cargo:rerun-if-changed=output.opus");

	// Get the path to the Cargo-managed build directory
	let out_dir = env::var("OUT_DIR").unwrap();
	let dest_path = Path::new(&out_dir).join("rain.atlas");

	generate_atlas(&dest_path);
}

fn generate_atlas(output_path: &Path) {
	let raw_data = fs::read(INPUT_PATH).expect("Failed to read output.opus");
	let mut reader = PacketReader::new(Cursor::new(raw_data));

	let mut raw_opus_packets = Vec::with_capacity(30218);
	let mut offsets = Vec::new();
	let mut current_offset = 0u32;

	while let Ok(Some(packet)) = reader.read_packet() {
		offsets.push(current_offset);
		current_offset += packet.data.len() as u32;
		raw_opus_packets.extend_from_slice(&packet.data);
	}
	offsets.push(current_offset);

	let mut out_file = File::create(output_path).expect("Failed to create rain.atlas in OUT_DIR");

	out_file
		.write_all(&(offsets.len() as u32 - 1).to_le_bytes())
		.unwrap();

	for o in &offsets {
		out_file.write_all(&o.to_le_bytes()).unwrap();
	}

	out_file.write_all(&raw_opus_packets).unwrap();
}
