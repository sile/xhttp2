use std::io::{self, Read};
use futures::{Future, Poll, Async};
use handy_async::future::Phase;
use handy_async::io::AsyncRead;
use handy_async::io::futures::ReadExact;

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
pub struct PushPromiseFrame<T> {
    io: T,

    // TODO: private
    pub stream_id: StreamId,
    pub promise_stream_id: StreamId,
    pub end_headers: bool,
    pub padding_len: Option<u8>,

    payload_len: usize,
    fragment_len: usize,
    rest_padding_len: Option<u8>,
}
impl<T> PushPromiseFrame<T> {
    pub fn payload_len(&self) -> usize {
        self.payload_len
    }
    pub fn into_io(self) -> T {
        self.io
    }
}
impl<R: Read> PushPromiseFrame<R> {
    pub fn read_from(reader: R, header: FrameHeader) -> Result<ReadPushPromiseFrame<R>> {
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
            padding_len: None,
            phase,
        })
    }
}
impl<R: Read> Read for PushPromiseFrame<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        use std::cmp;
        if self.fragment_len == 0 {
            while let Some(ref mut padding_len) = self.rest_padding_len {
                if *padding_len == 0 {
                    break;
                }
                let mut buf = [0; 0xFF];
                let len = cmp::min(0xFF, *padding_len);
                let read_size = self.io.read(&mut buf[0..len as usize])?;
                *padding_len -= read_size as u8;
            }
            Ok(0)
        } else {
            let len = cmp::min(self.fragment_len, buf.len());
            let read_size = self.io.read(&mut buf[0..len])?;

            self.fragment_len -= read_size;
            Ok(read_size)
        }
    }
}

#[derive(Debug)]
pub struct ReadPushPromiseFrame<R> {
    header: FrameHeader,
    read_bytes: usize,
    padding_len: Option<u8>,
    phase: Phase<ReadExact<R, [u8; 1]>, ReadStreamId<R>>,
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
    type Item = PushPromiseFrame<R>;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Async::Ready(phase) = track_async_io!(self.phase.poll())? {
            let next = match phase {
                Phase::A((reader, bytes)) => {
                    self.read_bytes += bytes.len();
                    self.padding_len = Some(bytes[0]);
                    Phase::B(StreamId::read_from(reader))
                }
                Phase::B((reader, promise_stream_id)) => {
                    let mut fragment_len = self.header.payload_length as usize - self.read_bytes;
                    if let Some(padding_len) = self.padding_len {
                        track_assert!(
                            padding_len as usize <= fragment_len,
                            ErrorKind::ProtocolError
                        );
                        fragment_len -= padding_len as usize;
                    }
                    let frame = PushPromiseFrame {
                        io: reader,
                        stream_id: self.header.stream_id,
                        promise_stream_id,
                        end_headers: (self.header.flags & FLAG_END_HEADERS) != 0,
                        padding_len: self.padding_len,
                        rest_padding_len: self.padding_len,
                        fragment_len,
                        payload_len: self.header.payload_length as usize,
                    };
                    return Ok(Async::Ready(frame));
                }
                _ => unreachable!(),
            };
            self.phase = next;
        }
        Ok(Async::NotReady)
    }
}
