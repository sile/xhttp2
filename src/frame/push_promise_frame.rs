use std::fmt;
use std::io::{Read, Write};
use futures::{Future, Poll, Async};
use handy_async::future::Phase;
use handy_async::io::{AsyncRead, WriteInto};
use handy_async::io::futures::{ReadExact, WritePattern};
use handy_async::pattern::{Endian, Buf};
use handy_async::pattern::combinators::BE;

use {Result, Error, ErrorKind};
use stream::{StreamId, ReadStreamId};
use super::FrameHeader;

const FLAG_END_HEADERS: u8 = 0x4;
const FLAG_PADDED: u8 = 0x8;

/// https://tools.ietf.org/html/rfc7540#section-6.2
///
/// ```text
///    +---------------+
///    |Pad Length? (8)|
///    +-+-------------+-----------------------------------------------+
///    |R|                  Promised Stream ID (31)                    |
///    +-+-----------------------------+-------------------------------+
///    |                   Header Block Fragment (*)                 ...
///    +---------------------------------------------------------------+
///    |                           Padding (*)                       ...
///    +---------------------------------------------------------------+
///
///                  Figure 11: PUSH_PROMISE Payload Format
/// ```
#[derive(Debug)]
pub struct PushPromiseFrame<B> {
    // TODO: private
    pub stream_id: StreamId,
    pub promise_stream_id: StreamId,
    pub end_headers: bool,
    pub padding_len: Option<u8>,
    pub fragment: B,
}
impl<B: AsRef<[u8]>> PushPromiseFrame<B> {
    pub fn payload_len(&self) -> usize {
        4 + self.fragment.as_ref().len() + self.padding_len.map_or(0, |x| x as usize + 1)
    }
    pub fn frame_header(&self) -> FrameHeader {
        let mut flags = 0;
        if self.end_headers {
            flags |= FLAG_END_HEADERS;
        }
        if self.padding_len.is_some() {
            flags |= FLAG_PADDED;
        }

        FrameHeader {
            payload_length: self.payload_len() as u32,
            frame_type: super::FRAME_TYPE_PUSH_PROMISE,
            flags,
            stream_id: self.stream_id,
        }
    }
    pub fn write_into<W: Write>(self, writer: W) -> WritePushPromiseFrame<W, B> {
        let padding: &'static [u8] = &[0; 255][0..self.padding_len.map_or(0, |x| x as usize)];
        let pattern = (
            self.padding_len,
            self.promise_stream_id.as_u32().be(),
            Buf(self.fragment),
            Buf(padding),
        );
        WritePushPromiseFrame { future: pattern.write_into(writer) }
    }
}
impl PushPromiseFrame<Vec<u8>> {
    pub fn read_from<R: Read>(reader: R, header: FrameHeader) -> Result<ReadPushPromiseFrame<R>> {
        track_assert!(
            !header.stream_id.is_connection_control_stream(),
            ErrorKind::ProtocolError
        );
        let phase = if (header.flags & FLAG_PADDED) != 0 {
            Phase::A(reader.async_read_exact([0; 1]))
        } else {
            Phase::B(StreamId::read_from(reader))
        };
        Ok(ReadPushPromiseFrame {
            header,
            read_bytes: 0,
            promise_stream_id: StreamId::from(0u8),
            padding_len: None,
            phase,
        })
    }
}

pub struct WritePushPromiseFrame<W: Write, B: AsRef<[u8]>> {
    future: WritePattern<(Option<u8>, BE<u32>, Buf<B>, Buf<&'static [u8]>), W>,
}
impl<W: Write, B: AsRef<[u8]>> Future for WritePushPromiseFrame<W, B> {
    type Item = W;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_async_io!(self.future.poll())?.map(
            |(writer, _)| writer,
        ))
    }
}
impl<W: Write, B: AsRef<[u8]>> fmt::Debug for WritePushPromiseFrame<W, B> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "WritePushPromiseFrame(_)")
    }
}

#[derive(Debug)]
pub struct ReadPushPromiseFrame<R> {
    header: FrameHeader,
    read_bytes: usize,
    promise_stream_id: StreamId,
    padding_len: Option<u8>,
    phase: Phase<ReadExact<R, [u8; 1]>, ReadStreamId<R>, ReadExact<R, Vec<u8>>>,
}
impl<R> ReadPushPromiseFrame<R> {
    pub fn reader(&self) -> &R {
        match self.phase {
            Phase::A(ref f) => f.reader(),
            Phase::B(ref f) => f.reader(),
            _ => unreachable!(),
        }
    }
    pub fn reader_mut(&mut self) -> &mut R {
        match self.phase {
            Phase::A(ref mut f) => f.reader_mut(),
            Phase::B(ref mut f) => f.reader_mut(),
            _ => unreachable!(),
        }
    }
}
impl<R: Read> Future for ReadPushPromiseFrame<R> {
    type Item = (R, PushPromiseFrame<Vec<u8>>);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Async::Ready(phase) = track_async_io!(self.phase.poll())? {
            let next = match phase {
                Phase::A((reader, bytes)) => {
                    self.read_bytes += 1;
                    self.padding_len = Some(bytes[0]);
                    Phase::B(StreamId::read_from(reader))
                }
                Phase::B((reader, promise_stream_id)) => {
                    self.read_bytes += 4;
                    self.promise_stream_id = promise_stream_id;

                    let buf = vec![0; self.header.payload_length as usize - self.read_bytes];
                    Phase::C(reader.async_read_exact(buf))
                }
                Phase::C((reader, mut buf)) => {
                    if let Some(padding_len) = self.padding_len {
                        track_assert!(buf.len() >= padding_len as usize, ErrorKind::ProtocolError);
                        let fragment_len = buf.len() - padding_len as usize;
                        buf.truncate(fragment_len);
                    }
                    let frame = PushPromiseFrame {
                        stream_id: self.header.stream_id,
                        promise_stream_id: self.promise_stream_id,
                        end_headers: (self.header.flags & FLAG_END_HEADERS) != 0,
                        padding_len: self.padding_len,
                        fragment: buf,
                    };
                    return Ok(Async::Ready((reader, frame)));
                }
                _ => unreachable!(),
            };
            self.phase = next;
        }
        Ok(Async::NotReady)
    }
}
