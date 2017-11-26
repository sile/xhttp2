extern crate clap;
extern crate futures;
extern crate fibers;
extern crate trackable;
extern crate xhttp2;

use std::net::SocketAddr;
use clap::{App, Arg};
use fibers::{Spawn, Executor, ThreadPoolExecutor};
use futures::{Future, Stream};
use xhttp2::Error;
use xhttp2::connection::Connection;

fn main() {
    let matches = App::new("server")
        .arg(
            Arg::with_name("PORT")
                .short("p")
                .takes_value(true)
                .default_value("50051"),
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
                        .and_then(move |client| Connection::accept(client.clone(), client))
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
