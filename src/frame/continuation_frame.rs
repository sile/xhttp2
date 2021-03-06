use std::io::{Read, Write};
use futures::{Future, Poll};
use handy_async::io::{AsyncRead, AsyncWrite};
use handy_async::io::futures::{ReadExact, WriteAll};

use {Result, ErrorKind, Error};
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
pub struct ContinuationFrame<B> {
    // TODO: private
    pub stream_id: StreamId,
    pub end_headers: bool,
    pub payload: B,
}
impl<B: AsRef<[u8]>> ContinuationFrame<B> {
    pub fn payload_len(&self) -> usize {
        self.payload.as_ref().len()
    }
    pub fn frame_header(&self) -> FrameHeader {
        let flags = if self.end_headers {
            FLAG_END_HEADERS
        } else {
            0
        };

        FrameHeader {
            payload_length: self.payload_len() as u32,
            frame_type: super::FRAME_TYPE_CONTINUATION,
            flags,
            stream_id: self.stream_id,
        }
    }
    pub fn write_into<W: Write>(self, writer: W) -> WriteContinuationFrame<W, B> {
        WriteContinuationFrame(writer.async_write_all(self.payload))
    }
}
impl ContinuationFrame<Vec<u8>> {
    pub fn read_from<R: Read>(reader: R, header: FrameHeader) -> Result<ReadContinuationFrame<R>> {
        track_assert!(
            !header.stream_id.is_connection_control_stream(),
            ErrorKind::ProtocolError
        );
        let payload = vec![0; header.payload_length as usize];
        Ok(ReadContinuationFrame {
            header,
            future: reader.async_read_exact(payload),
        })
    }
}

#[derive(Debug)]
pub struct WriteContinuationFrame<W, B>(WriteAll<W, B>);
impl<W: Write, B: AsRef<[u8]>> Future for WriteContinuationFrame<W, B> {
    type Item = W;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_async_io!(self.0.poll())?.map(|(writer, _)| writer))
    }
}

#[derive(Debug)]
pub struct ReadContinuationFrame<R> {
    header: FrameHeader,
    future: ReadExact<R, Vec<u8>>,
}
impl<R> ReadContinuationFrame<R> {
    pub fn reader(&self) -> &R {
        self.future.reader()
    }
    pub fn reader_mut(&mut self) -> &mut R {
        self.future.reader_mut()
    }
}
impl<R: Read> Future for ReadContinuationFrame<R> {
    type Item = (R, ContinuationFrame<Vec<u8>>);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_async_io!(self.future.poll())?.map(
            |(reader, payload)| {
                let frame = ContinuationFrame {
                    stream_id: self.header.stream_id,
                    end_headers: (self.header.flags & FLAG_END_HEADERS) != 0,
                    payload,
                };
                (reader, frame)
            },
        ))
    }
}
