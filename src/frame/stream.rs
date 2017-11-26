use std::io::Read;
use futures::{Future, Stream, Poll, Async};

use Error;
use frame::{Frame, ReadFrame};
use setting::Settings;

#[derive(Debug)]
pub struct FrameStream<R> {
    max_frame_size: u32,
    future: ReadFrame<R>,
}
impl<R: Read> FrameStream<R> {
    pub fn new(reader: R) -> Self {
        let max_frame_size = Settings::default().max_frame_size;
        FrameStream {
            max_frame_size,
            future: Frame::read_from(reader, max_frame_size),
        }
    }
    pub fn set_max_frame_size(&mut self, size: u32) {
        self.max_frame_size = size;
    }
}
impl<R: Read> Stream for FrameStream<R> {
    type Item = Frame<Vec<u8>>;
    type Error = Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        // TODO: handle eof
        if let Async::Ready((reader, frame)) = track!(self.future.poll())? {
            self.future = Frame::read_from(reader, self.max_frame_size);
            Ok(Async::Ready(Some(frame)))
        } else {
            Ok(Async::NotReady)
        }
    }
}
