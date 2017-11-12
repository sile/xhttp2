use std::io::Read;
use byteorder::{BigEndian, ByteOrder};
use futures::{Future, Poll, Async};
use handy_async::io::AsyncRead;
use handy_async::io::futures::ReadExact;

use {Result, Error, ErrorKind};
use stream::StreamId;
use super::FrameHeader;

/// https://tools.ietf.org/html/rfc7540#section-6.9
///
/// ```text
///    +-+-------------------------------------------------------------+
///    |R|              Window Size Increment (31)                     |
///    +-+-------------------------------------------------------------+
///
///                  Figure 14: WINDOW_UPDATE Payload Format
/// ```
#[derive(Debug, Clone)]
pub struct WindowUpdateFrame {
    pub stream_id: StreamId,
    pub window_size_increment: u32, // TODO: private
}
impl WindowUpdateFrame {
    pub fn read_from<R: Read>(reader: R, header: FrameHeader) -> Result<ReadWindowUpdateFrame<R>> {
        track_assert_eq!(header.payload_length, 4, ErrorKind::FrameSizeError);
        Ok(ReadWindowUpdateFrame {
            header,
            future: reader.async_read_exact([0; 4]),
        })
    }
}

#[derive(Debug)]
pub struct ReadWindowUpdateFrame<R> {
    header: FrameHeader,
    future: ReadExact<R, [u8; 4]>,
}
impl<R> ReadWindowUpdateFrame<R> {
    pub fn reader(&self) -> &R {
        self.future.reader()
    }
    pub fn reader_mut(&mut self) -> &mut R {
        self.future.reader_mut()
    }
}
impl<R: Read> Future for ReadWindowUpdateFrame<R> {
    type Item = (R, WindowUpdateFrame);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((reader, bytes)) = track_async_io!(self.future.poll())? {
            let window_size_increment = BigEndian::read_u32(&bytes[..]) & 0x7FFF_FFFF;
            track_assert_ne!(window_size_increment, 0, ErrorKind::ProtocolError);

            let frame = WindowUpdateFrame {
                stream_id: self.header.stream_id,
                window_size_increment,
            };
            Ok(Async::Ready((reader, frame)))
        } else {
            Ok(Async::NotReady)
        }
    }
}
