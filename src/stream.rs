use {Result, ErrorKind};

/// Stream Identifier:  A stream identifier (see Section 5.1.1) expressed
/// as an unsigned 31-bit integer.  The value 0x0 is reserved for
/// frames that are associated with the connection as a whole as
/// opposed to an individual stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StreamId(u32);
impl StreamId {
    pub fn new(id: u32) -> Result<Self> {
        track_assert_eq!(id >> 31, 0, ErrorKind::InternalError);
        Ok(StreamId(id))
    }
    pub fn connection_control_stream_id() -> Self {
        StreamId(0)
    }
    pub fn is_connection_control_stream(&self) -> bool {
        self.0 == 0
    }
    pub(crate) fn new_unchecked(id: u32) -> Self {
        StreamId(id)
    }
    pub fn as_u32(&self) -> u32 {
        self.0
    }
}
impl From<u8> for StreamId {
    fn from(f: u8) -> Self {
        StreamId(u32::from(f))
    }
}
impl From<u16> for StreamId {
    fn from(f: u16) -> Self {
        StreamId(u32::from(f))
    }
}
