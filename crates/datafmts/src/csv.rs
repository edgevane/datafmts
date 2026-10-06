use alloc::{format, string::String, vec::Vec};

use crate::{
    Decode, Decoder, Encode, Encoder, Error, Field, Kind, Primitive, Schema, StrScratch,
};

// String impls live with csv: both need alloc
impl Encode for String {
    fn encode<__Enc: Encoder + ?Sized>(&self, e: &mut __Enc) -> Result<(), __Enc::Error> {
        e.encode_str(self)
    }
}

impl Decode for String {
    fn decode<__Dec: Decoder + ?Sized>(d: &mut __Dec) -> Result<Self, __Dec::Error>
    where
        __Dec::Error: From<Error>,
    {
        d.decode_string()
    }
}

impl crate::DataStruct for String {
    const SCHEMA: &'static Schema = &Schema {
        name: "String",
        kind: Kind::Primitive(Primitive::Str),
    };
}

struct CsvEncoder {
    cells: Vec<String>,
}

macro_rules! enc_cell {
    ($($m:ident($t:ty)),*) => {$(
        fn $m(&mut self, v: $t) -> Result<(), Self::Error> {
            self.cells.push(format!("{v}"));
            Ok(())
        }
    )*};
}

impl Encoder for CsvEncoder {
    type Error = Error;

    enc_cell!(
        encode_bool(bool),
        encode_u8(u8),
        encode_u16(u16),
        encode_u32(u32),
        encode_u64(u64),
        encode_u128(u128),
        encode_i8(i8),
        encode_i16(i16),
        encode_i32(i32),
        encode_i64(i64),
        encode_i128(i128),
        encode_f32(f32),
        encode_f64(f64)
    );

    fn encode_char(&mut self, v: char) -> Result<(), Self::Error> {
        self.cells.push(format!("{v}"));
        Ok(())
    }

    fn encode_str(&mut self, v: &str) -> Result<(), Self::Error> {
        self.cells.push(String::from(v));
        Ok(())
    }

    // unit contributes no cell; () fields rejected by flat check
    fn encode_unit(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    // None is one empty cell; Some pushes nothing, payload is the cell
    fn encode_none(&mut self) -> Result<(), Self::Error> {
        self.cells.push(String::new());
        Ok(())
    }

    fn encode_some(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct CsvDecoder<'a> {
    cells: &'a [String],
    pos: usize,
}

impl<'a> CsvDecoder<'a> {
    fn next(&mut self) -> Result<&'a str, Error> {
        let c = self.cells.get(self.pos).ok_or(Error::Eof)?;
        self.pos += 1;
        Ok(c.as_str())
    }
}

macro_rules! dec_cell {
    ($($m:ident($t:ty)),*) => {$(
        fn $m(&mut self) -> Result<$t, Self::Error> {
            self.next()?.parse::<$t>().map_err(|_| Error::InvalidValue)
        }
    )*};
}

impl<'a> Decoder for CsvDecoder<'a> {
    type Error = Error;

    dec_cell!(
        decode_bool(bool),
        decode_u8(u8),
        decode_u16(u16),
        decode_u32(u32),
        decode_u64(u64),
        decode_u128(u128),
        decode_i8(i8),
        decode_i16(i16),
        decode_i32(i32),
        decode_i64(i64),
        decode_i128(i128),
        decode_f32(f32),
        decode_f64(f64),
        decode_char(char)
    );

    fn decode_str_into(&mut self, into: &mut StrScratch) -> Result<(), Self::Error> {
        let s = self.next()?;
        *into = StrScratch::new();
        into.push_bytes(s.as_bytes())
    }

    fn decode_string(&mut self) -> Result<String, Self::Error> {
        Ok(String::from(self.next()?))
    }

