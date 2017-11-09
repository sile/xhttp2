use byteorder::{ByteOrder, BigEndian};

use {Result, ErrorKind};
use stream::StreamId;
use super::FrameHeader;

/// https://tools.ietf.org/html/rfc7540#section-6.9
///
/// ```text
///    +-+-------------------------------------------------------------+
///    |R|              Window Size Increment (31)                     |
///    +-+-------------------------------------------------------------+
///
///                  Figure 14: WINDOW_UPDATE Payload Format
/// ```
#[derive(Debug, Clone)]
pub struct WindowUpdateFrame {
    pub stream_id: StreamId,
    pub window_size_increment: u32,
}
impl WindowUpdateFrame {
    pub fn from_vec(header: &FrameHeader, payload: Vec<u8>) -> Result<Self> {
        track_assert_eq!(payload.len(), 4, ErrorKind::FrameSizeError);

        let window_size_increment = BigEndian::read_u32(&payload[..]) & 0x7FFF_FFFF;
        track_assert_ne!(window_size_increment, 0, ErrorKind::ProtocolError);

        Ok(WindowUpdateFrame {
            stream_id: header.stream_id,
            window_size_increment,
        })
    }
}
