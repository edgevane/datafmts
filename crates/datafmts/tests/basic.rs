use datafmts::{DataStruct, Decode, Encode};

// Wished API v1: single derive gives Encode + Decode + Schema.
#[derive(DataStruct)]
struct Point {
    x: i32,
    y: i32,
}

// Lowercase alias from user request: #[derive(data_struct)]
#[derive(datafmts::data_struct)]
struct PointSnake {
    x: i32,
    y: i32,
}

#[test]
fn point_schema_has_two_int_fields() {
    let schema = Point::SCHEMA;
    assert_eq!(schema.name, "Point");
    match schema.kind {
        datafmts::Kind::Struct(s) => {
            assert_eq!(s.fields.len(), 2);
            assert_eq!(s.fields[0].name, "x");
            assert_eq!(s.fields[1].name, "y");
        }
        _ => panic!("expected struct schema"),
    }
}

#[test]
fn point_encode_decode_stub_roundtrip() {
    // Stub format: pushes i32 fields in order, no_std compatible.
    struct VecEnc {
        out: Vec<i32>,
    }
    impl datafmts::Encoder for VecEnc {
        type Error = datafmts::Error;
        fn encode_bool(&mut self, _v: bool) -> Result<(), Self::Error> {
            unimplemented!()
        }
        fn encode_u8(&mut self, _v: u8) -> Result<(), Self::Error> {
            unimplemented!()
        }
        fn encode_u16(&mut self, _v: u16) -> Result<(), Self::Error> {
            unimplemented!()
        }
        fn encode_u32(&mut self, _v: u32) -> Result<(), Self::Error> {
            unimplemented!()
        }
        fn encode_u64(&mut self, _v: u64) -> Result<(), Self::Error> {
            unimplemented!()
        }
        fn encode_u128(&mut self, _v: u128) -> Result<(), Self::Error> {
            unimplemented!()
        }
        fn encode_i8(&mut self, _v: i8) -> Result<(), Self::Error> {
            unimplemented!()
        }
        fn encode_i16(&mut self, _v: i16) -> Result<(), Self::Error> {
            unimplemented!()
        }
        fn encode_i32(&mut self, v: i32) -> Result<(), Self::Error> {
            self.out.push(v);
            Ok(())
        }
        fn encode_i64(&mut self, _v: i64) -> Result<(), Self::Error> {
            unimplemented!()
        }
        fn encode_i128(&mut self, _v: i128) -> Result<(), Self::Error> {
            unimplemented!()
        }
        fn encode_f32(&mut self, _v: f32) -> Result<(), Self::Error> {
            unimplemented!()
        }
        fn encode_f64(&mut self, _v: f64) -> Result<(), Self::Error> {
            unimplemented!()
        }
        fn encode_char(&mut self, _v: char) -> Result<(), Self::Error> {
            unimplemented!()
        }
        fn encode_str(&mut self, _v: &str) -> Result<(), Self::Error> {
            unimplemented!()
        }
        fn encode_unit(&mut self) -> Result<(), Self::Error> {
            unimplemented!()
        }
    }
    struct SliceDec<'a> {
        inp: &'a [i32],
        pos: usize,
    }
    impl<'a> datafmts::Decoder for SliceDec<'a> {
        type Error = datafmts::Error;
        fn decode_bool(&mut self) -> Result<bool, Self::Error> {
            unimplemented!()
        }
        fn decode_u8(&mut self) -> Result<u8, Self::Error> {
            unimplemented!()
        }
        fn decode_u16(&mut self) -> Result<u16, Self::Error> {
            unimplemented!()
        }
        fn decode_u32(&mut self) -> Result<u32, Self::Error> {
            unimplemented!()
        }
        fn decode_u64(&mut self) -> Result<u64, Self::Error> {
            unimplemented!()
        }
        fn decode_u128(&mut self) -> Result<u128, Self::Error> {
            unimplemented!()
        }
        fn decode_i8(&mut self) -> Result<i8, Self::Error> {
            unimplemented!()
        }
        fn decode_i16(&mut self) -> Result<i16, Self::Error> {
            unimplemented!()
        }
        fn decode_i32(&mut self) -> Result<i32, Self::Error> {
            let v = *self.inp.get(self.pos).ok_or(datafmts::Error::Eof)?;
            self.pos += 1;
            Ok(v)
        }
        fn decode_i64(&mut self) -> Result<i64, Self::Error> {
            unimplemented!()
        }
        fn decode_i128(&mut self) -> Result<i128, Self::Error> {
            unimplemented!()
        }
        fn decode_f32(&mut self) -> Result<f32, Self::Error> {
            unimplemented!()
        }
        fn decode_f64(&mut self) -> Result<f64, Self::Error> {
            unimplemented!()
        }
        fn decode_char(&mut self) -> Result<char, Self::Error> {
            unimplemented!()
        }
        fn decode_str_into(&mut self, _into: &mut datafmts::StrScratch) -> Result<(), Self::Error> {
            unimplemented!()
        }
        #[cfg(feature = "csv")]
        fn decode_string(&mut self) -> Result<String, Self::Error> {
            unimplemented!()
        }
        fn decode_unit(&mut self) -> Result<(), Self::Error> {
            unimplemented!()
        }
    }

    let p = Point { x: 1, y: 2 };
    let mut enc = VecEnc { out: Vec::new() };
    Encode::encode(&p, &mut enc).unwrap();
    assert_eq!(enc.out, vec![1, 2]);

    let mut dec = SliceDec {
        inp: &enc.out,
        pos: 0,
    };
    let q = Point::decode(&mut dec).unwrap();
    assert_eq!(q.x, 1);
    assert_eq!(q.y, 2);

    // snake alias also works
    let s = PointSnake::SCHEMA;
    assert_eq!(s.name, "PointSnake");
}
