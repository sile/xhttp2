use std::io::{Read, Write};
use byteorder::{ByteOrder, BigEndian};
use futures::{Future, Poll};
use handy_async::io::{AsyncRead, AsyncWrite};
use handy_async::io::futures::{ReadExact, WriteAll};

use {Result, ErrorKind, Error};
use stream::StreamId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Weight(u8);
impl Weight {
    pub fn new(weight: u16) -> Result<Self> {
        track_assert_ne!(weight, 0, ErrorKind::InternalError);
        track_assert!(weight < 257, ErrorKind::InternalError);
        Ok(Weight((weight - 1) as u8))
    }
    pub fn as_u16(&self) -> u16 {
        u16::from(self.0) + 1
    }
    pub fn from_weight_minus_one(weight_minus_one: u8) -> Self {
        Weight(weight_minus_one)
    }
    pub fn to_weight_minus_one(&self) -> u8 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct Priority {
    pub is_exclusive: bool,
    pub stream_dependency: StreamId,
    pub weight: Weight,
}
impl Priority {
    pub fn from_bytes(bytes: [u8; 5]) -> Self {
        let temp = BigEndian::read_u32(&bytes[0..4]);
        let is_exclusive = (temp >> 31) == 1;
        let stream_dependency = StreamId::new_unchecked(temp & 0x7FFF_FFFF);
        let weight = Weight::from_weight_minus_one(bytes[4]);
        Priority {
            is_exclusive,
            stream_dependency,
            weight,
        }
    }

    pub fn to_bytes(&self) -> [u8; 5] {
        let mut bytes = [0; 5];

        let temp = ((self.is_exclusive as u32) << 31) | self.stream_dependency.as_u32();
        BigEndian::write_u32(&mut bytes[0..4], temp);
        bytes[4] = self.weight.to_weight_minus_one();

        bytes
    }

    pub fn read_from<R: Read>(reader: R) -> ReadPriority<R> {
        ReadPriority(reader.async_read_exact([0; 5]))
    }

    pub fn write_into<W: Write>(self, writer: W) -> WritePriority<W> {
        WritePriority(writer.async_write_all(self.to_bytes()))
    }
}
impl Default for Priority {
    fn default() -> Self {
        Priority {
            is_exclusive: false,
            stream_dependency: StreamId::connection_control_stream_id(),
            weight: Weight::from_weight_minus_one(15),
        }
    }
}

#[derive(Debug)]
pub struct WritePriority<W>(WriteAll<W, [u8; 5]>);
impl<W: Write> Future for WritePriority<W> {
    type Item = W;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_async_io!(self.0.poll())?.map(|(writer, _)| writer))
    }
}

#[derive(Debug)]
pub struct ReadPriority<R>(ReadExact<R, [u8; 5]>);
impl<R> ReadPriority<R> {
    pub fn reader(&self) -> &R {
        self.0.reader()
    }
    pub fn reader_mut(&mut self) -> &mut R {
        self.0.reader_mut()
    }
}
impl<R: Read> Future for ReadPriority<R> {
    type Item = (R, Priority);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_async_io!(self.0.poll())?.map(|(reader, bytes)| {
            let priority = Priority::from_bytes(bytes);
            (reader, priority)
        }))
    }
}
