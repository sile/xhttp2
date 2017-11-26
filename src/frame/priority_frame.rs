use std::io::{Read, Write};
use futures::{Future, Poll};

use {Result, Error, ErrorKind};
use priority::{Priority, ReadPriority, WritePriority};
use stream::StreamId;
use super::FrameHeader;

/// https://tools.ietf.org/html/rfc7540#section-6.3
///
/// ```text
///    +-+-------------------------------------------------------------+
///    |E|                  Stream Dependency (31)                     |
///    +-+-------------+-----------------------------------------------+
///    |   Weight (8)  |
///    +-+-------------+
///
///                     Figure 8: PRIORITY Frame Payload
/// ```
#[derive(Debug)]
pub struct PriorityFrame {
    pub stream_id: StreamId,
    pub priority: Priority,
}
impl PriorityFrame {
    pub fn payload_len(&self) -> usize {
        5
    }
    pub fn frame_header(&self) -> FrameHeader {
        FrameHeader {
            payload_length: self.payload_len() as u32,
            frame_type: super::FRAME_TYPE_PRIORITY,
            flags: 0,
            stream_id: self.stream_id,
        }
    }
    pub fn read_from<R: Read>(reader: R, header: FrameHeader) -> Result<ReadPriorityFrame<R>> {
        track_assert_eq!(header.payload_length, 5, ErrorKind::FrameSizeError);
        track_assert!(
            !header.stream_id.is_connection_control_stream(),
            ErrorKind::ProtocolError
        );
        Ok(ReadPriorityFrame {
            header,
            future: Priority::read_from(reader),
        })
    }
    pub fn write_into<W: Write>(self, writer: W) -> WritePriorityFrame<W> {
        WritePriorityFrame(self.priority.write_into(writer))
    }
}

#[derive(Debug)]
pub struct WritePriorityFrame<W>(WritePriority<W>);
impl<W: Write> Future for WritePriorityFrame<W> {
    type Item = W;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        track!(self.0.poll())
    }
}

#[derive(Debug)]
pub struct ReadPriorityFrame<R> {
    header: FrameHeader,
    future: ReadPriority<R>,
}
impl<R> ReadPriorityFrame<R> {
    pub fn reader(&self) -> &R {
        self.future.reader()
    }
    pub fn reader_mut(&mut self) -> &mut R {
        self.future.reader_mut()
    }
}
impl<R: Read> Future for ReadPriorityFrame<R> {
    type Item = (R, PriorityFrame);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_async_io!(self.future.poll())?.map(
            |(reader, priority)| {
                let frame = PriorityFrame {
                    stream_id: self.header.stream_id,
                    priority,
                };
                (reader, frame)
            },
        ))
    }
}
