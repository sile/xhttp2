use {Result, ErrorKind};
use super::FrameHeader;
use super::flags;

/// https://tools.ietf.org/html/rfc7540#section-6.10
///
/// ```text
///    +---------------------------------------------------------------+
///    |                   Header Block Fragment (*)                 ...
///    +---------------------------------------------------------------+
///
///                   Figure 15: CONTINUATION Frame Payload
/// ```
#[derive(Debug, Clone)]
pub struct ContinuationFrame {
    pub stream_id: u32,
    pub end_headers: bool,
    pub fragment: Vec<u8>,
}
impl ContinuationFrame {
    pub fn from_vec(header: &FrameHeader, payload: Vec<u8>) -> Result<Self> {
        track_assert_ne!(header.stream_id, 0, ErrorKind::ProtocolError);
        Ok(ContinuationFrame {
            stream_id: header.stream_id,
            end_headers: (header.flags & flags::END_HEADERS) != 0,
            fragment: payload,
        })
    }
}
