use std::collections::VecDeque;
use std::io::Write;
use std::mem;

use futures::{Sink, StartSend, Poll, Async, AsyncSink, Future};

use Error;
use frame::{Frame, WriteFrame};

#[derive(Debug)]
pub struct FrameSink<W: Write, B: AsRef<[u8]>> {
    queue: VecDeque<Frame<B>>,
    state: FrameSinkState<W, B>,
}
impl<W: Write, B: AsRef<[u8]>> FrameSink<W, B> {
    pub fn new(writer: W) -> Self {
        FrameSink {
            queue: VecDeque::new(),
            state: FrameSinkState::Idle(writer),
        }
    }
    pub fn start_write_frame<T: Into<Frame<B>>>(&mut self, frame: T) {
        let _ = self.start_send(frame.into());
    }
}
impl<W: Write, B: AsRef<[u8]>> Sink for FrameSink<W, B> {
    type SinkItem = Frame<B>;
    type SinkError = Error;
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        if let FrameSinkState::Writing(_) = self.state {
            self.queue.push_back(item);
        } else if let FrameSinkState::Idle(writer) =
            mem::replace(&mut self.state, FrameSinkState::Done)
        {
            self.state = FrameSinkState::Writing(item.write_into(writer));
        } else {
            unreachable!()
        }
        Ok(AsyncSink::Ready)
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        loop {
            let next = if let FrameSinkState::Writing(ref mut f) = self.state {
                if let Async::Ready(writer) = track!(f.poll())? {
                    if let Some(frame) = self.queue.pop_front() {
                        FrameSinkState::Writing(frame.write_into(writer))
                    } else {
                        FrameSinkState::Idle(writer)
                    }
                } else {
                    break;
                }
            } else {
                break;
            };
            self.state = next;
        }
        Ok(Async::Ready(()))
    }
}

#[derive(Debug)]
enum FrameSinkState<W: Write, B: AsRef<[u8]>> {
    Idle(W),
    Writing(WriteFrame<W, B>),
    Done,
}
