use std::io::Read;
use byteorder::{BigEndian, ReadBytesExt};

use {Result, ErrorKind};
use super::FrameHeader;
use super::flags;

/// https://tools.ietf.org/html/rfc7540#section-6.6
///
/// ```text
///    +---------------+
///    |Pad Length? (8)|
///    +-+-------------+-----------------------------------------------+
///    |R|                  Promised Stream ID (31)                    |
///    +-+-----------------------------+-------------------------------+
///    |                   Header Block Fragment (*)                 ...
///    +---------------------------------------------------------------+
///    |                           Padding (*)                       ...
///    +---------------------------------------------------------------+
///
///                  Figure 11: PUSH_PROMISE Payload Format
/// ```
#[derive(Debug, Clone)]
pub struct PushPromiseFrame {
    pub stream_id: u32,
    pub promised_stream_id: u32,
    pub end_headers: bool,
    pub padding_len: u8,
    pub fragment: Vec<u8>,
}
impl PushPromiseFrame {
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

        let promised_stream_id = track_io!(reader.read_u32::<BigEndian>())? & 0x7FFF_FFFF;
        fragment_len -= 4;

        let mut fragment = vec![0; fragment_len];
        track_io!(reader.read_exact(&mut fragment))?;

        Ok(PushPromiseFrame {
            stream_id: header.stream_id,
            promised_stream_id,
            end_headers: (header.flags & flags::END_HEADERS) != 0,
            padding_len,
            fragment,
        })
    }
}
