use {Result, ErrorKind};
use super::{FrameHeader, Priority};

/// https://tools.ietf.org/html/rfc7540#section-6.3
///
/// ```text
///    +-+-------------------------------------------------------------+
///    |E|                  Stream Dependency (31)                     |
///    +-+-------------+-----------------------------------------------+
///    |   Weight (8)  |
///    +-+-------------+
///
///                     Figure 8: PRIORITY Frame Payload
/// ```
#[derive(Debug, Clone)]
pub struct PriorityFrame {
    pub stream_id: u32,
    pub priority: Priority,
}
impl PriorityFrame {
    pub fn from_vec(header: &FrameHeader, payload: Vec<u8>) -> Result<Self> {
        track_assert_ne!(header.stream_id, 0, ErrorKind::ProtocolError);
        track_assert_eq!(payload.len(), 5, ErrorKind::FrameSizeError);

        let priority = track!(Priority::read_from(&payload[..]))?;
        Ok(PriorityFrame {
            stream_id: header.stream_id,
            priority,
        })
    }
}
