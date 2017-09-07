use std::io::Read;
use byteorder::{ReadBytesExt, BigEndian};
use futures::{Future, Poll, Async};
use handy_async::io::AsyncRead;
use handy_async::io::futures::ReadExact;

use {Result, Error};

pub const FRAME_TYPE_DATA: u8 = 0x0;
pub const FRAME_TYPE_HEADERS: u8 = 0x1;
pub const FRAME_TYPE_PRIORITY: u8 = 0x2;
pub const FRAME_TYPE_RST_STREAM: u8 = 0x3;
pub const FRAME_TYPE_SETTINGS: u8 = 0x4;
pub const FRAME_TYPE_PUSH_PROMISE: u8 = 0x5;
pub const FRAME_TYPE_PING: u8 = 0x6;
pub const FRAME_TYPE_GOAWAY: u8 = 0x7;
pub const FRAME_TYPE_WINDOW_UPDATE: u8 = 0x8;
pub const FRAME_TYPE_CONTINUATION: u8 = 0x9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    Data,
    Headers,
    Priority,
    RstStream,
    Settings,
    PushPromise,
    Ping,
    Goaway,
    WindowUpdate,
    Continuation,
    Unknown(u8),
}

/// https://tools.ietf.org/html/rfc7540#section-4
///
/// ```text
///    +-----------------------------------------------+
///    |                 Length (24)                   |
///    +---------------+---------------+---------------+
///    |   Type (8)    |   Flags (8)   |
///    +-+-------------+---------------+-------------------------------+
///    |R|                 Stream Identifier (31)                      |
///    +=+=============================================================+
///    |                   Frame Payload (0...)                      ...
///    +---------------------------------------------------------------+
///
///                          Figure 1: Frame Layout
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameHeader {
    /// Length:  The length of the frame payload expressed as an unsigned
    /// 24-bit integer.  Values greater than 2^14 (16,384) MUST NOT be
    /// sent unless the receiver has set a larger value for
    /// SETTINGS_MAX_FRAME_SIZE.
    ///
    /// The 9 octets of the frame header are not included in this value.
    pub payload_length: u32, // u24

    /// Type:  The 8-bit type of the frame.  The frame type determines the
    /// format and semantics of the frame.  Implementations MUST ignore
    /// and discard any frame that has a type that is unknown.
    pub payload_type: u8,

    /// Flags:  An 8-bit field reserved for boolean flags specific to the
    /// frame type.
    /// Flags are assigned semantics specific to the indicated frame type.
    /// Flags that have no defined semantics for a particular frame type
    /// MUST be ignored and MUST be left unset (0x0) when sending.
    pub flags: u8,

    /// Stream Identifier:  A stream identifier (see Section 5.1.1) expressed
    /// as an unsigned 31-bit integer.  The value 0x0 is reserved for
    /// frames that are associated with the connection as a whole as
    /// opposed to an individual stream.
    ///
    /// R: A reserved 1-bit field.  The semantics of this bit are undefined,
    /// and the bit MUST remain unset (0x0) when sending and MUST be
    /// ignored when receiving.
    pub stream_id: u32,
}
impl FrameHeader {
    pub fn read_from<R: Read>(mut reader: R) -> Result<Self> {
        let payload_length = track!(reader.read_u24::<BigEndian>().map_err(Error::from))?;
        let payload_type = track!(reader.read_u8().map_err(Error::from))?;
        let flags = track!(reader.read_u8().map_err(Error::from))?;
        let stream_id = track!(reader.read_u32::<BigEndian>().map_err(Error::from))?;
        Ok(FrameHeader {
            payload_length,
            payload_type,
            flags,
            stream_id: stream_id & 0x7FFFFFFF,
        })
    }
}

pub fn read_frame_header<R: Read>(reader: R) -> ReadFrameHeader<R> {
    ReadFrameHeader(reader.async_read_exact([0; 9]))
}