    // unit contributes no cell
    fn decode_unit(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    // empty cell is None (consumed); non-empty peeks for payload decode
    fn decode_option_tag(&mut self) -> Result<bool, Self::Error> {
        match self.cells.get(self.pos) {
            None => Err(Error::Eof),
            Some(c) if c.is_empty() => {
                self.pos += 1;
                Ok(false)
            }
            Some(_) => Ok(true),
        }
    }
}

fn parse_csv(input: &[u8]) -> Result<Vec<Vec<String>>, Error> {
    let s = core::str::from_utf8(input).map_err(|_| Error::InvalidValue)?;
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut chars = s.chars().peekable();
    // true once current line contributed any char, delimiter or quoted field
    let mut line_used = false;
    let mut first = true;

    while let Some(c) = chars.next() {
        first = false;
        match c {
            ',' => {
                row.push(core::mem::replace(&mut cell, String::new()));
                line_used = true;
            }
            '\n' | '\r' => {
                if c == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                if line_used || !cell.is_empty() || !row.is_empty() {
                    row.push(core::mem::replace(&mut cell, String::new()));
                    rows.push(core::mem::replace(&mut row, Vec::new()));
                }
                line_used = false;
            }
            '"' if cell.is_empty() => {
                line_used = true;
                loop {
                    match chars.next() {
                        None => return Err(Error::InvalidValue),
                        Some('"') if chars.peek() == Some(&'"') => {
                            chars.next();
                            cell.push('"');
                        }
                        Some('"') => break,
                        Some(q) => cell.push(q),
                    }
                }
                match chars.peek() {
                    Some(&',') | Some(&'\n') | Some(&'\r') | None => {}
                    Some(_) => return Err(Error::InvalidValue),
                }
            }
            _ => {
                line_used = true;
                cell.push(c);
            }
        }
    }
    if !first && (line_used || !cell.is_empty() || !row.is_empty()) {
        row.push(cell);
        rows.push(row);
    }
    Ok(rows)
}

fn struct_fields(schema: &'static Schema) -> Result<&'static [Field], Error> {
    match &schema.kind {
        Kind::Struct(s) => Ok(s.fields),
        _ => Err(Error::Custom("csv: top-level must be struct")),
    }
}

// flat csv cell: primitive (not unit) or option thereof
fn check_flat(field: &Field) -> Result<(), Error> {
    let flat = match &field.schema().kind {
        Kind::Primitive(p) => *p != Primitive::Unit,
        Kind::Option(inner) => matches!(inner().kind, Kind::Primitive(p) if p != Primitive::Unit),
        _ => false,
    };
    if flat {
        Ok(())
    } else {
        Err(Error::Custom("csv: nested types unsupported"))
    }
}

fn push_escaped(out: &mut Vec<u8>, cell: &str, first_in_row: bool) {
    if !first_in_row {
        out.push(b',');
    }
    let needs = cell.bytes().any(|b| b == b'"' || b == b',' || b == b'\n' || b == b'\r');
    if !needs {
        out.extend_from_slice(cell.as_bytes());
        return;
    }
    out.push(b'"');
    for ch in cell.chars() {
        if ch == '"' {
            out.extend_from_slice(b"\"\"");
        } else {
            let mut b = [0u8; 4];
            out.extend_from_slice(ch.encode_utf8(&mut b).as_bytes());
        }
    }
    out.push(b'"');
}

/// Read one struct from CSV bytes; header must match schema field names.
pub fn read<T: crate::DataStruct>(input: &[u8]) -> Result<T, Error> {
    let fields = struct_fields(T::SCHEMA)?;
    for f in fields {
        check_flat(f)?;
    }
    let expected: Vec<&str> = fields.iter().map(|f| f.name).collect();

    let rows = parse_csv(input)?;
    if rows.is_empty() {
        if expected.is_empty() {
            let mut d = CsvDecoder {
                cells: &[],
                pos: 0,
            };
            return T::decode(&mut d);
        }
        return Err(Error::Eof);
    }
    if rows[0].iter().map(String::as_str).ne(expected.iter().copied()) {
        return Err(Error::InvalidValue);
    }
    let data: &[String] = rows.get(1).map(Vec::as_slice).unwrap_or(&[]);
    let mut d = CsvDecoder {
        cells: data,
        pos: 0,
    };
    let v = T::decode(&mut d)?;
    if d.pos != data.len() {
        return Err(Error::InvalidValue);
    }
    Ok(v)
}

/// Write one struct as CSV bytes; header from schema field names.
pub fn write<T: crate::DataStruct>(value: &T) -> Result<Vec<u8>, Error> {
    let fields = struct_fields(T::SCHEMA)?;
    for f in fields {
        check_flat(f)?;
    }
    let mut enc = CsvEncoder { cells: Vec::new() };
    value.encode(&mut enc)?;
    if enc.cells.len() != fields.len() {
        return Err(Error::Custom("csv: cell mismatch"));
    }
    let mut out = Vec::new();
    for (i, f) in fields.iter().enumerate() {
        push_escaped(&mut out, f.name, i == 0);
    }
    out.push(b'\n');
    for (i, c) in enc.cells.iter().enumerate() {
        push_escaped(&mut out, c, i == 0);
    }
    out.push(b'\n');
    Ok(out)
}
