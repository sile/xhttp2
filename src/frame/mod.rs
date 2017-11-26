use std::io::{Read, Write};
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
pub use self::sink::FrameSink;
pub use self::stream::FrameStream;
pub use self::window_update_frame::WindowUpdateFrame;

use {Error, ErrorKind};
use self::continuation_frame::{ReadContinuationFrame, WriteContinuationFrame};
use self::data_frame::{ReadDataFrame, WriteDataFrame};
use self::frame_header::{FrameHeader, ReadFrameHeader, WriteFrameHeader};
use self::goaway_frame::{ReadGoawayFrame, WriteGoawayFrame};
use self::headers_frame::{ReadHeadersFrame, WriteHeadersFrame};
use self::ping_frame::{ReadPingFrame, WritePingFrame};
use self::priority_frame::{ReadPriorityFrame, WritePriorityFrame};
use self::push_promise_frame::{ReadPushPromiseFrame, WritePushPromiseFrame};
use self::rst_stream_frame::{ReadRstStreamFrame, WriteRstStreamFrame};
use self::settings_frame::{ReadSettingsFrame, WriteSettingsFrame};
use self::window_update_frame::{ReadWindowUpdateFrame, WriteWindowUpdateFrame};

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
mod sink;
mod stream;
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
            Frame::Ping(ref frame) => frame.payload_len(),
            Frame::Priority(ref frame) => frame.payload_len(),
            Frame::RstStream(ref frame) => frame.payload_len(),
            Frame::PushPromise(ref frame) => frame.payload_len(),
            Frame::Settings(ref frame) => frame.payload_len(),
            Frame::WindowUpdate(ref frame) => frame.payload_len(),
        }
    }
    pub fn frame_header(&self) -> FrameHeader {
        match *self {
            Frame::Continuation(ref frame) => frame.frame_header(),
            Frame::Data(ref frame) => frame.frame_header(),
            Frame::Goaway(ref frame) => frame.frame_header(),
            Frame::Headers(ref frame) => frame.frame_header(),
            Frame::Ping(ref frame) => frame.frame_header(),
            Frame::Priority(ref frame) => frame.frame_header(),
            Frame::RstStream(ref frame) => frame.frame_header(),
            Frame::PushPromise(ref frame) => frame.frame_header(),
            Frame::Settings(ref frame) => frame.frame_header(),
            Frame::WindowUpdate(ref frame) => frame.frame_header(),
        }
    }
    pub fn write_into<W: Write>(self, writer: W) -> WriteFrame<W, B> {
        let phase = Phase::A(self.frame_header().write_into(writer));
        let frame = Some(self);
        WriteFrame { frame, phase }
    }
}
impl Frame<Vec<u8>> {
    pub fn read_from<R: Read>(reader: R, max_frame_size: u32) -> ReadFrame<R> {
        let phase = Phase::A(FrameHeader::read_from(reader));
        ReadFrame {
            max_frame_size,
            phase,
        }
    }
}
impl<B> From<ContinuationFrame<B>> for Frame<B> {
    fn from(f: ContinuationFrame<B>) -> Self {
        Frame::Continuation(f)
    }
}
impl<B> From<DataFrame<B>> for Frame<B> {
    fn from(f: DataFrame<B>) -> Self {
        Frame::Data(f)
    }
}
impl<B> From<GoawayFrame> for Frame<B> {
    fn from(f: GoawayFrame) -> Self {
        Frame::Goaway(f)
    }
}
impl<B> From<HeadersFrame<B>> for Frame<B> {
    fn from(f: HeadersFrame<B>) -> Self {
        Frame::Headers(f)
    }
}
impl<B> From<PingFrame> for Frame<B> {
    fn from(f: PingFrame) -> Self {
        Frame::Ping(f)
    }
}
impl<B> From<PriorityFrame> for Frame<B> {
    fn from(f: PriorityFrame) -> Self {
        Frame::Priority(f)
    }
}
impl<B> From<RstStreamFrame> for Frame<B> {
    fn from(f: RstStreamFrame) -> Self {
        Frame::RstStream(f)
    }
}
impl<B> From<PushPromiseFrame<B>> for Frame<B> {
    fn from(f: PushPromiseFrame<B>) -> Self {
        Frame::PushPromise(f)
    }
}
impl<B> From<SettingsFrame> for Frame<B> {
    fn from(f: SettingsFrame) -> Self {
        Frame::Settings(f)
    }
}
impl<B> From<WindowUpdateFrame> for Frame<B> {
    fn from(f: WindowUpdateFrame) -> Self {
        Frame::WindowUpdate(f)
    }
}