#[derive(Debug)]
pub struct ReadFrameHeader<R>(ReadExact<R, [u8; 9]>);
impl<R: Read> Future for ReadFrameHeader<R> {
    type Item = (R, FrameHeader);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((reader, bytes)) = track!(self.0.poll().map_err(Error::from))? {
            let header = track!(FrameHeader::read_from(&bytes[..]))?;
            Ok(Async::Ready((reader, header)))
        } else {
            Ok(Async::NotReady)
        }
    }
}

bitflags! {
    struct HeadersFlags: u8 {
        const HEADERS_END_STREAM = 0x01;
        const HEADERS_END_HEADERS = 0x04;
        const HEADERS_PADDED = 0x08;
        const HEADERS_PRIORITY = 0x20;
    }
}

/// https://tools.ietf.org/html/rfc7540#section-6.2
///
/// ```text
///    +---------------+
///    |Pad Length? (8)|
///    +-+-------------+-----------------------------------------------+
///    |E|                 Stream Dependency? (31)                     |
///    +-+-------------+-----------------------------------------------+
///    |  Weight? (8)  |
///    +-+-------------+-----------------------------------------------+
///    |                   Header Block Fragment (*)                 ...
///    +---------------------------------------------------------------+
///    |                           Padding (*)                       ...
///    +---------------------------------------------------------------+
///
///                      Figure 7: HEADERS Frame Payload
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadersFrame {
    pub priority: Option<Priority>,
    pub fragment: Vec<u8>, // TODO: HeaderBlockFragment,
    pub padding: Vec<u8>,
}
impl HeadersFrame {
    pub fn read_from<R: Read>(mut reader: R, header: &FrameHeader) -> Result<Self> {
        let flags = HeadersFlags::from_bits_truncate(header.flags);
        let mut fragment_len = header.payload_length as usize;

        let padding_len = if flags.contains(HEADERS_PADDED) {
            fragment_len -= 1;
            track!(reader.read_u8().map_err(Error::from))?
        } else {
            0
        };
        fragment_len -= padding_len as usize;

        let priority = if flags.contains(HEADERS_PRIORITY) {
            fragment_len -= 4;
            Some(track!(Priority::read_from(&mut reader))?)
        } else {
            None
        };

        let mut fragment = vec![0; fragment_len];
        track!(reader.read_exact(&mut fragment).map_err(Error::from))?;

        let mut padding = vec![0; padding_len as usize];
        track!(reader.read_exact(&mut padding).map_err(Error::from))?;

        Ok(HeadersFrame {
            priority,
            fragment,
            padding,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Priority {
    pub exclusive: bool,
    pub stream_dependency: u32,
    pub weight_minus_one: u8,
}
impl Priority {
    pub fn read_from<R: Read>(mut reader: R) -> Result<Self> {
        let value = track!(reader.read_u32::<BigEndian>().map_err(Error::from))?;
        let exclusive = (value >> 31) == 1;
        let stream_dependency = value & 0x7FFF_FFFF;
        let weight_minus_one = track!(reader.read_u8().map_err(Error::from))?;
        Ok(Priority {
            exclusive,
            stream_dependency,
            weight_minus_one,
        })
    }
}

pub fn read_headers_frame<R: Read>(reader: R, header: FrameHeader) -> ReadHeadersFrame<R> {
    let buf = vec![0; header.payload_length as usize];
    ReadHeadersFrame(reader.async_read_exact(buf), header)
}

#[derive(Debug)]
pub struct ReadHeadersFrame<R>(ReadExact<R, Vec<u8>>, FrameHeader);
impl<R: Read> Future for ReadHeadersFrame<R> {
    type Item = (R, HeadersFrame);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((reader, bytes)) = track!(self.0.poll().map_err(Error::from))? {
            let frame = track!(HeadersFrame::read_from(&bytes[..], &self.1))?;
            Ok(Async::Ready((reader, frame)))
        } else {
            Ok(Async::NotReady)
        }
    }
}
