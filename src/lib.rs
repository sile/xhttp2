extern crate byteorder;
extern crate futures;
extern crate handy_async;
extern crate hpack_codec;
#[macro_use]
extern crate trackable;

pub use error::{Error, ErrorKind};

macro_rules! track_io {
    ($expr:expr) => {
        track!($expr.map_err(::Error::from))
    }
}

macro_rules! track_async_io {
    ($expr:expr) => {
        track!($expr.map_err(::Error::from))
    }
}

pub mod frame;
pub mod frame2;
pub mod preface;
pub mod priority;
pub mod setting;
pub mod stream;

mod error;

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod test {
    use futures::{Future, Stream};
    use hpack_codec::Decoder as HpackDecoder;

    use frame::{FrameReceiver, Frame};
    use stream::StreamId;
    use super::*;

    #[test]
    fn it_works() {
        let data;
        #[cfg_attr(rustfmt, rustfmt_skip)]
        {
            data = [
                80, 82, 73, 32, 42, 32, 72, 84, 84, 80, 47, 50, 46, 48, 13, 10, 13, 10,
                83, 77, 13, 10, 13, 10, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 76, 1, 4, 0, 0,
                0, 1, 131, 134, 69, 149, 98, 114, 209, 65, 252, 30, 202, 36, 95, 21, 133,
                42, 75, 99, 27, 135, 235, 25, 104, 160, 255, 65, 138, 160, 228, 29, 19,
                157, 9, 184, 200, 0, 15, 95, 139, 29, 117, 208, 98, 13, 38, 61, 76, 77,
                101, 100, 122, 141, 154, 202, 200, 180, 199, 96, 43, 186, 184, 22, 144,
                189, 255, 64, 2, 116, 101, 134, 77, 131, 53, 5, 177, 31, 0, 0, 11, 0,
                1, 0, 0, 0, 1, 0, 0, 0, 0, 6, 10, 4, 119, 100, 103, 107
            ];
        }
        let input = data;

        // the preface
        let input = track_try_unwrap!(preface::read_preface(&input[..]).wait());
        assert_eq!(input.len(), data.len() - preface::PREFACE_BYTES.len());

        // the header of the first frame
        let mut frame_rx = FrameReceiver::new(&input[..]).wait();
        let frame = track_try_unwrap!(frame_rx.next().expect("first frame"));
        if let Frame::Settings(frame) = frame {
            assert!(!frame.is_ack());
            assert!(frame.settings().is_empty());
        } else {
            panic!("{:?}", frame);
        }

        // the header of the second frame
        let frame = track_try_unwrap!(frame_rx.next().expect("second frame"));
        if let Frame::Headers(frame) = frame {
            assert_eq!(frame.stream_id, StreamId::from(1u8));
            assert_eq!(frame.padding_len, 0);
            assert!(!frame.end_stream);
            assert!(frame.end_headers);
            assert!(frame.priority.is_none());

            let mut decoder = HpackDecoder::new(4096);
            let mut block = track_try_unwrap!(decoder.enter_header_block(&frame.fragment[..]));
            let mut fields = Vec::new();
            while let Some(field) = track_try_unwrap!(block.decode_field()) {
                fields.push((
                    String::from_utf8(field.name().to_owned()).unwrap(),
                    String::from_utf8(field.value().to_owned()).unwrap(),
                ));
            }
            assert_eq!(
                fields,
                [
                    (":method".to_string(), "POST".to_string()),
                    (":scheme".to_string(), "http".to_string()),
                    (
                        ":path".to_string(),
                        "/helloworld.Greeter/SayHello".to_string(),
                    ),
                    (":authority".to_string(), "localhost:3000".to_string()),
                    ("content-type".to_string(), "application/grpc".to_string()),
                    ("user-agent".to_string(), "grpc-go/1.7.0-dev".to_string()),
                    ("te".to_string(), "trailers".to_string()),
                ]
            );
        } else {
            panic!("{:?}", frame);
        }

        // the header of the third frame
        let frame = track_try_unwrap!(frame_rx.next().expect("second frame"));
        if let Frame::Data(frame) = frame {
            assert_eq!(frame.stream_id, StreamId::from(1u8));
            assert!(frame.end_stream);
            assert_eq!(frame.data, [0, 0, 0, 0, 6, 10, 4, 119, 100, 103, 107]);
        } else {
            panic!("{:?}", frame);
        }

        // eos
        assert!(frame_rx.next().expect("eos").is_err());
    }
}
