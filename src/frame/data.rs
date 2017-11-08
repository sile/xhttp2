use {Result, ErrorKind};
use stream::StreamId;
use super::FrameHeader;
use super::flags;

/// https://tools.ietf.org/html/rfc7540#section-6.1
///
/// ```text
///    +---------------+
///    |Pad Length? (8)|
///    +---------------+-----------------------------------------------+
///    |                            Data (*)                         ...
///    +---------------------------------------------------------------+
///    |                           Padding (*)                       ...
///    +---------------------------------------------------------------+
///
///                       Figure 6: DATA Frame Payload
/// ```
#[derive(Debug, Clone)]
pub struct DataFrame {
    pub stream_id: StreamId,
    pub end_stream: bool,
    pub padding_len: u8,
    pub data: Vec<u8>,
}
impl DataFrame {
    pub fn from_vec(header: &FrameHeader, mut payload: Vec<u8>) -> Result<Self> {
        track_assert!(
            !header.stream_id.is_connection_control_stream(),
            ErrorKind::ProtocolError
        );

        let mut padding_len = 0;
        if (header.flags & flags::PADDED) != 0 {
            // FIXME: optimize
            padding_len = payload.remove(0);

            let payload_len = payload.len();
            track_assert!(
                payload_len >= padding_len as usize,
                ErrorKind::ProtocolError
            );
            payload.truncate(payload_len - padding_len as usize);
        }
        Ok(DataFrame {
            stream_id: header.stream_id,
            padding_len,
            data: payload,
            end_stream: (header.flags & flags::END_STREAM) != 0,
        })
    }
}
