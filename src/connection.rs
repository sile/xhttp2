use std::io::Read;
use futures::{Future, Poll, Async};
use handy_async::future::Phase;

use Error;
use frame::{Frame, ReadFrame};
use preface::{self, ReadPreface};

#[derive(Debug)]
pub struct Connection<T> {
    io: T,
}
impl<T: Read> Connection<T> {
    pub fn accept(io: T) -> Accept<T> {
        let phase = Phase::A(preface::read_preface(io));
        Accept { phase }
    }
}

#[derive(Debug)]
pub struct Accept<T> {
    phase: Phase<ReadPreface<T>, ReadFrame<T>>,
}
impl<T: Read> Future for Accept<T> {
    type Item = Connection<T>;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        while let Async::Ready(phase) = track_async_io!(self.phase.poll())? {
            let next = match phase {
                Phase::A(io) => Phase::B(Frame::read_from(io)),
                Phase::B(frame) => {
                    if let Frame::Settings { io, frame } = frame {
                        // TODO: send setting
                        return Ok(Async::Ready(Connection { io }));
                    } else {
                        panic!("TODO: Error handling");
                    }
                }
                _ => unreachable!(),
            };
            self.phase = next;
        }
        Ok(Async::NotReady)
    }
}
