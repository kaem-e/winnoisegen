/// OpusAtlas
///
/// A flat, zero-parse container for our Opus packets.
///
/// Layout:
/// [u32: packet_count]
/// [u32: offsets[packet_count + 1]] // includes sentinel
/// [u8: packet_data...]
///
/// Offsets are relative to the start of the packet_data section.
///
/// Example:
/// packet_count = 3
/// offsets = [0, 120, 250, 400]
///
/// Packets:
/// packet 0 = data[0..120]
/// packet 1 = data[120..250]
/// packet 2 = data[250..400]
///
/// The final offset is a sentinel marking total data length.
///
/// Guarantees required:
/// - offsets must be monotonically increasing
/// - last offset <= data length
/// - offsets.len() == packet_count + 1
pub struct OpusAtlas {
	data: &'static [u8],
	count: usize,
	data_start: usize,
}

impl OpusAtlas {
	pub fn load(bytes: &'static [u8]) -> Self {
		let count = u32::from_le_bytes(bytes[0..4].try_into().unwrap()) as usize;
		let data_start = 4 + ((count + 1) * 4);
		Self {
			data: bytes,
			count,
			data_start,
		}
	}
	fn offset(&self, i: usize) -> usize {
		let ptr = 4 + (i * 4);
		u32::from_le_bytes(self.data[ptr..ptr + 4].try_into().unwrap()) as usize
	}
	pub fn get_packet(&self, i: usize) -> Option<&[u8]> {
		if i >= self.count {
			return None;
		}
		let start = self.offset(i);
		let end = self.offset(i + 1);
		Some(&self.data[self.data_start + start..self.data_start + end])
	}
	pub fn iter(&self) -> OpusAtlasIter<'_> {
		OpusAtlasIter {
			atlas: self,
			index: 0,
		}
	}
}

pub struct OpusAtlasIter<'a> {
	atlas: &'a OpusAtlas,
	index: usize,
}

impl<'a> Iterator for OpusAtlasIter<'a> {
	type Item = &'a [u8];
	fn next(&mut self) -> Option<Self::Item> {
		let packet = self.atlas.get_packet(self.index)?;
		self.index += 1;
		Some(packet)
	}
}
