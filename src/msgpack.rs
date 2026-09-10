#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
    Nil,
    Bool(bool),
    Integer(i64),
    Float(f64),
    String(&'a str),
    Binary(&'a [u8]),
    Array(Vec<Value<'a>>),
    Map(Vec<(Value<'a>, Value<'a>)>),
    Extension(i8, &'a [u8]),
}

impl<'a> Value<'a> {
    pub fn as_str(&self) -> Option<&'a str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Float(f) => Some(*f),
            Value::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&'a [u8]> {
        match self {
            Value::Binary(b) => Some(b),
            Value::String(s) => Some(s.as_bytes()),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value<'a>]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_geom_bytes(&self) -> Option<&'a [u8]> {
        match self {
            Value::Extension(71, data) => Some(data), // 71 == 'G'
            Value::Binary(b) => Some(b),
            _ => None,
        }
    }
}

pub struct Decoder<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Decoder<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        Self { buf, pos: 0 }
    }

    pub fn remaining(&self) -> usize {
        self.buf.len().saturating_sub(self.pos)
    }

    fn read_u8(&mut self) -> Result<u8, String> {
        if self.pos >= self.buf.len() {
            return Err("Unexpected EOF reading u8".into());
        }
        let b = self.buf[self.pos];
        self.pos += 1;
        Ok(b)
    }

    fn read_slice(&mut self, len: usize) -> Result<&'a [u8], String> {
        if self.pos + len > self.buf.len() {
            return Err(format!("Unexpected EOF reading slice of len {}", len));
        }
        let s = &self.buf[self.pos..self.pos + len];
        self.pos += len;
        Ok(s)
    }

    fn read_u16(&mut self) -> Result<u16, String> {
        let s = self.read_slice(2)?;
        Ok(u16::from_be_bytes([s[0], s[1]]))
    }

    fn read_u32(&mut self) -> Result<u32, String> {
        let s = self.read_slice(4)?;
        Ok(u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    }

    fn read_u64(&mut self) -> Result<u64, String> {
        let s = self.read_slice(8)?;
        Ok(u64::from_be_bytes([
            s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7],
        ]))
    }

    fn read_i8(&mut self) -> Result<i8, String> {
        Ok(self.read_u8()? as i8)
    }

    fn read_i16(&mut self) -> Result<i16, String> {
        let s = self.read_slice(2)?;
        Ok(i16::from_be_bytes([s[0], s[1]]))
    }

    fn read_i32(&mut self) -> Result<i32, String> {
        let s = self.read_slice(4)?;
        Ok(i32::from_be_bytes([s[0], s[1], s[2], s[3]]))
    }

    fn read_i64(&mut self) -> Result<i64, String> {
        let s = self.read_slice(8)?;
        Ok(i64::from_be_bytes([
            s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7],
        ]))
    }

    pub fn decode_value(&mut self) -> Result<Value<'a>, String> {
        let tag = self.read_u8()?;
        match tag {
            0x00..=0x7f => Ok(Value::Integer(tag as i64)),
            0x80..=0x8f => {
                let count = (tag & 0x0f) as usize;
                let mut map = Vec::with_capacity(count);
                for _ in 0..count {
                    let k = self.decode_value()?;
                    let v = self.decode_value()?;
                    map.push((k, v));
                }
                Ok(Value::Map(map))
            }
            0x90..=0x9f => {
                let count = (tag & 0x0f) as usize;
                let mut arr = Vec::with_capacity(count);
                for _ in 0..count {
                    arr.push(self.decode_value()?);
                }
                Ok(Value::Array(arr))
            }
            0xa0..=0xbf => {
                let len = (tag & 0x1f) as usize;
                let bytes = self.read_slice(len)?;
                let s = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
                Ok(Value::String(s))
            }
            0xc0 => Ok(Value::Nil),
            0xc2 => Ok(Value::Bool(false)),
            0xc3 => Ok(Value::Bool(true)),
            0xc4 => {
                let len = self.read_u8()? as usize;
                Ok(Value::Binary(self.read_slice(len)?))
            }
            0xc5 => {
                let len = self.read_u16()? as usize;
                Ok(Value::Binary(self.read_slice(len)?))
            }
            0xc6 => {
                let len = self.read_u32()? as usize;
                Ok(Value::Binary(self.read_slice(len)?))
            }
            0xc7 => {
                let len = self.read_u8()? as usize;
                let ext_type = self.read_i8()?;
                Ok(Value::Extension(ext_type, self.read_slice(len)?))
            }
            0xc8 => {
                let len = self.read_u16()? as usize;
                let ext_type = self.read_i8()?;
                Ok(Value::Extension(ext_type, self.read_slice(len)?))
            }
            0xc9 => {
                let len = self.read_u32()? as usize;
                let ext_type = self.read_i8()?;
                Ok(Value::Extension(ext_type, self.read_slice(len)?))
            }
            0xca => {
                let s = self.read_slice(4)?;
                let f = f32::from_be_bytes([s[0], s[1], s[2], s[3]]);
                Ok(Value::Float(f as f64))
            }
            0xcb => {
                let s = self.read_slice(8)?;
                let f = f64::from_be_bytes([s[0], s[1], s[2], s[3], s[4], s[5], s[6], s[7]]);
                Ok(Value::Float(f))
            }
            0xcc => Ok(Value::Integer(self.read_u8()? as i64)),
            0xcd => Ok(Value::Integer(self.read_u16()? as i64)),
            0xce => Ok(Value::Integer(self.read_u32()? as i64)),
            0xcf => {
                let val = self.read_u64()?;
                Ok(Value::Integer(val as i64))
            }
            0xd0 => Ok(Value::Integer(self.read_i8()? as i64)),
            0xd1 => Ok(Value::Integer(self.read_i16()? as i64)),
            0xd2 => Ok(Value::Integer(self.read_i32()? as i64)),
            0xd3 => Ok(Value::Integer(self.read_i64()?)),
            0xd4 => {
                let ext_type = self.read_i8()?;
                Ok(Value::Extension(ext_type, self.read_slice(1)?))
            }
            0xd5 => {
                let ext_type = self.read_i8()?;
                Ok(Value::Extension(ext_type, self.read_slice(2)?))
            }
            0xd6 => {
                let ext_type = self.read_i8()?;
                Ok(Value::Extension(ext_type, self.read_slice(4)?))
            }
            0xd7 => {
                let ext_type = self.read_i8()?;
                Ok(Value::Extension(ext_type, self.read_slice(8)?))
            }
            0xd8 => {
                let ext_type = self.read_i8()?;
                Ok(Value::Extension(ext_type, self.read_slice(16)?))
            }
            0xd9 => {
                let len = self.read_u8()? as usize;
                let bytes = self.read_slice(len)?;
                let s = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
                Ok(Value::String(s))
            }
            0xda => {
                let len = self.read_u16()? as usize;
                let bytes = self.read_slice(len)?;
                let s = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
                Ok(Value::String(s))
            }
            0xdb => {
                let len = self.read_u32()? as usize;
                let bytes = self.read_slice(len)?;
                let s = std::str::from_utf8(bytes).map_err(|e| e.to_string())?;
                Ok(Value::String(s))
            }
            0xdc => {
                let count = self.read_u16()? as usize;
                let mut arr = Vec::with_capacity(count);
                for _ in 0..count {
                    arr.push(self.decode_value()?);
                }
                Ok(Value::Array(arr))
            }
            0xdd => {
                let count = self.read_u32()? as usize;
                let mut arr = Vec::with_capacity(count);
                for _ in 0..count {
                    arr.push(self.decode_value()?);
                }
                Ok(Value::Array(arr))
            }
            0xde => {
                let count = self.read_u16()? as usize;
                let mut map = Vec::with_capacity(count);
                for _ in 0..count {
                    let k = self.decode_value()?;
                    let v = self.decode_value()?;
                    map.push((k, v));
                }
                Ok(Value::Map(map))
            }
            0xdf => {
                let count = self.read_u32()? as usize;
                let mut map = Vec::with_capacity(count);
                for _ in 0..count {
                    let k = self.decode_value()?;
                    let v = self.decode_value()?;
                    map.push((k, v));
                }
                Ok(Value::Map(map))
            }
            0xe0..=0xff => Ok(Value::Integer(tag as i8 as i64)),
            _ => Err(format!("Unsupported msgpack tag: 0x{:02x}", tag)),
        }
    }
}

pub fn decode(buf: &[u8]) -> Result<Value<'_>, String> {
    let mut decoder = Decoder::new(buf);
    decoder.decode_value()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_primitives() {
        assert_eq!(decode(&[0x05]).unwrap(), Value::Integer(5));
        assert_eq!(decode(&[0xff]).unwrap(), Value::Integer(-1));
        assert_eq!(decode(&[0xc0]).unwrap(), Value::Nil);
        assert_eq!(decode(&[0xc2]).unwrap(), Value::Bool(false));
        assert_eq!(decode(&[0xc3]).unwrap(), Value::Bool(true));
        assert_eq!(decode(&[0xa5, b'h', b'e', b'l', b'l', b'o']).unwrap(), Value::String("hello"));
    }

    #[test]
    fn test_array() {
        // [1, "a"]
        assert_eq!(
            decode(&[0x92, 0x01, 0xa1, b'a']).unwrap(),
            Value::Array(vec![Value::Integer(1), Value::String("a")])
        );
    }
}