#[derive(Debug)]
pub struct WriteFrame<W: Write, B: AsRef<[u8]>> {
    frame: Option<Frame<B>>,
    phase: Phase<WriteFrameHeader<W>, WriteFramePayload<W, B>>,
}
impl<W: Write, B: AsRef<[u8]>> Future for WriteFrame<W, B> {
    type Item = W;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Async::Ready(phase) = track_async_io!(self.phase.poll())? {
            let next = match phase {
                Phase::A(writer) => {
                    let frame = self.frame.take().expect("Never fails");
                    let future = match frame {
                        Frame::Continuation(frame) => {
                            WriteFramePayload::Continuation(frame.write_into(writer))
                        }
                        Frame::Data(frame) => WriteFramePayload::Data(frame.write_into(writer)),
                        Frame::Goaway(frame) => WriteFramePayload::Goaway(frame.write_into(writer)),
                        Frame::Headers(frame) => WriteFramePayload::Headers(
                            frame.write_into(writer),
                        ),
                        Frame::Ping(frame) => WriteFramePayload::Ping(frame.write_into(writer)),
                        Frame::Priority(frame) => WriteFramePayload::Priority(
                            frame.write_into(writer),
                        ),
                        Frame::PushPromise(frame) => WriteFramePayload::PushPromise(
                            frame.write_into(writer),
                        ),
                        Frame::RstStream(frame) => WriteFramePayload::RstStream(
                            frame.write_into(writer),
                        ),
                        Frame::Settings(frame) => WriteFramePayload::Settings(
                            frame.write_into(writer),
                        ),
                        Frame::WindowUpdate(frame) => WriteFramePayload::WindowUpdate(
                            frame.write_into(writer),
                        ),
                    };
                    Phase::B(future)
                }
                Phase::B(writer) => return Ok(Async::Ready(writer)),
                _ => unreachable!(),
            };
            self.phase = next;
        }
        Ok(Async::NotReady)
    }
}

#[derive(Debug)]
enum WriteFramePayload<W: Write, B: AsRef<[u8]>> {
    Continuation(WriteContinuationFrame<W, B>),
    Data(WriteDataFrame<W, B>),
    Goaway(WriteGoawayFrame<W>),
    Headers(WriteHeadersFrame<W, B>),
    Ping(WritePingFrame<W>),
    Priority(WritePriorityFrame<W>),
    PushPromise(WritePushPromiseFrame<W, B>),
    RstStream(WriteRstStreamFrame<W>),
    Settings(WriteSettingsFrame<W>),
    WindowUpdate(WriteWindowUpdateFrame<W>),
}
impl<W: Write, B: AsRef<[u8]>> Future for WriteFramePayload<W, B> {
    type Item = W;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match *self {
            WriteFramePayload::Continuation(ref mut f) => track!(f.poll()),
            WriteFramePayload::Data(ref mut f) => track!(f.poll()),
            WriteFramePayload::Goaway(ref mut f) => track!(f.poll()),
            WriteFramePayload::Headers(ref mut f) => track!(f.poll()),
            WriteFramePayload::Ping(ref mut f) => track!(f.poll()),
            WriteFramePayload::Priority(ref mut f) => track!(f.poll()),
            WriteFramePayload::PushPromise(ref mut f) => track!(f.poll()),
            WriteFramePayload::RstStream(ref mut f) => track!(f.poll()),
            WriteFramePayload::Settings(ref mut f) => track!(f.poll()),
            WriteFramePayload::WindowUpdate(ref mut f) => track!(f.poll()),
        }
    }
}

#[derive(Debug)]
pub struct ReadFrame<R> {
    max_frame_size: u32,
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
                    track_assert!(
                        header.payload_length <= self.max_frame_size,
                        ErrorKind::FrameSizeError
                    );

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
