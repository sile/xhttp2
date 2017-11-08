use {Result, ErrorKind};

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
