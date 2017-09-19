use std::io::Read;
use futures::{Future, Stream, Poll, Async};
use handy_async::future::Phase;
use handy_async::io::AsyncRead;
use handy_async::io::futures::ReadExact;

use self::header::{FrameHeader, ReadFrameHeader};

use {Error, ErrorKind};

mod header;

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

#[derive(Debug)]
pub struct Settings {
    pub max_frame_size: u32,
}

#[derive(Debug)]
pub enum Frame {
    Data,
    Headers,
    Priority,
    RstStream,
    Settings,
    PushPromize,
    Ping,
    Goaway,
    WindowUpdate,
    Continuation,
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
            let next = match phase {
                Phase::A((reader, header)) => {
                    track_assert!(
                        header.payload_length <= self.settings.max_frame_size,
                        ErrorKind::Other,
                        "Too large frame size: value={}, max={}",
                        header.payload_length,
                        self.settings.max_frame_size
                    );
                    Phase::B(ReadFrame::new(reader, header))
                }
                _ => unreachable!(),
            };
            self.phase = next;
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
            let frame = track!(Frame::from_vec(buf))?;
            return Ok(Async::Ready((reader, frame)));
        }
        Ok(Async::NotReady)
    }
}
