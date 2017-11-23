extern crate byteorder;
extern crate futures;
extern crate handy_async;
#[macro_use]
extern crate trackable;

pub use error::{Error, ErrorKind};

// TODO: remove
// macro_rules! track_io {
//     ($expr:expr) => {
//         track!($expr.map_err(::Error::from))
//     }
// }

macro_rules! track_async_io {
    ($expr:expr) => {
        track!($expr.map_err(::Error::from))
    } 
}

pub mod connection;
pub mod frame;
pub mod preface;
pub mod priority;
pub mod setting;
pub mod stream;

mod error;

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod test {
    use futures::Future;

    use super::*;
    use super::frame::Frame;

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

        // the first frame
        let (input, frame) = track_try_unwrap!(Frame::read_from(&input[..]).wait());
        if let Frame::Settings(frame) = frame {
            assert!(!frame.is_ack());
            assert!(frame.settings().is_empty());
        } else {
            panic!("{:?}", frame);
        };

        // the second frame
        let (input, frame) = track_try_unwrap!(Frame::read_from(&input[..]).wait());
        if let Frame::Headers(frame) = frame {
            assert_eq!(frame.stream_id, 1u8.into());
            assert!(frame.padding_len.is_none());
            assert!(!frame.end_stream);
            assert!(frame.end_headers);
            assert!(frame.priority.is_none());
            assert_eq!(frame.fragment.len(), 76);
        } else {
            panic!("{:?}", frame);
        };

        // the third frame
        let (input, frame) = track_try_unwrap!(Frame::read_from(&input[..]).wait());
        if let Frame::Data(frame) = frame {
            assert_eq!(frame.stream_id, 1u8.into());
            assert!(frame.end_stream);
            assert_eq!(frame.data.len(), 11);
        } else {
            panic!("{:?}", frame);
        };

        assert!(input.is_empty());
    }
}
