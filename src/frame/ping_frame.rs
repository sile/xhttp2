use std::io::{Read, Write};
use futures::{Future, Poll};
use handy_async::io::{AsyncRead, AsyncWrite};
use handy_async::io::futures::{ReadExact, WriteAll};

use {Result, Error, ErrorKind};
use stream::StreamId;
use super::FrameHeader;

const FLAG_ACK: u8 = 0x01;

/// https://tools.ietf.org/html/rfc7540#section-6.7
///
/// ```text
///    +---------------------------------------------------------------+
///    |                                                               |
///    |                      Opaque Data (64)                         |
///    |                                                               |
///    +---------------------------------------------------------------+
///
///                      Figure 12: PING Payload Format
/// ```
#[derive(Debug, Clone)]
pub struct PingFrame {
    pub ack: bool,
    pub data: [u8; 8],
}
impl PingFrame {
    pub fn payload_len(&self) -> usize {
        8
    }
    pub fn frame_header(&self) -> FrameHeader {
        FrameHeader {
            payload_length: self.payload_len() as u32,
            frame_type: super::FRAME_TYPE_PING,
            flags: 0,
            stream_id: StreamId::connection_control_stream_id(),
        }
    }
    pub fn write_into<W: Write>(self, writer: W) -> WritePingFrame<W> {
        WritePingFrame(writer.async_write_all(self.data))
    }
    pub fn read_from<R: Read>(reader: R, header: FrameHeader) -> Result<ReadPingFrame<R>> {
        track_assert_eq!(header.payload_length, 8, ErrorKind::FrameSizeError);
        track_assert!(
            header.stream_id.is_connection_control_stream(),
            ErrorKind::ProtocolError
        );
        Ok(ReadPingFrame {
            header,
            future: reader.async_read_exact([0; 8]),
        })
    }
}

#[derive(Debug)]
pub struct WritePingFrame<W>(WriteAll<W, [u8; 8]>);
impl<W: Write> Future for WritePingFrame<W> {
    type Item = W;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_async_io!(self.0.poll())?.map(|(writer, _)| writer))
    }
}

#[derive(Debug)]
pub struct ReadPingFrame<R> {
    header: FrameHeader,
    future: ReadExact<R, [u8; 8]>,
}
impl<R> ReadPingFrame<R> {
    pub fn reader(&self) -> &R {
        self.future.reader()
    }
    pub fn reader_mut(&mut self) -> &mut R {
        self.future.reader_mut()
    }
}
impl<R: Read> Future for ReadPingFrame<R> {
    type Item = (R, PingFrame);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_async_io!(self.future.poll())?.map(
            |(reader, bytes)| {
                let frame = PingFrame {
                    ack: self.header.flags & FLAG_ACK != 0,
                    data: bytes,
                };
                (reader, frame)
            },
        ))
    }
}
