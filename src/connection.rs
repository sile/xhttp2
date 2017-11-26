use std::collections::VecDeque;
use std::fmt;
use std::io::{Read, Write};
use futures::{self, Future, Poll, Async, Sink};

use {Result, Error, ErrorKind};
use frame::{self, Frame, SettingsFrame, FrameSink, FrameStream};
use preface::{self, ReadPreface};
use setting::{Setting, Settings};

// TODO: move
pub struct Bytes(Box<AsRef<[u8]> + Send + 'static>);
impl AsRef<[u8]> for Bytes {
    fn as_ref(&self) -> &[u8] {
        (*self.0).as_ref()
    }
}
impl fmt::Debug for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Bytes({:?})", self.as_ref())
    }
}

// TODO: move
pub trait ChannelFactory {
    type Sender: Sink<SinkItem = Frame<Bytes>, SinkError = Error> + Clone;
    type Receiver: futures::Stream<Item = Frame<Bytes>, Error = Error>;
    fn channel(&mut self) -> (Self::Sender, Self::Receiver);
}

// TODO: move
#[derive(Debug)]
pub enum Event {
    Stream,
    Pong { data: [u8; 8] },
}

#[derive(Debug)]
pub struct Connection<R, W: Write, C> {
    is_settings_received: bool,
    events: VecDeque<Event>,
    stream: FrameStream<R>,
    channel_factory: C,
    sink: FrameSink<W, Bytes>,
    settings: Settings,
}
impl<R: Read, W: Write, C> Connection<R, W, C> {
    pub fn accept(reader: R, writer: W, channel_factory: C) -> Accept<R, W, C> {
        let future = preface::read_preface(reader);
        Accept {
            future,
            writer: Some(writer),
            channel_factory: Some(channel_factory),
        }
    }

    pub fn ping(&mut self, data: [u8; 8]) {
        self.sink.start_write_frame(
            frame::PingFrame { ack: false, data },
        );
    }

