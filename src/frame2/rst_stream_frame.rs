use std::io::Read;
use byteorder::{ByteOrder, BigEndian};
use futures::{Future, Poll};
use handy_async::io::AsyncRead;
use handy_async::io::futures::ReadExact;

use {Result, Error, ErrorKind};
use stream::StreamId;
use super::FrameHeader;

/// https://tools.ietf.org/html/rfc7540#section-6.4
///
/// ```text
///    +---------------------------------------------------------------+
///    |                        Error Code (32)                        |
///    +---------------------------------------------------------------+
///
///                    Figure 9: RST_STREAM Frame Payload
/// ```
#[derive(Debug)]
pub struct RstStreamFrame {
    pub stream_id: StreamId,
    pub error: Error,
}
impl RstStreamFrame {
    pub fn read_from<R: Read>(reader: R, header: FrameHeader) -> Result<ReadRstStreamFrame<R>> {
        track_assert_eq!(header.payload_length, 4, ErrorKind::FrameSizeError);
        track_assert!(
            !header.stream_id.is_connection_control_stream(),
            ErrorKind::ProtocolError
        );
        Ok(ReadRstStreamFrame {
            header,
            future: reader.async_read_exact([0; 4]),
        })
    }
}

#[derive(Debug)]
pub struct ReadRstStreamFrame<R> {
    header: FrameHeader,
    future: ReadExact<R, [u8; 4]>,
}
impl<R> ReadRstStreamFrame<R> {
    pub fn reader(&self) -> &R {
        self.future.reader()
    }
    pub fn reader_mut(&mut self) -> &mut R {
        self.future.reader_mut()
    }
}
impl<R: Read> Future for ReadRstStreamFrame<R> {
    type Item = (R, RstStreamFrame);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_async_io!(self.future.poll())?.map(
            |(reader, bytes)| {
                let code = BigEndian::read_u32(&bytes[..]);
                let frame = RstStreamFrame {
                    stream_id: self.header.stream_id,
                    error: Error::from_code(code),
                };
                (reader, frame)
            },
        ))
    }
}
