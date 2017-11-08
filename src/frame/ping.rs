use {Result, ErrorKind};
use super::FrameHeader;
use super::flags;

/// https://tools.ietf.org/html/rfc7540#section-6.7
///
/// ```text
///    +---------------------------------------------------------------+
///    |                                                               |
///    |                      Opaque Data (64)                         |
///    |                                                               |
///    +---------------------------------------------------------------+
///
///                      Figure 12: PING Payload Format
/// ```
#[derive(Debug, Clone)]
pub struct PingFrame {
    pub ack: bool,
    pub data: [u8; 8],
}
impl PingFrame {
    pub fn from_vec(header: &FrameHeader, payload: Vec<u8>) -> Result<Self> {
        track_assert!(
            header.stream_id.is_connection_control_stream(),
            ErrorKind::ProtocolError
        );
        track_assert_eq!(payload.len(), 8, ErrorKind::FrameSizeError);

        let mut data = [0; 8];
        data.copy_from_slice(&payload[..]);

        Ok(PingFrame {
            ack: (header.flags & flags::ACK) != 0,
            data,
        })
    }
}
