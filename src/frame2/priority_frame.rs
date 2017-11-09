use std::io::Read;
use futures::{Future, Poll};
use handy_async::io::AsyncRead;
use handy_async::io::futures::ReadExact;

use {Result, Error, ErrorKind};
use priority::Priority;
use stream::StreamId;
use super::FrameHeader;

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
#[derive(Debug)]
pub struct PriorityFrame {
    pub stream_id: StreamId,
    pub priority: Priority,
}
impl PriorityFrame {
    pub fn read_from<R: Read>(reader: R, header: FrameHeader) -> Result<ReadPriorityFrame<R>> {
        track_assert_eq!(header.payload_length, 5, ErrorKind::FrameSizeError);
        track_assert!(
            !header.stream_id.is_connection_control_stream(),
            ErrorKind::ProtocolError
        );
        Ok(ReadPriorityFrame {
            header,
            future: reader.async_read_exact([0; 5]),
        })
    }
}

#[derive(Debug)]
pub struct ReadPriorityFrame<R> {
    header: FrameHeader,
    future: ReadExact<R, [u8; 5]>,
}
impl<R> ReadPriorityFrame<R> {
    pub fn reader(&self) -> &R {
        self.future.reader()
    }
    pub fn reader_mut(&mut self) -> &mut R {
        self.future.reader_mut()
    }
}
impl<R: Read> Future for ReadPriorityFrame<R> {
    type Item = (R, PriorityFrame);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_async_io!(self.future.poll())?.map(
            |(reader, bytes)| {
                let frame = PriorityFrame {
                    stream_id: self.header.stream_id,
                    priority: Priority::from_bytes(bytes),
                };
                (reader, frame)
            },
        ))
    }
}
