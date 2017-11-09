use std::io::Read;
use futures::{Future, Poll, Async};
use handy_async::future::Phase;

pub use self::priority_frame::PriorityFrame;
pub use self::rst_stream_frame::RstStreamFrame;

use Error;
use self::frame_header::{FrameHeader, ReadFrameHeader};
use self::priority_frame::ReadPriorityFrame;
use self::rst_stream_frame::ReadRstStreamFrame;

mod frame_header;
mod priority_frame;
mod rst_stream_frame;

const FRAME_TYPE_DATA: u8 = 0x0;
const FRAME_TYPE_HEADERS: u8 = 0x1;
const FRAME_TYPE_PRIORITY: u8 = 0x2;
const FRAME_TYPE_RST_STREAM: u8 = 0x3;
const FRAME_TYPE_SETTINGS: u8 = 0x4;
const FRAME_TYPE_PUSH_PROMISE: u8 = 0x5;
const FRAME_TYPE_PING: u8 = 0x6;
const FRAME_TYPE_GOAWAY: u8 = 0x7;
const FRAME_TYPE_WINDOW_UPDATE: u8 = 0x8;
const FRAME_TYPE_CONTINUATION: u8 = 0x9;

#[derive(Debug)]
pub enum Frame {
    Priority(PriorityFrame),
    RstStream(RstStreamFrame),
}
impl Frame {
    pub fn read_from<R: Read>(reader: R) -> ReadFrame<R> {
        ReadFrame { phase: Phase::A(FrameHeader::read_from(reader)) }
    }
}

#[derive(Debug)]
pub struct ReadFrame<R> {
    phase: Phase<ReadFrameHeader<R>, ReadFramePayload<R>>,
}
impl<R: Read> ReadFrame<R> {
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
impl<R: Read> Future for ReadFrame<R> {
    type Item = Frame;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Async::Ready(phase) = track!(self.phase.poll().map_err(Error::from))? {
            let next = match phase {
                Phase::A((reader, header)) => {
                    match header.frame_type {
                        FRAME_TYPE_DATA => unimplemented!(),
                        FRAME_TYPE_HEADERS => unimplemented!(),
                        FRAME_TYPE_PRIORITY => {
                            let future = track!(PriorityFrame::read_from(reader, header))?;
                            Phase::B(ReadFramePayload::Priority(future))
                        }
                        FRAME_TYPE_RST_STREAM => {
                            let future = track!(RstStreamFrame::read_from(reader, header))?;
                            Phase::B(ReadFramePayload::RstStream(future))
                        }
                        FRAME_TYPE_SETTINGS => unimplemented!(),
                        FRAME_TYPE_PUSH_PROMISE => unimplemented!(),
                        FRAME_TYPE_PING => unimplemented!(),
                        FRAME_TYPE_GOAWAY => unimplemented!(),
                        FRAME_TYPE_WINDOW_UPDATE => unimplemented!(),
                        FRAME_TYPE_CONTINUATION => unimplemented!(),
                        _ => {
                            // Implementations MUST ignore and discard any frame that has a type that is unknown.
                            // (RFC 7540#section-4.1)
                            unimplemented!(
                                "Check payload size and ignore this frame if it is valid"
                            )
                        }
                    }
                }
                _ => unreachable!(),
            };
            self.phase = next;
        }
        Ok(Async::NotReady)
    }
}

#[derive(Debug)]
enum ReadFramePayload<R> {
    Priority(ReadPriorityFrame<R>),
    RstStream(ReadRstStreamFrame<R>),
}
impl<R> ReadFramePayload<R> {
    pub fn reader(&self) -> &R {
        match *self {
            ReadFramePayload::Priority(ref f) => f.reader(),
            ReadFramePayload::RstStream(ref f) => f.reader(),
        }
    }
    pub fn reader_mut(&mut self) -> &mut R {
        match *self {
            ReadFramePayload::Priority(ref mut f) => f.reader_mut(),
            ReadFramePayload::RstStream(ref mut f) => f.reader_mut(),
        }
    }
}
impl<R: Read> Future for ReadFramePayload<R> {
    type Item = (R, Frame);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match *self {
            ReadFramePayload::Priority(ref mut f) => {
                Ok(track!(f.poll())?.map(|(r, f)| (r, Frame::Priority(f))))
            }
            ReadFramePayload::RstStream(ref mut f) => {
                Ok(track!(f.poll())?.map(|(r, f)| (r, Frame::RstStream(f))))
            }
        }
    }
}
