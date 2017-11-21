use std::io::{self, Read};
use futures::{Future, Poll, Async};
use handy_async::future::Phase;
use handy_async::io::AsyncRead;
use handy_async::io::futures::ReadExact;

use {Result, Error, ErrorKind};
use aux_futures::Finished;
use stream::StreamId;
use super::FrameHeader;

const FLAG_END_STREAM: u8 = 0x1;
const FLAG_PADDED: u8 = 0x8;

/// https://tools.ietf.org/html/rfc7540#section-6.1
///
/// ```text
///    +---------------+
///    |Pad Length? (8)|
///    +---------------+-----------------------------------------------+
///    |                            Data (*)                         ...
///    +---------------------------------------------------------------+
///    |                           Padding (*)                       ...
///    +---------------------------------------------------------------+
///
///                       Figure 6: DATA Frame Payload
/// ```
#[derive(Debug)]
pub struct DataFrame<T> {
    io: T,

    // TODO: private
    pub stream_id: StreamId,
    pub end_stream: bool,
    pub padding_len: Option<u8>,

    fragment_len: usize,
    rest_padding_len: Option<u8>,
}
impl<T> DataFrame<T> {
    pub fn into_io(self) -> T {
        self.io
    }
}
impl<R: Read> DataFrame<R> {
    pub fn read_from(reader: R, header: FrameHeader) -> Result<ReadDataFrame<R>> {
        track_assert!(
            !header.stream_id.is_connection_control_stream(),
            ErrorKind::ProtocolError
        );
        let phase = if (header.flags & FLAG_PADDED) != 0 {
            Phase::A(reader.async_read_exact([0; 1]))
        } else {
            Phase::B(Finished::new(reader))
        };
        Ok(ReadDataFrame {
            header,
            read_bytes: 0,
            padding_len: None,
            phase,
        })
    }
}
impl<R: Read> Read for DataFrame<R> {
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
pub struct ReadDataFrame<R> {
    header: FrameHeader,
    read_bytes: usize,
    padding_len: Option<u8>,
    phase: Phase<ReadExact<R, [u8; 1]>, Finished<R>>,
}
impl<R> ReadDataFrame<R> {
    pub fn reader(&self) -> &R {
        match self.phase {
            Phase::A(ref f) => f.reader(),
            Phase::B(ref f) => f.item(),
            _ => unreachable!(),
        }
    }
    pub fn reader_mut(&mut self) -> &mut R {
        match self.phase {
            Phase::A(ref mut f) => f.reader_mut(),
            Phase::B(ref mut f) => f.item_mut(),
            _ => unreachable!(),
        }
    }
}
impl<R: Read> Future for ReadDataFrame<R> {
    type Item = DataFrame<R>;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Async::Ready(phase) = track_async_io!(self.phase.poll())? {
            let next = match phase {
                Phase::A((reader, bytes)) => {
                    self.read_bytes += bytes.len();
                    self.padding_len = Some(bytes[0]);
                    Phase::B(Finished::new(reader))
                }
                Phase::B(reader) => {
                    let mut fragment_len = self.header.payload_length as usize - self.read_bytes;
                    if let Some(padding_len) = self.padding_len {
                        track_assert!(
                            padding_len as usize <= fragment_len,
                            ErrorKind::ProtocolError
                        );
                        fragment_len -= padding_len as usize;
                    }
                    let frame = DataFrame {
                        io: reader,
                        stream_id: self.header.stream_id,
                        end_stream: (self.header.flags & FLAG_END_STREAM) != 0,
                        padding_len: self.padding_len,
                        rest_padding_len: self.padding_len,
                        fragment_len,
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
