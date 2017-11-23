use std::io::Read;
use futures::{Future, Poll, Async};
use handy_async::future::Phase;

pub use self::continuation_frame::ContinuationFrame;
pub use self::data_frame::DataFrame;
pub use self::goaway_frame::GoawayFrame;
pub use self::headers_frame::HeadersFrame;
pub use self::ping_frame::PingFrame;
pub use self::priority_frame::PriorityFrame;
pub use self::push_promise_frame::PushPromiseFrame;
pub use self::rst_stream_frame::RstStreamFrame;
pub use self::settings_frame::SettingsFrame;
pub use self::window_update_frame::WindowUpdateFrame;

use Error;
use self::continuation_frame::ReadContinuationFrame;
use self::data_frame::ReadDataFrame;
use self::frame_header::{FrameHeader, ReadFrameHeader};
use self::goaway_frame::ReadGoawayFrame;
use self::headers_frame::ReadHeadersFrame;
use self::ping_frame::ReadPingFrame;
use self::priority_frame::ReadPriorityFrame;
use self::push_promise_frame::ReadPushPromiseFrame;
use self::rst_stream_frame::ReadRstStreamFrame;
use self::settings_frame::ReadSettingsFrame;
use self::window_update_frame::ReadWindowUpdateFrame;

mod continuation_frame;
mod data_frame;
mod frame_header;
mod goaway_frame;
mod headers_frame;
mod ping_frame;
mod priority_frame;
mod push_promise_frame;
mod rst_stream_frame;
mod settings_frame;
mod window_update_frame;

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
pub enum Frame<B> {
    Continuation(ContinuationFrame<B>),
    Data(DataFrame<B>),
    Goaway(GoawayFrame),
    Headers(HeadersFrame<B>),
    Ping(PingFrame),
    Priority(PriorityFrame),
    RstStream(RstStreamFrame),
    PushPromise(PushPromiseFrame<B>),
    Settings(SettingsFrame),
    WindowUpdate(WindowUpdateFrame),
}
impl<B: AsRef<[u8]>> Frame<B> {
    pub fn payload_len(&self) -> usize {
        match *self {
            Frame::Continuation(ref frame) => frame.payload_len(),
            Frame::Data(ref frame) => frame.payload_len(),
            Frame::Goaway(ref frame) => frame.payload_len(),
            Frame::Headers(ref frame) => frame.payload_len(),
            Frame::Ping(_) => 8,
            Frame::Priority(_) => 5,
            Frame::RstStream(_) => 4,
            Frame::PushPromise(ref frame) => frame.payload_len(),
            Frame::Settings(ref frame) => frame.payload_len(),
            Frame::WindowUpdate(_) => 4,
        }
    }
}
impl<R: Read> Frame<R> {
    pub fn read_from(reader: R) -> ReadFrame<R> {
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
    type Item = (R, Frame<Vec<u8>>);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Async::Ready(phase) = track!(self.phase.poll().map_err(Error::from))? {
            let next = match phase {
                Phase::A((reader, header)) => {
                    match header.frame_type {
                        FRAME_TYPE_DATA => {
                            let future = track!(DataFrame::read_from(reader, header))?;
                            Phase::B(ReadFramePayload::Data(future))
                        }
                        FRAME_TYPE_HEADERS => {
                            let future = track!(HeadersFrame::read_from(reader, header))?;
                            Phase::B(ReadFramePayload::Headers(future))
                        }
                        FRAME_TYPE_PRIORITY => {
                            let future = track!(PriorityFrame::read_from(reader, header))?;
                            Phase::B(ReadFramePayload::Priority(future))
                        }
                        FRAME_TYPE_RST_STREAM => {
                            let future = track!(RstStreamFrame::read_from(reader, header))?;
                            Phase::B(ReadFramePayload::RstStream(future))
                        }
                        FRAME_TYPE_SETTINGS => {
                            let future = track!(SettingsFrame::read_from(reader, header))?;
                            Phase::B(ReadFramePayload::Settings(future))
                        }
                        FRAME_TYPE_PUSH_PROMISE => {
                            let future = track!(PushPromiseFrame::read_from(reader, header))?;
                            Phase::B(ReadFramePayload::PushPromise(future))
                        }
                        FRAME_TYPE_PING => {
                            let future = track!(PingFrame::read_from(reader, header))?;
                            Phase::B(ReadFramePayload::Ping(future))
                        }
                        FRAME_TYPE_GOAWAY => {
                            let future = track!(GoawayFrame::read_from(reader, header))?;
                            Phase::B(ReadFramePayload::Goaway(future))
                        }
                        FRAME_TYPE_WINDOW_UPDATE => {
                            let future = track!(WindowUpdateFrame::read_from(reader, header))?;
                            Phase::B(ReadFramePayload::WindowUpdate(future))
                        }
                        FRAME_TYPE_CONTINUATION => {
                            let future = track!(ContinuationFrame::read_from(reader, header))?;
                            Phase::B(ReadFramePayload::Continuation(future))
                        }
                        _ => {
                            // Implementations MUST ignore and discard any frame that has
                            // a type that is unknown.
                            // (RFC 7540#section-4.1)
                            unimplemented!(
                                "Check payload size and ignore this frame if it is valid"
                            )
                        }
                    }
                }
                Phase::B((reader, frame)) => return Ok(Async::Ready((reader, frame))),
                _ => unreachable!(),
            };
            self.phase = next;
        }
        Ok(Async::NotReady)
    }
}