    fn new(reader: R, writer: W, channel_factory: C) -> Self {
        let settings = Settings::default();
        let mut sink = FrameSink::new(writer);
        sink.start_write_frame(SettingsFrame::Syn(vec![])); // TODO:

        Connection {
            is_settings_received: false,
            events: VecDeque::new(),
            stream: FrameStream::new(reader),
            sink,
            settings,
            channel_factory,
        }
    }
    fn handle_continuation_frame(
        &mut self,
        frame: frame::ContinuationFrame<Vec<u8>>,
    ) -> Result<()> {
        unimplemented!("{:?}", frame);
    }
    fn handle_data_frame(&mut self, frame: frame::DataFrame<Vec<u8>>) -> Result<()> {
        unimplemented!("{:?}", frame);
    }
    fn handle_goaway_frame(&mut self, frame: frame::GoawayFrame) -> Result<()> {
        unimplemented!("{:?}", frame);
    }
    fn handle_headers_frame(&mut self, frame: frame::HeadersFrame<Vec<u8>>) -> Result<()> {
        if frame.end_stream {
            unimplemented!("{:?}", frame);
        }
        if frame.priority.is_some() {
            unimplemented!("{:?}", frame);
        }
        if !frame.end_headers {
            unimplemented!("{:?}", frame);
        }
        // self.events.push_back(

        //     Event::Headers { block: frame.fragment },
        // );
        //Ok(())
        unimplemented!("{:?}", frame);
    }
    fn handle_ping_frame(&mut self, frame: frame::PingFrame) -> Result<()> {
        if frame.ack {
            self.events.push_back(Event::Pong { data: frame.data });
        } else {
            self.sink.start_write_frame(frame::PingFrame {
                ack: true,
                data: frame.data,
            });
        }
        Ok(())
    }
    fn handle_priority_frame(&mut self, frame: frame::PriorityFrame) -> Result<()> {
        unimplemented!("{:?}", frame);
    }
    fn handle_rst_stream_frame(&mut self, frame: frame::RstStreamFrame) -> Result<()> {
        unimplemented!("{:?}", frame);
    }
    fn handle_push_promise_frame(&mut self, frame: frame::PushPromiseFrame<Vec<u8>>) -> Result<()> {
        unimplemented!("{:?}", frame);
    }
    fn handle_settings_frame(&mut self, frame: frame::SettingsFrame) -> Result<()> {
        match frame {
            SettingsFrame::Syn(settings) => {
                for setting in settings {
                    track!(self.handle_setting(setting))?;
                }
                self.is_settings_received = true;
            }
            SettingsFrame::Ack => {
                unimplemented!("{:?}", frame);
            }
        }
        Ok(())
    }
    fn handle_window_update_frame(&mut self, frame: frame::WindowUpdateFrame) -> Result<()> {
        unimplemented!("{:?}", frame);
    }
    fn handle_setting(&mut self, setting: Setting) -> Result<()> {
        unimplemented!("{:?}", setting);
    }
    fn handle_frame(&mut self, frame: Frame<Vec<u8>>) -> Result<()> {
        println!("[DEBUG] frame: {:?}", frame);
        match frame {
            Frame::Continuation(frame) => {
                // TODO: エラー種別は要確認（以下同）
                track_assert!(self.is_settings_received, ErrorKind::ProtocolError);
                track!(self.handle_continuation_frame(frame))?;
            }
            Frame::Data(frame) => {
                track_assert!(self.is_settings_received, ErrorKind::ProtocolError);
                track!(self.handle_data_frame(frame))?;
            }
            Frame::Goaway(frame) => {
                track_assert!(self.is_settings_received, ErrorKind::ProtocolError);
                track!(self.handle_goaway_frame(frame))?;
            }
            Frame::Headers(frame) => {
                track_assert!(self.is_settings_received, ErrorKind::ProtocolError);
                track!(self.handle_headers_frame(frame))?;
            }
            Frame::Ping(frame) => {
                track_assert!(self.is_settings_received, ErrorKind::ProtocolError);
                track!(self.handle_ping_frame(frame))?;
            }
            Frame::Priority(frame) => {
                track_assert!(self.is_settings_received, ErrorKind::ProtocolError);
                track!(self.handle_priority_frame(frame))?;
            }
            Frame::RstStream(frame) => {
                track_assert!(self.is_settings_received, ErrorKind::ProtocolError);
                track!(self.handle_rst_stream_frame(frame))?;
            }
            Frame::PushPromise(frame) => {
                track_assert!(self.is_settings_received, ErrorKind::ProtocolError);
                track!(self.handle_push_promise_frame(frame))?;
            }
            Frame::Settings(frame) => {
                track!(self.handle_settings_frame(frame))?;
            }
            Frame::WindowUpdate(frame) => {
                track_assert!(self.is_settings_received, ErrorKind::ProtocolError);
                track!(self.handle_window_update_frame(frame))?;
            }
        }
        Ok(())
    }
}
impl<R: Read, W: Write, C> futures::Stream for Connection<R, W, C> {
    type Item = Event;
    type Error = Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            if let Some(event) = self.events.pop_front() {
                return Ok(Async::Ready(Some(event)));
            }

            track!(self.sink.poll_complete())?;

            // TODO: handle errors and send goaway message if needed.
            match track!(futures::Stream::poll(&mut self.stream))? {
                Async::Ready(Some(frame)) => {
                    track!(self.handle_frame(frame))?;
                }
                Async::Ready(None) => return Ok(Async::Ready(None)),
                Async::NotReady => break,
            }
        }
        Ok(Async::NotReady)
    }
}

#[derive(Debug)]
pub struct Accept<R, W, C> {
    future: ReadPreface<R>,
    writer: Option<W>,
    channel_factory: Option<C>,
}
impl<R: Read, W: Write, C> Future for Accept<R, W, C> {
    type Item = Connection<R, W, C>;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready(reader) = track!(self.future.poll())? {
            let connection = Connection::new(
                reader,
                self.writer.take().expect("Never fails"),
                self.channel_factory.take().expect("Never fails"),
            );
            Ok(Async::Ready(connection))
        } else {
            Ok(Async::NotReady)
        }
    }
}
