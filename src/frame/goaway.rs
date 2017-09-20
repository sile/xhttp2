use byteorder::{ByteOrder, BigEndian};

use {Result, Error, ErrorKind};
use super::FrameHeader;

/// https://tools.ietf.org/html/rfc7540#section-6.8
///
/// ```text
///    +-+-------------------------------------------------------------+
///    |R|                  Last-Stream-ID (31)                        |
///    +-+-------------------------------------------------------------+
///    |                      Error Code (32)                          |
///    +---------------------------------------------------------------+
///    |                  Additional Debug Data (*)                    |
///    +---------------------------------------------------------------+
///
///                     Figure 13: GOAWAY Payload Format
/// ```
#[derive(Debug, Clone)]
pub struct GoawayFrame {
    pub last_stream_id: u32,
    pub error: Error,
    pub debug_data: Vec<u8>,
}
impl GoawayFrame {
    pub fn from_vec(header: &FrameHeader, mut payload: Vec<u8>) -> Result<Self> {
        track_assert_eq!(header.stream_id, 0, ErrorKind::ProtocolError);

        let last_stream_id = BigEndian::read_u32(&payload[0..4]) & 0x7FFF_FFFF;
        let error = Error::from_code(BigEndian::read_u32(&payload[4..8]));
        payload.drain(0..8);
        Ok(GoawayFrame {
            last_stream_id,
            error,
            debug_data: payload,
        })
    }
}
