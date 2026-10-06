use datafmts::{DataStruct, Decode, Encode};

// ---- unit / tuple ----
#[derive(DataStruct, Debug, PartialEq)]
struct Unit;

#[derive(DataStruct, Debug, PartialEq)]
struct Tuple(i32, bool);

// ---- enum ----
#[derive(DataStruct, Debug, PartialEq)]
enum E {
    A,
    B(i32),
    C { x: i32 },
}

// ---- generics ----
#[derive(DataStruct, Debug, PartialEq)]
struct Wrap<T> {
    v: T,
}

// ---- rename ----
#[derive(DataStruct, Debug, PartialEq)]
#[datafmt(rename_all = "camelCase")]
struct Renamed {
    foo_bar: i32,
    #[datafmt(rename = "X")]
    y: i32,
}

// ---- default / skip ----
#[derive(DataStruct, Debug, PartialEq)]
struct WithDefault {
    a: i32,
    #[datafmt(default)]
    b: i32,
}

#[derive(DataStruct, Debug, PartialEq)]
struct WithSkip {
    a: i32,
    #[datafmt(skip)]
    skipped: i32,
}

// ---- stub format supporting bool/u32/i32 ----
#[derive(Debug, Clone, PartialEq)]
enum Val {
    Bool(bool),
    U32(u32),
    I32(i32),
}

