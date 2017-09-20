use std::io::Read;
use byteorder::{BigEndian, ReadBytesExt};
use futures::{Future, Stream, Poll, Async};
use handy_async::future::Phase;
use handy_async::io::AsyncRead;
use handy_async::io::futures::ReadExact;

pub use self::data::DataFrame;
pub use self::goaway::GoawayFrame;
pub use self::headers::HeadersFrame;
pub use self::ping::PingFrame;
pub use self::priority::PriorityFrame;
pub use self::push_promise::PushPromiseFrame;
pub use self::rst_stream::RstStreamFrame;
pub use self::settings::SettingsFrame;
pub use self::window_update::WindowUpdateFrame;

use self::header::{FrameHeader, ReadFrameHeader};

use {Result, Error, ErrorKind};

mod data;
mod goaway;
mod header;
mod headers;
mod ping;
mod priority;
mod push_promise;
mod rst_stream;
mod settings;
mod window_update;

pub const FRAME_TYPE_DATA: u8 = 0x0;
pub const FRAME_TYPE_HEADERS: u8 = 0x1;
pub const FRAME_TYPE_PRIORITY: u8 = 0x2;
pub const FRAME_TYPE_RST_STREAM: u8 = 0x3;
pub const FRAME_TYPE_SETTINGS: u8 = 0x4;
pub const FRAME_TYPE_PUSH_PROMISE: u8 = 0x5;
pub const FRAME_TYPE_PING: u8 = 0x6;
pub const FRAME_TYPE_GOAWAY: u8 = 0x7;
pub const FRAME_TYPE_WINDOW_UPDATE: u8 = 0x8;
pub const FRAME_TYPE_CONTINUATION: u8 = 0x9;

mod flags {
    pub const ACK: u8 = 0x01;
    pub const END_STREAM: u8 = 0x01;
    pub const END_HEADERS: u8 = 0x04;
    pub const PADDED: u8 = 0x08;
    pub const PRIORITY: u8 = 0x20;
}

#[derive(Debug)]
pub struct Settings {
    pub max_frame_size: u32,
}

#[derive(Debug)]
pub enum Frame {
    Data(DataFrame),
    Headers(HeadersFrame),
    Priority(PriorityFrame),
    RstStream(RstStreamFrame),
    Settings(SettingsFrame),
    PushPromise(PushPromiseFrame),
    Ping(PingFrame),
    Goaway(GoawayFrame),
    WindowUpdate(WindowUpdateFrame),
    Continuation,
}
impl Frame {
    fn from_vec(header: &FrameHeader, payload: Vec<u8>) -> Result<Self> {
        match header.payload_type {
            FRAME_TYPE_DATA => track!(DataFrame::from_vec(header, payload)).map(Frame::Data),
            FRAME_TYPE_HEADERS => {
                track!(HeadersFrame::from_vec(header, payload).map(Frame::Headers))
            }
            FRAME_TYPE_PRIORITY => {
                track!(PriorityFrame::from_vec(header, payload).map(
                    Frame::Priority,
                ))
            }
            FRAME_TYPE_RST_STREAM => {
                track!(RstStreamFrame::from_vec(header, payload).map(
                    Frame::RstStream,
                ))
            }
            FRAME_TYPE_SETTINGS => {
                track!(SettingsFrame::from_vec(header, payload).map(
                    Frame::Settings,
                ))
            }
            FRAME_TYPE_PUSH_PROMISE => {
                track!(PushPromiseFrame::from_vec(header, payload).map(
                    Frame::PushPromise,
                ))
            }
            FRAME_TYPE_PING => track!(PingFrame::from_vec(header, payload).map(Frame::Ping)),
            FRAME_TYPE_GOAWAY => track!(GoawayFrame::from_vec(header, payload).map(Frame::Goaway)),
            FRAME_TYPE_WINDOW_UPDATE => {
                track!(WindowUpdateFrame::from_vec(header, payload).map(
                    Frame::WindowUpdate,
                ))
            }
            FRAME_TYPE_CONTINUATION => unimplemented!(),
            other => track_panic!(ErrorKind::Other, "Unknown payload type: {}", other),
        }
    }
}

#[derive(Debug)]
pub struct FrameReceiver<R> {
    settings: Settings,
    phase: Phase<ReadFrameHeader<R>, ReadFrame<R>>,
}
impl<R: Read> FrameReceiver<R> {
    pub fn new(reader: R, settings: Settings) -> Self {
        let phase = Phase::A(ReadFrameHeader::new(reader));
        FrameReceiver { settings, phase }
    }
}
impl<R: Read> Stream for FrameReceiver<R> {
    type Item = Frame;
    type Error = Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        while let Async::Ready(phase) = track!(self.phase.poll().map_err(Error::from))? {
            let (next, frame) = match phase {
                Phase::A((reader, header)) => {
                    track_assert!(
                        header.payload_length <= self.settings.max_frame_size,
                        ErrorKind::Other,
                        "Too large frame size: value={}, max={}",
                        header.payload_length,
                        self.settings.max_frame_size
                    );
                    (Phase::B(ReadFrame::new(reader, header)), None)
                }
                Phase::B((reader, frame)) => (Phase::A(ReadFrameHeader::new(reader)), Some(frame)),
                _ => unreachable!(),
            };
            self.phase = next;
            if let Some(frame) = frame {
                return Ok(Async::Ready(Some(frame)));
            }
        }
        Ok(Async::NotReady)
    }
}

#[derive(Debug)]
pub struct FrameSender;

#[derive(Debug)]
pub struct ReadFrame<R> {
    header: FrameHeader,
    future: ReadExact<R, Vec<u8>>,
}
impl<R: Read> ReadFrame<R> {
    pub fn new(reader: R, header: FrameHeader) -> Self {
        let future = reader.async_read_exact(vec![0; header.payload_length as usize]);
        ReadFrame { header, future }
    }
}
impl<R: Read> Future for ReadFrame<R> {
    type Item = (R, Frame);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((reader, buf)) = track!(self.future.poll().map_err(Error::from))? {
            let frame = track!(Frame::from_vec(&self.header, buf))?;
            return Ok(Async::Ready((reader, frame)));
        }
        Ok(Async::NotReady)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Priority {
    pub exclusive: bool,
    pub stream_dependency: u32,
    pub weight_minus_one: u8,
}
impl Priority {
    pub fn read_from<R: Read>(mut reader: R) -> Result<Self> {
        let value = track!(reader.read_u32::<BigEndian>().map_err(Error::from))?;
        let exclusive = (value >> 31) == 1;
        let stream_dependency = value & 0x7FFF_FFFF;
        let weight_minus_one = track!(reader.read_u8().map_err(Error::from))?;
        Ok(Priority {
            exclusive,
            stream_dependency,
            weight_minus_one,
        })
    }
}
