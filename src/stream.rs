use std::io::Read;
use byteorder::{ByteOrder, BigEndian};
use fibers::sync::mpsc;
use futures::{Future, Poll};
use handy_async::io::AsyncRead;
use handy_async::io::futures::ReadExact;

use {Result, ErrorKind, Error};
use header::Header;

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
    pub fn is_client_initiated_stream(&self) -> bool {
        self.0 % 2 == 1
    }
    pub fn is_server_initiated_stream(&self) -> bool {
        self.0 % 2 == 0
    }
    pub(crate) fn new_unchecked(id: u32) -> Self {
        StreamId(id)
    }
    pub fn as_u32(&self) -> u32 {
        self.0
    }

    pub fn read_from<R: Read>(reader: R) -> ReadStreamId<R> {
        ReadStreamId(reader.async_read_exact([0; 4]))
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

#[derive(Debug)]
pub struct ReadStreamId<R>(ReadExact<R, [u8; 4]>);
impl<R> ReadStreamId<R> {
    pub fn reader(&self) -> &R {
        self.0.reader()
    }
    pub fn reader_mut(&mut self) -> &mut R {
        self.0.reader_mut()
    }
}
impl<R: Read> Future for ReadStreamId<R> {
    type Item = (R, StreamId);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_async_io!(self.0.poll())?.map(|(reader, bytes)| {
            let stream_id = StreamId::new_unchecked(BigEndian::read_u32(&bytes[..]) & 0x7FFF_FFFF);
            (reader, stream_id)
        }))
    }
}

#[derive(Debug)]
pub struct Stream {
    id: StreamId,
    tx: mpsc::Sender<(StreamId, StreamItem)>,
    rx: mpsc::Receiver<StreamItem>,
}
impl Stream {
    pub fn new(id: StreamId, tx: mpsc::Sender<(StreamId, StreamItem)>) -> (Self, StreamHandle) {
        let (handle_tx, rx) = mpsc::channel();
        let handle = StreamHandle::new(handle_tx);
        (Stream { id, tx, rx }, handle)
    }
}

#[derive(Debug)]
pub enum StreamState {
    Idle,
    ReservedRemote,
    Open,
    HalfClosedLocal,
    HalfClosedRemote,
    Closed,
}

#[derive(Debug)]
pub struct StreamHandle {
    tx: mpsc::Sender<StreamItem>,
    state: StreamState,
}
impl StreamHandle {
    fn new(tx: mpsc::Sender<StreamItem>) -> Self {
        StreamHandle {
            tx,
            state: StreamState::Idle,
        }
    }
    pub fn handle_header(&mut self, header: Header) -> Result<()> {
        // TODO: check state
        let _ = self.tx.send(StreamItem::Header(header));
        Ok(())
    }
}

#[derive(Debug)]
pub enum StreamItem {
    Header(Header),
}
