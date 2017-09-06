extern crate byteorder;
extern crate fibers;
extern crate futures;
extern crate handy_async;
#[macro_use]
extern crate trackable;

pub use error::{Error, ErrorKind};

pub mod frame;
pub mod preface;

mod error;

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod test {
    use futures::Future;
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
        let (_input, header) = track_try_unwrap!(frame::read_frame_header(&input[..]).wait());
        assert_eq!(
            header,
            frame::FrameHeader {
                payload_length: 0,
                payload_type: 4,
                flags: 0,
                stream_id: 0,
            }
        );
    }
}
