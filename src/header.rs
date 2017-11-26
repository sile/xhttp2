use std::fmt;

use hpack_codec::Decoder as HpackDecoder;

use Result;

pub struct Header {
    fields: Vec<FieldPosition>,
    buf: Vec<u8>,
}
impl Header {
    pub fn decode(decoder: &mut HpackDecoder, block: &[u8]) -> Result<Self> {
        let mut header = track!(decoder.enter_header_block(block))?;
        let mut fields = Vec::new();
        let mut buf = Vec::new();
        while let Some(field) = track!(header.decode_field())? {
            let name_offset = buf.len();
            buf.extend_from_slice(field.name());

            let value_offset = buf.len();
            buf.extend_from_slice(field.value());

            fields.push(FieldPosition {
                name_offset,
                value_offset,
            });
        }
        Ok(Header { fields, buf })
    }
    pub fn fields(&self) -> Fields {
        Fields {
            index: 0,
            header: self,
        }
    }
    fn get_field(&self, index: usize) -> Option<(&[u8], &[u8])> {
        if let Some(field) = self.fields.get(index) {
            let next_name_offset = self.fields.get(index + 1).map_or(
                self.buf.len(),
                |f| f.name_offset,
            );
            let name = &self.buf[field.name_offset..field.value_offset];
            let value = &self.buf[field.value_offset..next_name_offset];
            Some((name, value))
        } else {
            None
        }
    }
}
impl fmt::Debug for Header {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use std::str;

        write!(f, "Header {{")?;
        for (i, field) in self.fields().enumerate() {
            if i != 0 {
                write!(f, ", ")?;
            }
            if let Ok(name) = str::from_utf8(field.0) {
                write!(f, "b{:?}: ", name)?;
            } else {
                write!(f, "{:?}: ", field.0)?;
            }
            if let Ok(value) = str::from_utf8(field.1) {
                write!(f, "b{:?}", value)?;
            } else {
                write!(f, "{:?}", field.1)?;
            }
        }
        write!(f, "}}")?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct FieldPosition {
    name_offset: usize,
    value_offset: usize,
}

#[derive(Debug)]
pub struct Fields<'a> {
    index: usize,
    header: &'a Header,
}
impl<'a> Iterator for Fields<'a> {
    type Item = (&'a [u8], &'a [u8]);
    fn next(&mut self) -> Option<Self::Item> {
        if let Some(field) = self.header.get_field(self.index) {
            self.index += 1;
            Some(field)
        } else {
            None
        }
    }
}
