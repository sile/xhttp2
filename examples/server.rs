extern crate clap;
extern crate futures;
extern crate handy_async;
extern crate fibers;
extern crate xhttp2;

use std::io;
use clap::{App, Arg};
use fibers::{Spawn, Executor, ThreadPoolExecutor};
use futures::{Future, Stream};
use handy_async::io::AsyncWrite;
use xhttp2::frame::FrameReceiver;

fn main() {
    let matches = App::new("tcp_echo_srv")
        .arg(
            Arg::with_name("PORT")
                .short("p")
                .takes_value(true)
                .default_value("3000"),
        )
        .get_matches();
    let port = matches.value_of("PORT").unwrap();
    let addr = format!("0.0.0.0:{}", port).parse().expect(
        "Invalid TCP bind address",
    );

    let mut executor = ThreadPoolExecutor::new().expect("Cannot create Executor");
    let handle0 = executor.handle();
    let monitor = executor.spawn_monitor(fibers::net::TcpListener::bind(addr).and_then(
        move |listener| {
            println!("# Start listening: {}: ", addr);
            listener.incoming().for_each(move |(client, addr)| {
                println!("# CONNECTED: {}", addr);
                let handle1 = handle0.clone();
                handle0.spawn(
                    client
                        .and_then(move |client| {
                            let (reader, writer) = (client.clone(), client);
                            let (tx, rx) = fibers::sync::mpsc::channel();

                            // writer
                            handle1.spawn(
                                rx.map_err(|_| -> io::Error { unreachable!() })
                                    .fold(writer, |writer, buf: Vec<u8>| {
                                        println!("# SEND: {} bytes", buf.len());
                                        writer.async_write_all(buf).map(|(w, _)| w).map_err(|e| {
                                            e.into_error()
                                        })
                                    })
                                    .then(|r| {
                                        println!("# Writer finished: {:?}", r);
                                        Ok(())
                                    }),
                            );

                            // reader
                            xhttp2::preface::read_preface(reader)
                                .and_then(|reader| {
                                    let stream = FrameReceiver::new(reader);
                                    stream.fold(tx, |tx, frame| {
                                        println!("# RECV: {:?}", frame);
                                        Ok(tx) as xhttp2::Result<_>
                                    })
                                })
                                .map_err(|e| {
                                    println!("{}", e);
                                    io::Error::new(io::ErrorKind::Other, e)
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
