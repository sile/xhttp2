use std::io::Read;
use byteorder::{BigEndian, ByteOrder};
use futures::{Future, Poll, Async};
use handy_async::io::AsyncRead;
use handy_async::io::futures::ReadExact;

use Error;
use stream::StreamId;

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
#[derive(Debug, Clone)]
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
    pub stream_id: StreamId,
}
impl FrameHeader {
    pub fn read_from<R: Read>(reader: R) -> ReadFrameHeader<R> {
        ReadFrameHeader(reader.async_read_exact([0; 9]))
    }
}

#[derive(Debug)]
pub struct ReadFrameHeader<R>(ReadExact<R, [u8; 9]>);
impl<R: Read> Future for ReadFrameHeader<R> {
    type Item = (R, FrameHeader);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((reader, bytes)) = track!(self.0.poll().map_err(Error::from))? {
            let payload_length = BigEndian::read_u24(&bytes[0..3]);
            let payload_type = bytes[3];
            let flags = bytes[4];
            let stream_id =
                StreamId::new_unchecked(BigEndian::read_u32(&bytes[5..9]) & 0x7FFF_FFFF);

            let header = FrameHeader {
                payload_length,
                payload_type,
                flags,
                stream_id,
            };
            Ok(Async::Ready((reader, header)))
        } else {
            Ok(Async::NotReady)
        }
    }
}
