use std::io::Read;
use byteorder::ReadBytesExt;

use {Result, ErrorKind};
use super::{FrameHeader, Priority};
use super::flags;

/// https://tools.ietf.org/html/rfc7540#section-6.2
///
/// ```text
///    +---------------+
///    |Pad Length? (8)|
///    +-+-------------+-----------------------------------------------+
///    |E|                 Stream Dependency? (31)                     |
///    +-+-------------+-----------------------------------------------+
///    |  Weight? (8)  |
///    +-+-------------+-----------------------------------------------+
///    |                   Header Block Fragment (*)                 ...
///    +---------------------------------------------------------------+
///    |                           Padding (*)                       ...
///    +---------------------------------------------------------------+
///
///                      Figure 7: HEADERS Frame Payload
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadersFrame {
    pub stream_id: u32,
    pub end_stream: bool,
    pub end_headers: bool,
    pub priority: Option<Priority>,
    pub padding_len: u8,
    pub fragment: Vec<u8>,
}
impl HeadersFrame {
    pub fn from_vec(header: &FrameHeader, payload: Vec<u8>) -> Result<Self> {
        track_assert_ne!(header.stream_id, 0, ErrorKind::ProtocolError);

        let mut reader = &payload[..];
        let mut fragment_len = header.payload_length as usize;

        let padding_len = if (header.flags & flags::PADDED) != 0 {
            fragment_len -= 1;
            track_io!(reader.read_u8())?
        } else {
            0
        };
        fragment_len -= padding_len as usize;

        let priority = if (header.flags & flags::PRIORITY) != 0 {
            fragment_len -= 5;
            Some(track!(Priority::read_from(&mut reader))?)
        } else {
            None
        };

        let mut fragment = vec![0; fragment_len];
        track_io!(reader.read_exact(&mut fragment))?;

        Ok(HeadersFrame {
            stream_id: header.stream_id,
            end_stream: (header.flags & flags::END_STREAM) != 0,
            end_headers: (header.flags & flags::END_HEADERS) != 0,
            priority,
            padding_len,
            fragment,
        })
    }
}
