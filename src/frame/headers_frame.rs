use std::fmt;
use std::io::{Read, Write};
use futures::{Future, Poll, Async};
use handy_async::future::Phase;
use handy_async::io::{AsyncRead, WriteInto};
use handy_async::io::futures::{ReadExact, WritePattern};
use handy_async::pattern::Buf;

use {Result, Error, ErrorKind};
use priority::{Priority, ReadPriority};
use stream::StreamId;
use super::FrameHeader;

const FLAG_END_STREAM: u8 = 0x1;
const FLAG_END_HEADERS: u8 = 0x4;
const FLAG_PADDED: u8 = 0x8;
const FLAG_PRIORITY: u8 = 0x20;

/// https://tools.ietf.org/html/rfc7540#section-6.2
///
/// ```text
///    +---------------+
///    |Pad Length? (8)|
///    +-+-------------+-----------------------------------------------+
///    |E|                 Stream Dependency? (31)                     |
///    +-+-------------+-----------------------------------------------+
///    |  Weight? (8)  |
///    +-+-------------+-----------------------------------------------+
///    |                   Header Block Fragment (*)                 ...
///    +---------------------------------------------------------------+
///    |                           Padding (*)                       ...
///    +---------------------------------------------------------------+
///
///                      Figure 7: HEADERS Frame Payload
/// ```
#[derive(Debug)]
pub struct HeadersFrame<B> {
    // TODO: private
    pub stream_id: StreamId,
    pub end_stream: bool,
    pub end_headers: bool,
    pub priority: Option<Priority>,
    pub padding_len: Option<u8>,
    pub fragment: B,
}
impl<B: AsRef<[u8]>> HeadersFrame<B> {
    pub fn payload_len(&self) -> usize {
        self.fragment.as_ref().len() + self.padding_len.map_or(0, |x| x as usize + 1) +
            self.priority.as_ref().map_or(0, |_| 5)
    }
    pub fn frame_header(&self) -> FrameHeader {
        let mut flags = 0;
        if self.end_stream {
            flags |= FLAG_END_STREAM;
        }
        if self.end_headers {
            flags |= FLAG_END_HEADERS;
        }
        if self.padding_len.is_some() {
            flags |= FLAG_PADDED;
        }
        if self.priority.is_some() {
            flags |= FLAG_PRIORITY;
        }

        FrameHeader {
            payload_length: self.payload_len() as u32,
            frame_type: super::FRAME_TYPE_HEADERS,
            flags,
            stream_id: self.stream_id,
        }
    }
    pub fn write_into<W: Write>(self, writer: W) -> WriteHeadersFrame<W, B> {
        let padding: &'static [u8] = &[0; 255][0..self.padding_len.map_or(0, |x| x as usize)];
        let pattern = (
            self.padding_len,
            self.priority.map(|x| Buf(x.to_bytes())),
            Buf(self.fragment),
            Buf(padding),
        );
        WriteHeadersFrame { future: pattern.write_into(writer) }
    }
}
impl HeadersFrame<Vec<u8>> {
    pub fn read_from<R: Read>(reader: R, header: FrameHeader) -> Result<ReadHeadersFrame<R>> {
        track_assert!(
            !header.stream_id.is_connection_control_stream(),
            ErrorKind::ProtocolError
        );
        let phase = if (header.flags & FLAG_PADDED) != 0 {
            Phase::A(reader.async_read_exact([0; 1]))
        } else if (header.flags & FLAG_PRIORITY) != 0 {
            Phase::B(Priority::read_from(reader))
        } else {
            let fragment = vec![0; header.payload_length as usize];
            Phase::C(reader.async_read_exact(fragment))
        };
        Ok(ReadHeadersFrame {
            header,
            read_bytes: 0,
            padding_len: None,
            priority: None,
            phase,
        })
    }
}

pub struct WriteHeadersFrame<W: Write, B: AsRef<[u8]>> {
    future: WritePattern<(Option<u8>, Option<Buf<[u8; 5]>>, Buf<B>, Buf<&'static [u8]>), W>,
}
impl<W: Write, B: AsRef<[u8]>> Future for WriteHeadersFrame<W, B> {
    type Item = W;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_async_io!(self.future.poll())?.map(
            |(writer, _)| writer,
        ))
    }
}
impl<W: Write, B: AsRef<[u8]>> fmt::Debug for WriteHeadersFrame<W, B> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "WriteHeadersFrame(_)")
    }
}

#[derive(Debug)]
pub struct ReadHeadersFrame<R> {
    header: FrameHeader,
    read_bytes: usize,
    padding_len: Option<u8>,
    priority: Option<Priority>,
    phase: Phase<ReadExact<R, [u8; 1]>, ReadPriority<R>, ReadExact<R, Vec<u8>>>,
}
impl<R> ReadHeadersFrame<R> {
    pub fn reader(&self) -> &R {
        match self.phase {
            Phase::A(ref f) => f.reader(),
            Phase::B(ref f) => f.reader(),
            Phase::C(ref f) => f.reader(),
            _ => unreachable!(),
        }
    }
    pub fn reader_mut(&mut self) -> &mut R {
        match self.phase {
            Phase::A(ref mut f) => f.reader_mut(),
            Phase::B(ref mut f) => f.reader_mut(),
            Phase::C(ref mut f) => f.reader_mut(),
            _ => unreachable!(),
        }
    }
}
impl<R: Read> Future for ReadHeadersFrame<R> {
    type Item = (R, HeadersFrame<Vec<u8>>);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Async::Ready(phase) = track_async_io!(self.phase.poll())? {
            let next = match phase {
                Phase::A((reader, bytes)) => {
                    self.read_bytes += bytes.len();
                    self.padding_len = Some(bytes[0]);
                    if (self.header.flags & FLAG_PRIORITY) != 0 {
                        Phase::B(Priority::read_from(reader))
                    } else {
                        let buf = vec![0; self.header.payload_length as usize - self.read_bytes];
                        Phase::C(reader.async_read_exact(buf))
                    }
                }
                Phase::B((reader, priority)) => {
                    self.read_bytes += 5;
                    self.priority = Some(priority);

                    let buf = vec![0; self.header.payload_length as usize - self.read_bytes];
                    Phase::C(reader.async_read_exact(buf))
                }
                Phase::C((reader, mut buf)) => {
                    if let Some(padding_len) = self.padding_len {
                        track_assert!(buf.len() >= padding_len as usize, ErrorKind::ProtocolError);
                        let fragment_len = buf.len() - padding_len as usize;
                        buf.truncate(fragment_len);
                    }
                    let frame = HeadersFrame {
                        stream_id: self.header.stream_id,
                        end_stream: (self.header.flags & FLAG_END_STREAM) != 0,
                        end_headers: (self.header.flags & FLAG_END_HEADERS) != 0,
                        priority: self.priority.clone(),
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
