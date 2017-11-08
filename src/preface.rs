// https://tools.ietf.org/html/rfc7540#section-3.5
use std::io::Read;
use futures::{Future, Poll, Async};
use handy_async::io::AsyncRead;
use handy_async::io::futures::ReadExact;

use {Error, ErrorKind};

pub(crate) const PREFACE_BYTES: [u8; 24] = *b"PRI * HTTP/2.0\r\n\r\nSM\r\n\r\n";

pub fn read_preface<R: Read>(reader: R) -> ReadPreface<R> {
    ReadPreface(reader.async_read_exact([0; 24]))
}

#[derive(Debug)]
pub struct ReadPreface<R>(ReadExact<R, [u8; 24]>);
impl<R: Read> Future for ReadPreface<R> {
    type Item = R;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((reader, bytes)) = track!(self.0.poll().map_err(Error::from))? {
            track_assert_eq!(bytes, PREFACE_BYTES, ErrorKind::ProtocolError); // TODO
            Ok(Async::Ready(reader))
        } else {
            Ok(Async::NotReady)
        }
    }
}
