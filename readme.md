todo;



# INFO INFO FINFI

> !Info:
> The `rain.atlas` file in assets is the raw opus data unpacked. its done this
> way because just storing raw bytes of .opus meant i had to use ogg decoding
> which took ~4 secs optimized, unacceptable

## Binary Layout (The "Atlas" Format)
The file is a flat, little-endian binary blob structured as follows:

| Offset | Type | Name | Description |
|---|---|---|---|
| 0x00 | u32 | Count | Total number of Opus packets stored. |
| 0x04 | [u32; Count + 1] | Offset Table | Absolute pointers to packet starts, relative to the Data Section. |
| End of Table | [u8] | Data Section | All raw Opus packets concatenated back-to-back. |

## How to Resolve a Packet
To get packet `i`, you just calculate:

   1. Start Pointer: OffsetTable[i]
   2. End Pointer: OffsetTable[i + 1] (This is why we store Count + 1 offsets!)
   3. Data Slice: DataSection[Start..End]

[!TIP]
This layout is essentially a String Table for audio. It turns a sequential Ogg stream into a random-access array, making the "unpacking" step a simple memory-mapped slice.

Should I drop the build.rs code here so it just auto-generates this layout every time you hit cargo build?
