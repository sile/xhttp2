use std::io::{Read, Write};
use byteorder::{BigEndian, ByteOrder};
use futures::{Future, Poll, Async};
use handy_async::io::{AsyncRead, AsyncWrite};
use handy_async::io::futures::{ReadExact, WriteAll};

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
    pub fn payload_len(&self) -> usize {
        4
    }
    pub fn frame_header(&self) -> FrameHeader {
        FrameHeader {
            payload_length: self.payload_len() as u32,
            frame_type: super::FRAME_TYPE_WINDOW_UPDATE,
            flags: 0,
            stream_id: self.stream_id,
        }
    }
    pub fn write_into<W: Write>(self, writer: W) -> WriteWindowUpdateFrame<W> {
        let mut buf = [0; 4];
        BigEndian::write_u32(&mut buf[..], self.window_size_increment);
        WriteWindowUpdateFrame(writer.async_write_all(buf))
    }
    pub fn read_from<R: Read>(reader: R, header: FrameHeader) -> Result<ReadWindowUpdateFrame<R>> {
        track_assert_eq!(header.payload_length, 4, ErrorKind::FrameSizeError);
        Ok(ReadWindowUpdateFrame {
            header,
            future: reader.async_read_exact([0; 4]),
        })
    }
}

#[derive(Debug)]
pub struct WriteWindowUpdateFrame<W>(WriteAll<W, [u8; 4]>);
impl<W: Write> Future for WriteWindowUpdateFrame<W> {
    type Item = W;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_async_io!(self.0.poll())?.map(|(writer, _)| writer))
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
