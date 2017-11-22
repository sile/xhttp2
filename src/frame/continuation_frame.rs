use std::io::{self, Read};
use futures::{Future, Poll};

use {Result, ErrorKind, Error};
use aux_futures::Finished;
use stream::StreamId;
use super::FrameHeader;

const FLAG_END_HEADERS: u8 = 0x4;

/// https://tools.ietf.org/html/rfc7540#section-6.10
///
/// ```text
///    +---------------------------------------------------------------+
///    |                   Header Block Fragment (*)                 ...
///    +---------------------------------------------------------------+
///
///                   Figure 15: CONTINUATION Frame Payload
/// ```
#[derive(Debug)]
pub struct ContinuationFrame<T> {
    io: T,

    // TODO: private
    pub stream_id: StreamId,
    pub end_headers: bool,

    payload_len: usize,
    fragment_len: usize,
}
impl<T> ContinuationFrame<T> {
    pub fn payload_len(&self) -> usize {
        self.payload_len
    }
    pub fn into_io(self) -> T {
        self.io
    }
}
impl<R: Read> ContinuationFrame<R> {
    pub fn read_from(reader: R, header: FrameHeader) -> Result<ReadContinuationFrame<R>> {
        track_assert!(
            !header.stream_id.is_connection_control_stream(),
            ErrorKind::ProtocolError
        );
        let frame = ContinuationFrame {
            io: reader,
            stream_id: header.stream_id,
            end_headers: (header.flags & FLAG_END_HEADERS) != 0,
            fragment_len: header.payload_length as usize,
            payload_len: header.payload_length as usize,
        };
        Ok(ReadContinuationFrame(Finished::new(frame)))
    }
}
impl<R: Read> Read for ContinuationFrame<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        use std::cmp;
        let len = cmp::min(self.fragment_len, buf.len());
        let read_size = self.io.read(&mut buf[0..len])?;

        self.fragment_len -= read_size;
        Ok(read_size)
    }
}

#[derive(Debug)]
pub struct ReadContinuationFrame<R>(Finished<ContinuationFrame<R>>);
impl<R> ReadContinuationFrame<R> {
    pub fn reader(&self) -> &R {
        &self.0.item().io
    }
    pub fn reader_mut(&mut self) -> &mut R {
        &mut self.0.item_mut().io
    }
}
impl<R> Future for ReadContinuationFrame<R> {
    type Item = ContinuationFrame<R>;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.0.poll()
    }
}
