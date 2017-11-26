use std::fmt;
use std::io::{Read, Write};
use byteorder::{BigEndian, ByteOrder};
use futures::{Future, Poll, Async};
use handy_async::io::{AsyncRead, WriteInto};
use handy_async::io::futures::{ReadExact, WritePattern};
use handy_async::pattern::Endian;
use handy_async::pattern::combinators::BE;

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
    pub fn payload_len(&self) -> usize {
        4 + 4 + self.debug_data.len()
    }
    pub fn frame_header(&self) -> FrameHeader {
        FrameHeader {
            payload_length: self.payload_len() as u32,
            frame_type: super::FRAME_TYPE_GOAWAY,
            flags: 0,
            stream_id: StreamId::connection_control_stream_id(),
        }
    }
    pub fn write_into<W: Write>(self, writer: W) -> WriteGoawayFrame<W> {
        let pattern = (
            self.last_stream_id.as_u32().be(),
            self.error.as_code().be(),
            self.debug_data,
        );
        WriteGoawayFrame(pattern.write_into(writer))
    }
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

pub struct WriteGoawayFrame<W: Write>(WritePattern<(BE<u32>, BE<u32>, Vec<u8>), W>);
impl<W: Write> Future for WriteGoawayFrame<W> {
    type Item = W;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_async_io!(self.0.poll())?.map(|(writer, _)| writer))
    }
}
impl<W: Write> fmt::Debug for WriteGoawayFrame<W> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "WriteGoawayFrame(_)")
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
