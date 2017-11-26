use std::io::{Read, Write};
use futures::{Future, Poll, Async};
use handy_async::io::{AsyncRead, AsyncWrite};
use handy_async::io::futures::{ReadExact, WriteAll};

use {Result, Error, ErrorKind};
use setting::Setting;
use stream::StreamId;
use super::FrameHeader;

const FLAG_ACK: u8 = 0x1;

/// https://tools.ietf.org/html/rfc7540#section-6.5
#[derive(Debug, Clone)]
pub enum SettingsFrame {
    Syn(Vec<Setting>),
    Ack,
}
impl SettingsFrame {
    pub fn payload_len(&self) -> usize {
        if let SettingsFrame::Syn(ref settings) = *self {
            settings.len() * 6
        } else {
            0
        }
    }
    pub fn is_ack(&self) -> bool {
        if let SettingsFrame::Ack = *self {
            true
        } else {
            false
        }
    }
    pub fn settings(&self) -> &[Setting] {
        if let SettingsFrame::Syn(ref settings) = *self {
            settings
        } else {
            &[][..]
        }
    }
    pub fn frame_header(&self) -> FrameHeader {
        let mut flags = 0;
        if self.is_ack() {
            flags |= FLAG_ACK;
        }

        FrameHeader {
            payload_length: self.payload_len() as u32,
            frame_type: super::FRAME_TYPE_SETTINGS,
            flags,
            stream_id: StreamId::connection_control_stream_id(),
        }
    }
    pub fn write_into<W: Write>(self, writer: W) -> WriteSettingsFrame<W> {
        let mut buf = Vec::with_capacity(self.settings().len() * 6);
        for s in self.settings() {
            buf.extend_from_slice(&s.to_bytes()[..]);
        }
        WriteSettingsFrame(writer.async_write_all(buf))
    }
    pub fn read_from<R: Read>(reader: R, header: FrameHeader) -> Result<ReadSettingsFrame<R>> {
        track_assert_eq!(header.payload_length % 6, 0, ErrorKind::FrameSizeError);
        track_assert!(
            header.stream_id.is_connection_control_stream(),
            ErrorKind::ProtocolError
        );
        let bytes = vec![0; header.payload_length as usize];
        Ok(ReadSettingsFrame {
            header,
            future: reader.async_read_exact(bytes),
        })
    }
}

#[derive(Debug)]
pub struct WriteSettingsFrame<W>(WriteAll<W, Vec<u8>>);
impl<W: Write> Future for WriteSettingsFrame<W> {
    type Item = W;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        Ok(track_async_io!(self.0.poll())?.map(|(writer, _)| writer))
    }
}

#[derive(Debug)]
pub struct ReadSettingsFrame<R> {
    header: FrameHeader,
    future: ReadExact<R, Vec<u8>>,
}
impl<R> ReadSettingsFrame<R> {
    pub fn reader(&self) -> &R {
        self.future.reader()
    }
    pub fn reader_mut(&mut self) -> &mut R {
        self.future.reader_mut()
    }
}
impl<R: Read> Future for ReadSettingsFrame<R> {
    type Item = (R, SettingsFrame);
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready((reader, bytes)) = track_async_io!(self.future.poll())? {
            let frame = if (self.header.flags & FLAG_ACK) != 0 {
                track_assert_eq!(self.header.payload_length, 0, ErrorKind::FrameSizeError);
                SettingsFrame::Ack
            } else {
                let mut settings = Vec::new();
                for c in bytes.chunks(6) {
                    let chunk = [c[0], c[1], c[2], c[3], c[4], c[5]];
                    if let Some(setting) = track!(Setting::from_bytes(chunk))? {
                        settings.push(setting);
                    }
                }
                SettingsFrame::Syn(settings)
            };
            Ok(Async::Ready((reader, frame)))
        } else {
            Ok(Async::NotReady)
        }
    }
}
