extern crate clap;
extern crate futures;
extern crate fibers;
#[macro_use]
extern crate trackable;
extern crate xhttp2;

use std::net::SocketAddr;
use clap::{App, Arg};
use fibers::{Spawn, Executor, ThreadPoolExecutor};
use fibers::sync::mpsc;
use futures::{Future, Stream, Sink, Poll, StartSend, AsyncSink, Async};
use trackable::error::ErrorKindExt;
use xhttp2::{Error, ErrorKind};
use xhttp2::connection::{Connection, ChannelFactory, Bytes};
use xhttp2::frame::Frame;

struct FibersChannelFactory;
impl ChannelFactory for FibersChannelFactory {
    type Sender = FrameSender;
    type Receiver = FrameReceiver;
    fn channel(&mut self) -> (Self::Sender, Self::Receiver) {
        let (tx, rx) = mpsc::channel();
        (FrameSender(tx), FrameReceiver(rx))
    }
}

#[derive(Clone)]
struct FrameSender(mpsc::Sender<Frame<Bytes>>);
impl Sink for FrameSender {
    type SinkItem = Frame<Bytes>;
    type SinkError = Error;
    fn start_send(&mut self, item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
        track!(self.0.send(item).map(|_| AsyncSink::Ready).map_err(|_| {
            ErrorKind::InternalError.cause("receiver terminated").into()
        }))
    }
    fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
        Ok(Async::Ready(()))
    }
}

struct FrameReceiver(mpsc::Receiver<Frame<Bytes>>);
impl Stream for FrameReceiver {
    type Item = Frame<Bytes>;
    type Error = Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.0.poll().map_err(|_| unreachable!())
    }
}

fn main() {
    let matches = App::new("server")
        .arg(
            Arg::with_name("PORT")
                .short("p")
                .takes_value(true)
                .default_value("3000"),
        )
        .get_matches();
    let port = matches.value_of("PORT").unwrap();
    let addr: SocketAddr = format!("0.0.0.0:{}", port).parse().expect(
        "Invalid TCP bind address",
    );

    let mut executor = ThreadPoolExecutor::new().expect("Cannot create Executor");
    let handle0 = executor.handle();
    let monitor = executor.spawn_monitor(fibers::net::TcpListener::bind(addr).and_then(
        move |listener| {
            println!("# Start listening: {}: ", addr);
            listener.incoming().for_each(move |(client, addr)| {
                println!("# TCP CONNECTED: {}", addr);
                handle0.spawn(
                    client
                        .map_err(Error::from)
                        .and_then(move |client| {
                            Connection::accept(client.clone(), client, FibersChannelFactory)
                        })
                        .and_then(|mut connection| {
                            println!("# HTTP2 CONNECTED");
                            connection.ping([1; 8]);
                            connection.for_each(|event| {
                                println!("[EVENT] {:?}", event);
                                Ok(())
                            })
                        })
                        .then(|r| {
                            println!("# Client finished: {:?}", r);
                            Ok(())
                        }),
                );
                Ok(())
            })
        },
    ));
    let result = executor.run_fiber(monitor).expect("Execution failed");
    println!("# Listener finished: {:?}", result);
}
