# Opus File

An Opus Stream is almost always contained in an Ogg container.
Within the Ogg file there need to be two headers:
 - [[Opus File#Identification Header|The Identification Header]]
 - [[#Comment Header|The Comment Header]]

[Opus File Specification](https://datatracker.ietf.org/doc/html/rfc7845.html)

## Ogg

Everything is an Ogg page in an Ogg file. Meaning that the file start immediate with a page.
[Wikipedia](https://en.wikipedia.org/wiki/Ogg "Ogg")

### Page

![[Ogg_page_header_structure_(en).svg]]
The Capture Pattern will should always contain "OggS". Marking the start of a page
The version should be 0.
The header type has three bit flags where only one can be active at any time:
- `0000 0001`: continuation of stream
- `0000 0010`: start of stream
- `0000 0100`: end of stream, or only page in a stream
The Granule Position is the time marker, what is means depends on the codec that is in the Ogg page, it is 8 bytes.
The Bitstream Serial Number is a unique identifier for the logical bitstream, it is 4 bytes.
The Page Sequence Number is a monotonically increasing field for each logical bitstream. The first page is 0, the second 1, etc. It is 4 bytes.
Checksum, speaks for itself. Uses [CRC32](https://en.wikipedia.org/wiki/Computation_of_cyclic_redundancy_checks#CRC-32_algorithm)
The Page Segments indicates the number of segments in the page
The Segment table indicates the length of each segment
When the segment's length is 255, this indicates that the following segment is part of the same packet.
When the segment's length is 0–254, this indicates that this segment is the final segment in this packet.
When the full packet's length is a multiple of 255, the final segment is length 0.

### Package

After the Segment table the packages start.
Packages should be the smallest form for a media stream, one frame, one tick etc.
 
## Identification Header

![[opus_head_structure.svg]]
The Magic Signature is "OpusHead"
The Version should always be 1
The Channels need to be larger than 0
The Pre-Skip marks how much samples you need to skip/discard form the decoder, at 48 kHz
The Input Sample Rate is how at how much Hz the original input stream was before encoding, during decoding you should do NOTHING with this
The Output Gain, should to be applied to the output stream 
Mapping, The Channel Mapping sounds to unimportant/complicated. It refers to how the output channels should be handled.
This is further described in the Channel Mapping Table.
If the Mapping is zero you can probably ignore it.

## Comment Header

Just research Vorbis comments, it is the same.
The Magic Signature is "OpusTags".