#[derive(Debug)]
enum ReadFramePayload<R> {
    Continuation(ReadContinuationFrame<R>),
    Data(ReadDataFrame<R>),
    Goaway(ReadGoawayFrame<R>),
    Headers(ReadHeadersFrame<R>),
    Ping(ReadPingFrame<R>),
    Priority(ReadPriorityFrame<R>),
    PushPromise(ReadPushPromiseFrame<R>),
    RstStream(ReadRstStreamFrame<R>),
    Settings(ReadSettingsFrame<R>),
    WindowUpdate(ReadWindowUpdateFrame<R>),
}
impl<R> ReadFramePayload<R> {
    pub fn reader(&self) -> &R {
        match *self {
            ReadFramePayload::Continuation(ref f) => f.reader(),
            ReadFramePayload::Data(ref f) => f.reader(),
            ReadFramePayload::Goaway(ref f) => f.reader(),
            ReadFramePayload::Headers(ref f) => f.reader(),
            ReadFramePayload::Ping(ref f) => f.reader(),
            ReadFramePayload::Priority(ref f) => f.reader(),
            ReadFramePayload::RstStream(ref f) => f.reader(),
            ReadFramePayload::PushPromise(ref f) => f.reader(),
            ReadFramePayload::Settings(ref f) => f.reader(),
            ReadFramePayload::WindowUpdate(ref f) => f.reader(),
        }
    }
    pub fn reader_mut(&mut self) -> &mut R {
        match *self {
            ReadFramePayload::Continuation(ref mut f) => f.reader_mut(),
            ReadFramePayload::Data(ref mut f) => f.reader_mut(),
            ReadFramePayload::Goaway(ref mut f) => f.reader_mut(),
            ReadFramePayload::Headers(ref mut f) => f.reader_mut(),
            ReadFramePayload::Ping(ref mut f) => f.reader_mut(),
            ReadFramePayload::Priority(ref mut f) => f.reader_mut(),
            ReadFramePayload::RstStream(ref mut f) => f.reader_mut(),
            ReadFramePayload::PushPromise(ref mut f) => f.reader_mut(),
            ReadFramePayload::Settings(ref mut f) => f.reader_mut(),
            ReadFramePayload::WindowUpdate(ref mut f) => f.reader_mut(),
        }
    }
}
impl<R: Read> Future for ReadFramePayload<R> {
    type Item = (R, Frame<Vec<u8>>);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match *self {
            ReadFramePayload::Continuation(ref mut f) => Ok(
                track!(f.poll())?.map(|(reader, frame)| {
                    (reader, Frame::Continuation(frame))
                }),
            ),
            ReadFramePayload::Data(ref mut f) => Ok(track!(f.poll())?.map(|(reader, frame)| {
                (reader, Frame::Data(frame))
            })),
            ReadFramePayload::Goaway(ref mut f) => {
                Ok(track!(f.poll())?.map(|(reader, frame)| {
                    (reader, Frame::Goaway(frame))
                }))
            }
            ReadFramePayload::Headers(ref mut f) => Ok(track!(f.poll())?.map(|(reader, frame)| {
                (reader, Frame::Headers(frame))
            })),
            ReadFramePayload::Ping(ref mut f) => {
                Ok(track!(f.poll())?.map(|(reader, frame)| {
                    (reader, Frame::Ping(frame))
                }))
            }
            ReadFramePayload::Priority(ref mut f) => {
                Ok(track!(f.poll())?.map(|(reader, frame)| {
                    (reader, Frame::Priority(frame))
                }))
            }
            ReadFramePayload::PushPromise(ref mut f) => Ok(
                track!(f.poll())?.map(|(reader, frame)| {
                    (reader, Frame::PushPromise(frame))
                }),
            ),
            ReadFramePayload::RstStream(ref mut f) => {
                Ok(track!(f.poll())?.map(|(reader, frame)| {
                    (reader, Frame::RstStream(frame))
                }))
            }
            ReadFramePayload::Settings(ref mut f) => {
                Ok(track!(f.poll())?.map(|(reader, frame)| {
                    (reader, Frame::Settings(frame))
                }))
            }
            ReadFramePayload::WindowUpdate(ref mut f) => {
                Ok(track!(f.poll())?.map(|(reader, frame)| {
                    (reader, Frame::WindowUpdate(frame))
                }))
            }
        }
    }
}