struct VecEnc {
    out: Vec<Val>,
}
macro_rules! enc_unimpl {
    ($($m:ident($t:ty)),*) => {$(
        fn $m(&mut self, _v: $t) -> Result<(), Self::Error> {
            unimplemented!()
        }
    )*};
}
impl datafmts::Encoder for VecEnc {
    type Error = datafmts::Error;
    fn encode_bool(&mut self, v: bool) -> Result<(), Self::Error> {
        self.out.push(Val::Bool(v));
        Ok(())
    }
    fn encode_u32(&mut self, v: u32) -> Result<(), Self::Error> {
        self.out.push(Val::U32(v));
        Ok(())
    }
    fn encode_i32(&mut self, v: i32) -> Result<(), Self::Error> {
        self.out.push(Val::I32(v));
        Ok(())
    }
    enc_unimpl!(
        encode_u8(u8),
        encode_u16(u16),
        encode_u64(u64),
        encode_u128(u128),
        encode_i8(i8),
        encode_i16(i16),
        encode_i64(i64),
        encode_i128(i128),
        encode_f32(f32),
        encode_f64(f64),
        encode_char(char)
    );
    fn encode_str(&mut self, _v: &str) -> Result<(), Self::Error> {
        unimplemented!()
    }
    fn encode_unit(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

struct SliceDec<'a> {
    inp: &'a [Val],
    pos: usize,
}
macro_rules! dec_unimpl {
    ($($m:ident($t:ty)),*) => {$(
        fn $m(&mut self) -> Result<$t, Self::Error> {
            unimplemented!()
        }
    )*};
}
impl<'a> datafmts::Decoder for SliceDec<'a> {
    type Error = datafmts::Error;
    fn decode_bool(&mut self) -> Result<bool, Self::Error> {
        match self.inp.get(self.pos) {
            Some(Val::Bool(b)) => {
                self.pos += 1;
                Ok(*b)
            }
            _ => Err(datafmts::Error::InvalidValue),
        }
    }
    fn decode_u32(&mut self) -> Result<u32, Self::Error> {
        match self.inp.get(self.pos) {
            Some(Val::U32(v)) => {
                self.pos += 1;
                Ok(*v)
            }
            _ => Err(datafmts::Error::InvalidValue),
        }
    }
    fn decode_i32(&mut self) -> Result<i32, Self::Error> {
        match self.inp.get(self.pos) {
            Some(Val::I32(v)) => {
                self.pos += 1;
                Ok(*v)
            }
            None => Err(datafmts::Error::Eof),
            _ => Err(datafmts::Error::InvalidValue),
        }
    }
    dec_unimpl!(
        decode_u8(u8),
        decode_u16(u16),
        decode_u64(u64),
        decode_u128(u128),
        decode_i8(i8),
        decode_i16(i16),
        decode_i64(i64),
        decode_i128(i128),
        decode_f32(f32),
        decode_f64(f64),
        decode_char(char)
    );
    fn decode_str_into(&mut self, _into: &mut datafmts::StrScratch) -> Result<(), Self::Error> {
        unimplemented!()
    }
    #[cfg(feature = "csv")]
    fn decode_string(&mut self) -> Result<String, Self::Error> {
        unimplemented!()
    }
    fn decode_unit(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

fn roundtrip<T: Encode + Decode + PartialEq + core::fmt::Debug>(v: T) -> T {
    // Encode needs Encoder, decode needs Decoder — build both via Val vectors.
    // We encode via VecEnc then decode via SliceDec.
    let mut enc = VecEnc { out: Vec::new() };
    Encode::encode(&v, &mut enc).unwrap();
    let mut dec = SliceDec {
        inp: &enc.out,
        pos: 0,
    };
    let out = T::decode(&mut dec).unwrap();
    assert_eq!(out, v);
    out
}

#[test]
fn unit_tuple_schema_and_roundtrip() {
    assert!(matches!(
        Unit::SCHEMA.kind,
        datafmts::Kind::Struct(s) if s.kind == datafmts::StructKind::Unit
    ));
    roundtrip(Unit);
    match Tuple::SCHEMA.kind {
        datafmts::Kind::Struct(s) => {
            assert_eq!(s.kind, datafmts::StructKind::Tuple);
            assert_eq!(s.fields.len(), 2);
        }
        _ => panic!("tuple struct"),
    }
    roundtrip(Tuple(7, true));
}

#[test]
fn enum_roundtrip_and_schema() {
    match E::SCHEMA.kind {
        datafmts::Kind::Enum(e) => {
            assert_eq!(e.variants.len(), 3);
            assert_eq!(e.variants[0].name, "A");
            assert_eq!(e.variants[1].name, "B");
            assert_eq!(e.variants[2].name, "C");
        }
        _ => panic!("enum"),
    }
    roundtrip(E::A);
    roundtrip(E::B(5));
    roundtrip(E::C { x: 9 });

    // invalid variant -> InvalidVariant
    let mut dec = SliceDec {
        inp: &[Val::U32(99)],
        pos: 0,
    };
    let r = E::decode(&mut dec);
    assert_eq!(r, Err(datafmts::Error::InvalidVariant(99)));
}

#[test]
fn generics_rename_default_skip() {
    // generics
    assert_eq!(Wrap::<i32>::SCHEMA.name, "Wrap");
    roundtrip(Wrap { v: 3i32 });

    // rename_all camelCase + explicit rename
    match Renamed::SCHEMA.kind {
        datafmts::Kind::Struct(s) => {
            assert_eq!(s.fields[0].name, "fooBar");
            assert_eq!(s.fields[1].name, "X");
        }
        _ => panic!("renamed"),
    }

    // default: missing trailing field uses Default
    let mut dec = SliceDec {
        inp: &[Val::I32(1)],
        pos: 0,
    };
    let w = WithDefault::decode(&mut dec).unwrap();
    assert_eq!(w, WithDefault { a: 1, b: 0 });

    // skip: not in schema, not on wire, defaults on decode
    match WithSkip::SCHEMA.kind {
        datafmts::Kind::Struct(s) => {
            assert_eq!(s.fields.len(), 1);
            assert_eq!(s.fields[0].name, "a");
        }
        _ => panic!("skip schema"),
    }
    let mut enc = VecEnc { out: Vec::new() };
    Encode::encode(&WithSkip { a: 4, skipped: 999 }, &mut enc).unwrap();
    assert_eq!(enc.out, vec![Val::I32(4)]);
    let mut dec = SliceDec {
        inp: &[Val::I32(4)],
        pos: 0,
    };
    let ws = WithSkip::decode(&mut dec).unwrap();
    assert_eq!(ws.a, 4);
    assert_eq!(ws.skipped, 0);
}
