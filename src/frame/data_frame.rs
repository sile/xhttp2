use std::io::Read;
use futures::{Future, Poll, Async};
use handy_async::future::Phase;
use handy_async::io::AsyncRead;
use handy_async::io::futures::ReadExact;

use {Result, Error, ErrorKind};
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
pub struct DataFrame<B> {
    // TODO: private
    pub stream_id: StreamId,
    pub end_stream: bool,
    pub padding_len: Option<u8>,
    pub data: B,
}
impl<B: AsRef<[u8]>> DataFrame<B> {
    pub fn payload_len(&self) -> usize {
        self.data.as_ref().len() + self.padding_len.map_or(0, |x| x as usize + 1)
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
            let data = vec![0; header.payload_length as usize];
            Phase::B(reader.async_read_exact(data))
        };
        Ok(ReadDataFrame {
            header,
            padding_len: None,
            phase,
        })
    }
}

#[derive(Debug)]
pub struct ReadDataFrame<R> {
    header: FrameHeader,
    padding_len: Option<u8>,
    phase: Phase<ReadExact<R, [u8; 1]>, ReadExact<R, Vec<u8>>>,
}
impl<R> ReadDataFrame<R> {
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
impl<R: Read> Future for ReadDataFrame<R> {
    type Item = (R, DataFrame<Vec<u8>>);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Async::Ready(phase) = track_async_io!(self.phase.poll())? {
            let next = match phase {
                Phase::A((reader, bytes)) => {
                    self.padding_len = Some(bytes[0]);
                    let data_and_padding = vec![0; self.header.payload_length as usize - 1];
                    Phase::B(reader.async_read_exact(data_and_padding))
                }
                Phase::B((reader, mut buf)) => {
                    if let Some(padding_len) = self.padding_len {
                        track_assert!(buf.len() >= padding_len as usize, ErrorKind::ProtocolError);
                        let data_len = buf.len() - padding_len as usize;
                        buf.truncate(data_len);
                    }
                    let frame = DataFrame {
                        stream_id: self.header.stream_id,
                        end_stream: (self.header.flags & FLAG_END_STREAM) != 0,
                        padding_len: self.padding_len,
                        data: buf,
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
