use std::collections::{VecDeque, HashMap};
use std::fmt;
use std::io::{Read, Write};
use fibers::sync::mpsc;
use futures::{self, Future, Poll, Async, Sink};
use hpack_codec::Decoder as HpackDecoder;

use {Result, Error, ErrorKind};
use frame::{self, Frame, SettingsFrame, FrameSink, FrameStream};
use header::Header;
use preface::{self, ReadPreface};
use setting::{Setting, Settings};
use stream::{StreamId, Stream, StreamHandle, StreamItem};

// TODO: move
pub struct Bytes(Box<AsRef<[u8]> + Send + 'static>);
impl Bytes {
    pub fn new<B>(bytes: B) -> Self
    where
        B: AsRef<[u8]> + Send + 'static,
    {
        Bytes(Box::new(bytes))
    }
}
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
#[derive(Debug)]
pub enum Event {
    Stream(Stream),
    Pong { data: [u8; 8] },
}

#[derive(Debug)]
pub struct Connection<R, W: Write> {
    is_settings_received: bool,
    events: VecDeque<Event>,
    stream: FrameStream<R>,
    sink: FrameSink<W, Bytes>,
    settings: Settings,
    next_self_stream_id: StreamId,
    next_peer_stream_id: StreamId,
    streams: HashMap<StreamId, StreamHandle>,
    stream_item_tx: mpsc::Sender<(StreamId, StreamItem)>,
    stream_item_rx: mpsc::Receiver<(StreamId, StreamItem)>,
    hpack_decoder: HpackDecoder,
}
impl<R: Read, W: Write> Connection<R, W> {
    pub fn accept(reader: R, writer: W) -> Accept<R, W> {
        let future = preface::read_preface(reader);
        Accept {
            future,
            writer: Some(writer),
        }
    }

    pub fn ping(&mut self, data: [u8; 8]) {
        self.sink.start_write_frame(
            frame::PingFrame { ack: false, data },
        );
    }

    fn new(reader: R, writer: W) -> Self {
        let settings = Settings::default();
        let mut sink = FrameSink::new(writer);
        sink.start_write_frame(SettingsFrame::Syn(vec![])); // TODO:

        let (stream_item_tx, stream_item_rx) = mpsc::channel();
        Connection {
            is_settings_received: false,
            events: VecDeque::new(),
            stream: FrameStream::new(reader),
            sink,
            settings,
            next_self_stream_id: StreamId::from(2u8), // TODO: use `is_server`
            next_peer_stream_id: StreamId::from(1u8),
            streams: HashMap::new(),
            stream_item_tx,
            stream_item_rx,
            hpack_decoder: HpackDecoder::new(4096),
        }
    }
    fn handle_continuation_frame(
        &mut self,
        frame: frame::ContinuationFrame<Vec<u8>>,
    ) -> Result<()> {
        unimplemented!("{:?}", frame);
    }
    fn handle_data_frame(&mut self, frame: frame::DataFrame<Vec<u8>>) -> Result<()> {
        // TODO: flow control
        if let Some(ref mut stream) = self.streams.get_mut(&frame.stream_id) {
            stream.handle_data(frame.data);
            if frame.end_stream {
                stream.handle_end_stream();
            }
        } else {
            // > If a DATA frame is received
            // > whose stream is not in "open" or "half-closed (local)" state, the
            // > recipient MUST respond with a stream error (Section 5.4.2) of type
            // > STREAM_CLOSED.
            // >
            // > [RFC 7540]
            unimplemented!("{:?}", frame);
        }
        Ok(())
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
        if self.streams.contains_key(&frame.stream_id) {
            unimplemented!("{:?}", frame);
        }

        // > The identifier of a newly established stream MUST be numerically
        // > greater than all streams that the initiating endpoint has opened or
        // > reserved.  This governs streams that are opened using a HEADERS frame
        // > and streams that are reserved using PUSH_PROMISE.  An endpoint that
        // > receives an unexpected stream identifier MUST respond with a
        // > connection error (Section 5.4.1) of type PROTOCOL_ERROR.
        // >
        // > [RFC 7540]
        track_assert!(
            frame.stream_id >= self.next_peer_stream_id,
            ErrorKind::ProtocolError
        );
        let header = track!(Header::decode(&mut self.hpack_decoder, &frame.fragment))?;

        let (stream, mut handle) = Stream::new(frame.stream_id, self.stream_item_tx.clone());
        track!(handle.handle_header(header))?;

        self.streams.insert(frame.stream_id, handle);
        self.events.push_back(Event::Stream(stream));
        Ok(())
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
impl<R: Read, W: Write> futures::Stream for Connection<R, W> {
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
pub struct Accept<R, W> {
    future: ReadPreface<R>,
    writer: Option<W>,
}
impl<R: Read, W: Write> Future for Accept<R, W> {
    type Item = Connection<R, W>;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready(reader) = track!(self.future.poll())? {
            let connection = Connection::new(reader, self.writer.take().expect("Never fails"));
            Ok(Async::Ready(connection))
        } else {
            Ok(Async::NotReady)
        }
    }
}
