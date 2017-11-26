use std::io::{Read, Write};
use byteorder::{ByteOrder, BigEndian};
use futures::{Future, Poll};
use handy_async::io::{AsyncRead, AsyncWrite};
use handy_async::io::futures::{ReadExact, WriteAll};

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
    pub fn payload_len(&self) -> usize {
        4
    }
    pub fn frame_header(&self) -> FrameHeader {
        FrameHeader {
            payload_length: self.payload_len() as u32,
            frame_type: super::FRAME_TYPE_RST_STREAM,
            flags: 0,
            stream_id: self.stream_id,
        }
    }
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
    pub fn write_into<W: Write>(self, writer: W) -> WriteRstStreamFrame<W> {
        let mut buf = [0; 4];
        BigEndian::write_u32(&mut buf[..], self.error.as_code());
        WriteRstStreamFrame(writer.async_write_all(buf))
    }
}

#[derive(Debug)]
pub struct WriteRstStreamFrame<W>(WriteAll<W, [u8; 4]>);
impl<W: Write> Future for WriteRstStreamFrame<W> {
    type Item = W;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_async_io!(self.0.poll())?.map(|(writer, _)| writer))
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
