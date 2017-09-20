use byteorder::{ByteOrder, BigEndian};

use {Result, Error, ErrorKind};
use super::FrameHeader;

/// https://tools.ietf.org/html/rfc7540#section-6.4
///
/// ```text
///    +---------------------------------------------------------------+
///    |                        Error Code (32)                        |
///    +---------------------------------------------------------------+
///
///                    Figure 9: RST_STREAM Frame Payload
/// ```
#[derive(Debug, Clone)]
pub struct RstStreamFrame {
    pub stream_id: u32,
    pub error: Error,
}
impl RstStreamFrame {
    pub fn from_vec(header: &FrameHeader, payload: Vec<u8>) -> Result<Self> {
        track_assert_ne!(header.stream_id, 0, ErrorKind::ProtocolError);
        track_assert_eq!(payload.len(), 5, ErrorKind::FrameSizeError);

        let code = BigEndian::read_u32(&payload[..]);
        let error = Error::from_code(code);
        Ok(RstStreamFrame {
            stream_id: header.stream_id,
            error,
        })
    }
}
