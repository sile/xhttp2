use byteorder::{ByteOrder, BigEndian};

use {Result, Error, ErrorKind};
use stream::StreamId;
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
    pub stream_id: StreamId,
    pub error: Error,
}
impl RstStreamFrame {
    pub fn from_vec(header: &FrameHeader, payload: Vec<u8>) -> Result<Self> {
        track_assert!(
            !header.stream_id.is_connection_control_stream(),
            ErrorKind::ProtocolError
        );
        track_assert_eq!(payload.len(), 4, ErrorKind::FrameSizeError);

        let code = BigEndian::read_u32(&payload[..]);
        let error = Error::from_code(code);
        Ok(RstStreamFrame {
            stream_id: header.stream_id,
            error,
        })
    }
}
