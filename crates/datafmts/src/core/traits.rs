use crate::{Error, Schema};

pub trait Encoder {
    type Error;

    /// Encode bool in order.
    fn encode_bool(&mut self, v: bool) -> Result<(), Self::Error>;
    /// Encode u8 in order.
    fn encode_u8(&mut self, v: u8) -> Result<(), Self::Error>;
    /// Encode u16 in order.
    fn encode_u16(&mut self, v: u16) -> Result<(), Self::Error>;
    /// Encode u32 in order; also enum variant index.
    fn encode_u32(&mut self, v: u32) -> Result<(), Self::Error>;
    /// Encode u64 in order.
    fn encode_u64(&mut self, v: u64) -> Result<(), Self::Error>;
    /// Encode u128 in order.
    fn encode_u128(&mut self, v: u128) -> Result<(), Self::Error>;
    /// Encode i8 in order.
    fn encode_i8(&mut self, v: i8) -> Result<(), Self::Error>;
    /// Encode i16 in order.
    fn encode_i16(&mut self, v: i16) -> Result<(), Self::Error>;
    /// Encode i32 in order.
    fn encode_i32(&mut self, v: i32) -> Result<(), Self::Error>;
    /// Encode i64 in order.
    fn encode_i64(&mut self, v: i64) -> Result<(), Self::Error>;
    /// Encode i128 in order.
    fn encode_i128(&mut self, v: i128) -> Result<(), Self::Error>;
    /// Encode f32 in order.
    fn encode_f32(&mut self, v: f32) -> Result<(), Self::Error>;
    /// Encode f64 in order.
    fn encode_f64(&mut self, v: f64) -> Result<(), Self::Error>;
    /// Encode char in order.
    fn encode_char(&mut self, v: char) -> Result<(), Self::Error>;
    /// Encode str bytes in order.
    fn encode_str(&mut self, v: &str) -> Result<(), Self::Error>;
    /// Encode unit.
    fn encode_unit(&mut self) -> Result<(), Self::Error>;
    /// Encode None discriminant.
    fn encode_none(&mut self) -> Result<(), Self::Error> {
        self.encode_bool(false)
    }
    /// Encode Some discriminant; caller encodes payload next.
    fn encode_some(&mut self) -> Result<(), Self::Error> {
        self.encode_bool(true)
    }
}

pub trait Decoder {
    type Error;

    /// Decode bool in order.
    fn decode_bool(&mut self) -> Result<bool, Self::Error>;
    /// Decode u8 in order.
    fn decode_u8(&mut self) -> Result<u8, Self::Error>;
    /// Decode u16 in order.
    fn decode_u16(&mut self) -> Result<u16, Self::Error>;
    /// Decode u32 in order; also enum variant index.
    fn decode_u32(&mut self) -> Result<u32, Self::Error>;
    /// Decode u64 in order.
    fn decode_u64(&mut self) -> Result<u64, Self::Error>;
    /// Decode u128 in order.
    fn decode_u128(&mut self) -> Result<u128, Self::Error>;
    /// Decode i8 in order.
    fn decode_i8(&mut self) -> Result<i8, Self::Error>;
    /// Decode i16 in order.
    fn decode_i16(&mut self) -> Result<i16, Self::Error>;
    /// Decode i32 in order.
    fn decode_i32(&mut self) -> Result<i32, Self::Error>;
    /// Decode i64 in order.
    fn decode_i64(&mut self) -> Result<i64, Self::Error>;
    /// Decode i128 in order.
    fn decode_i128(&mut self) -> Result<i128, Self::Error>;
    /// Decode f32 in order.
    fn decode_f32(&mut self) -> Result<f32, Self::Error>;
    /// Decode f64 in order.
    fn decode_f64(&mut self) -> Result<f64, Self::Error>;
    /// Decode char in order.
    fn decode_char(&mut self) -> Result<char, Self::Error>;
    /// Decode str bytes into scratch; Err on overflow.
    fn decode_str_into(&mut self, _into: &mut StrScratch) -> Result<(), Self::Error>;
    /// Decode owned string of arbitrary length.
    #[cfg(feature = "csv")]
    fn decode_string(&mut self) -> Result<alloc::string::String, Self::Error>;
    /// Decode unit.
    fn decode_unit(&mut self) -> Result<(), Self::Error>;
    /// Decode Option discriminant; true = Some.
    fn decode_option_tag(&mut self) -> Result<bool, Self::Error> {
        self.decode_bool()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StrScratch {
    pub len: usize,
    pub buf: [u8; 256],
}

impl StrScratch {
    /// Empty scratch.
    pub const fn new() -> Self {
        Self {
            len: 0,
            buf: [0u8; 256],
        }
    }

    /// View filled prefix as str; Err on invalid UTF-8.
    pub fn as_str(&self) -> Result<&str, Error> {
        core::str::from_utf8(&self.buf[..self.len]).map_err(|_| Error::InvalidValue)
    }

    /// Append bytes; Err on overflow.
    pub fn push_bytes(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if self.len + bytes.len() > self.buf.len() {
            return Err(Error::InvalidValue);
        }
        self.buf[self.len..self.len + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len();
        Ok(())
    }
}

pub trait Encode {
    /// Encode self in declaration order.
    fn encode<E: Encoder + ?Sized>(&self, e: &mut E) -> Result<(), E::Error>;
}

pub trait Decode: Sized {
    /// Decode next value; D::Error must map core Error for InvalidVariant.
    fn decode<D: Decoder + ?Sized>(d: &mut D) -> Result<Self, D::Error>
    where
        D::Error: From<Error>;
}

pub trait DataStruct: Encode + Decode {
    const SCHEMA: &'static Schema;
}
