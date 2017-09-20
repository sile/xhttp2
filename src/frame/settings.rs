use byteorder::{ByteOrder, BigEndian};

use {Result, ErrorKind};
use super::FrameHeader;
use super::flags;

const SETTINGS_HEADER_TABLE_SIZE: u16 = 0x1;
const SETTINGS_ENABLE_PUSH: u16 = 0x2;
const SETTINGS_MAX_CONCURRENT_STREAMS: u16 = 0x3;
const SETTINGS_INITIAL_WINDOW_SIZE: u16 = 0x4;
const SETTINGS_MAX_FRAME_SIZE: u16 = 0x5;
const SETTINGS_MAX_HEADER_LIST_SIZE: u16 = 0x6;

const MAX_FLOW_CONTROL_WINDOW_SIZE: u32 = (1 << 31) - 1;

/// https://tools.ietf.org/html/rfc7540#section-6.5
#[derive(Debug, Clone)]
pub enum SettingsFrame {
    Syn(Vec<Setting>),
    Ack,
}
impl SettingsFrame {
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

    pub fn from_vec(header: &FrameHeader, payload: Vec<u8>) -> Result<Self> {
        track_assert_eq!(header.stream_id, 0, ErrorKind::ProtocolError);
        if (header.flags & flags::ACK) != 0 {
            track_assert_eq!(payload.len(), 0, ErrorKind::FrameSizeError);
            Ok(SettingsFrame::Ack)
        } else {
            track_assert_eq!(payload.len() % 6, 0, ErrorKind::FrameSizeError);
            let mut settings = Vec::with_capacity(payload.len() / 6);
            for chunk in payload.chunks(6) {
                if let Some(setting) = track!(Setting::from_bytes(chunk))? {
                    settings.push(setting);
                }
            }
            Ok(SettingsFrame::Syn(settings))
        }
    }
}

#[derive(Debug, Clone)]
pub enum Setting {
    HeaderTableSize(u32),
    EnablePush(bool),
    MaxConcurrentStreams(u32),
    InitialWindowSize(u32),
    MaxFrameSize(u32),
    MaxHeaderListSize(u32),
}
impl Setting {
    fn from_bytes(bytes: &[u8]) -> Result<Option<Self>> {
        let id = BigEndian::read_u16(&bytes[0..2]);
        let value = BigEndian::read_u32(&bytes[2..6]);
        Ok(Some(match id {
            SETTINGS_HEADER_TABLE_SIZE => Setting::HeaderTableSize(value),
            SETTINGS_ENABLE_PUSH => {
                track_assert!(value <= 1, ErrorKind::ProtocolError);
                Setting::EnablePush(value == 1)
            }
            SETTINGS_MAX_CONCURRENT_STREAMS => Setting::MaxConcurrentStreams(value),
            SETTINGS_INITIAL_WINDOW_SIZE => {
                track_assert!(
                    value <= MAX_FLOW_CONTROL_WINDOW_SIZE,
                    ErrorKind::FlowControlError
                );
                Setting::InitialWindowSize(value)
            }
            SETTINGS_MAX_FRAME_SIZE => {
                track_assert!(1 << 14 <= value, ErrorKind::ProtocolError);
                track_assert!(value <= 1 << 24 - 1, ErrorKind::ProtocolError);
                Setting::MaxFrameSize(value)
            }
            SETTINGS_MAX_HEADER_LIST_SIZE => Setting::MaxHeaderListSize(value),
            _ => {
                // > An endpoint that receives a SETTINGS frame with any unknown or
                // > unsupported identifier MUST ignore that setting.
                // >
                // > [RFC 7540](https://tools.ietf.org/html/rfc7540#section-6.5.2)
                return Ok(None);
            }
        }))
    }
}
