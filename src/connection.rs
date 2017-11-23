// use std::io::{Read, Write};
// use futures::{Future, Poll, Async};
// use handy_async::future::Phase;

// use {Error, ErrorKind};
// use frame::{self, Frame, ReadFrame, WriteFrame};
// use preface::{self, ReadPreface};
// use setting::Settings;

// #[derive(Debug)]
// pub struct Connection<R, W> {
//     reader: R,
//     writer: W,
//     settings: Settings,
// }
// impl<R: Read, W: Write> Connection<R, W> {
//     pub fn accept(reader: R, writer: W) -> Accept<R, W> {
//         // let server_preface = Frame::Settings {
//         //     io: writer,
//         //     frame: frame::SettingsFrame::Syn(Vec::new()),
//         // }; // TODO:

//         let read_phase = Phase::A(preface::read_preface(reader));
//         // let write_phase = Phase::A(server_preface.write_into());
//         Accept {
//             read_phase,
//             // write_phase,
//         }
//     }
// }

// #[derive(Debug)]
// pub struct Accept<R, W> {
//     read_phase: Phase<ReadPreface<R>, ReadFrame<R>, ReadFrame<R>>,
//     w: W, // write_phase: Phase<WriteFrame<W>>
// }
// impl<R: Read, W: Write> Future for Accept<R, W> {
//     type Item = Connection<R, W>;
//     type Error = Error;
//     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
//         // TODO: send goaway when an error ocurred
//         while let Async::Ready(phase) = track_async_io!(self.read_phase.poll())? {
//             let next = match phase {
//                 Phase::A(io) => Phase::B(Frame::read_from(io)),
//                 Phase::B(frame) => {
//                     let default_settings = Settings::default();
//                     track_assert!(
//                         frame.payload_len() <= default_settings.max_frame_size as usize,
//                         ErrorKind::FrameSizeError
//                     );

//                     println!("[DEBUG:{}:{}] {:?}", module_path!(), line!(), frame);
//                     if let Frame::Settings { io, frame } = frame {
//                         // TODO: send setting
//                         Phase::C(Frame::read_from(io))
//                     } else {
//                         panic!("TODO: Error handling");
//                     }
//                 }
//                 Phase::C(frame) => {
//                     //panic!("TODO: {:?}", frame);

//                     // return Ok(Async::Ready(Connection {
//                     //     io,
//                     //     settings: default_settings,
//                     // }));
//                 }
//                 _ => unreachable!(),
//             };
//             self.read_phase = next;
//         }
//         Ok(Async::NotReady)
//     }
// }
