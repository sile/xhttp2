use std::io::Read;
use byteorder::{BigEndian, ByteOrder};
use futures::{Future, Poll, Async};
use handy_async::io::AsyncRead;
use handy_async::io::futures::ReadExact;

use {Result, Error, ErrorKind};
use stream::StreamId;
use super::FrameHeader;

/// https://tools.ietf.org/html/rfc7540#section-6.8
///
/// ```text
///    +-+-------------------------------------------------------------+
///    |R|                  Last-Stream-ID (31)                        |
///    +-+-------------------------------------------------------------+
///    |                      Error Code (32)                          |
///    +---------------------------------------------------------------+
///    |                  Additional Debug Data (*)                    |
///    +---------------------------------------------------------------+
///
///                     Figure 13: GOAWAY Payload Format
/// ```
#[derive(Debug, Clone)]
pub struct GoawayFrame {
    pub last_stream_id: StreamId,
    pub error: Error,
    pub debug_data: Vec<u8>,
}
impl GoawayFrame {
    pub fn read_from<R: Read>(reader: R, header: FrameHeader) -> Result<ReadGoawayFrame<R>> {
        track_assert!(
            header.stream_id.is_connection_control_stream(),
            ErrorKind::ProtocolError
        );
        let bytes = vec![0; header.payload_length as usize];
        Ok(ReadGoawayFrame {
            header,
            future: reader.async_read_exact(bytes),
        })
    }
}

#[derive(Debug)]
pub struct ReadGoawayFrame<R> {
    header: FrameHeader,
    future: ReadExact<R, Vec<u8>>,
}
impl<R> ReadGoawayFrame<R> {
    pub fn reader(&self) -> &R {
        self.future.reader()
    }
    pub fn reader_mut(&mut self) -> &mut R {
        self.future.reader_mut()
    }
}
impl<R: Read> Future for ReadGoawayFrame<R> {
    type Item = (R, GoawayFrame);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((reader, mut bytes)) = track_async_io!(self.future.poll())? {
            let last_stream_id =
                StreamId::new_unchecked(BigEndian::read_u32(&bytes[0..4]) & 0x7FFF_FFFF);
            let error = Error::from_code(BigEndian::read_u32(&bytes[4..8]));
            bytes.drain(0..8);
            let frame = GoawayFrame {
                last_stream_id,
                error,
                debug_data: bytes,
            };
            Ok(Async::Ready((reader, frame)))
        } else {
            Ok(Async::NotReady)
        }
    }
}
