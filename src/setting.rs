use byteorder::{BigEndian, ByteOrder};

use {Result, ErrorKind};

const SETTINGS_HEADER_TABLE_SIZE: u16 = 0x1;
const SETTINGS_ENABLE_PUSH: u16 = 0x2;
const SETTINGS_MAX_CONCURRENT_STREAMS: u16 = 0x3;
const SETTINGS_INITIAL_WINDOW_SIZE: u16 = 0x4;
const SETTINGS_MAX_FRAME_SIZE: u16 = 0x5;
const SETTINGS_MAX_HEADER_LIST_SIZE: u16 = 0x6;

const MAX_FLOW_CONTROL_WINDOW_SIZE: u32 = (1 << 31) - 1;

#[derive(Debug)]
pub struct Settings {
    pub header_table_size: u32,
    pub enable_push: bool,
    pub max_concurrent_streams: Option<u32>, // `None` means infinite
    pub initial_window_size: u32,
    pub max_frame_size: u32,
    pub max_header_list_size: Option<u32>, // `None` means infinite
}
impl Default for Settings {
    fn default() -> Self {
        // https://tools.ietf.org/html/rfc7540#section-11.3
        Settings {
            header_table_size: 4096,
            enable_push: true,
            max_concurrent_streams: None,
            initial_window_size: 65535,
            max_frame_size: 16384,
            max_header_list_size: None,
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
    pub fn from_bytes(bytes: [u8; 6]) -> Result<Option<Self>> {
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
    pub fn to_bytes(&self) -> [u8; 6] {
        let convert = |kind, value| {
            let mut bytes = [0; 6];
            BigEndian::write_u16(&mut bytes[0..2], kind);
            BigEndian::write_u32(&mut bytes[2..6], value);
            bytes
        };
        match *self {
            Setting::HeaderTableSize(v) => convert(SETTINGS_HEADER_TABLE_SIZE, v),
            Setting::EnablePush(v) => convert(SETTINGS_ENABLE_PUSH, v as u32),
            Setting::MaxConcurrentStreams(v) => convert(SETTINGS_MAX_CONCURRENT_STREAMS, v),
            Setting::InitialWindowSize(v) => convert(SETTINGS_INITIAL_WINDOW_SIZE, v),
            Setting::MaxFrameSize(v) => convert(SETTINGS_MAX_FRAME_SIZE, v),
            Setting::MaxHeaderListSize(v) => convert(SETTINGS_MAX_HEADER_LIST_SIZE, v),
        }
    }
}
