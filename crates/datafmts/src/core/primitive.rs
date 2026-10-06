use crate::{Decode, Decoder, Encode, Encoder, Error, Kind, Primitive, Schema};

macro_rules! leaf {
    ($ty:ty, $prim:expr, $enc:ident, $dec:ident, $schema_name:expr) => {
        impl Encode for $ty {
            fn encode<E: Encoder + ?Sized>(&self, e: &mut E) -> Result<(), E::Error> {
                e.$enc(*self)
            }
        }
        impl Decode for $ty {
            fn decode<D: Decoder + ?Sized>(d: &mut D) -> Result<Self, D::Error>
            where
                D::Error: From<Error>,
            {
                d.$dec()
            }
        }
        impl crate::DataStruct for $ty {
            const SCHEMA: &'static Schema = &Schema {
                name: $schema_name,
                kind: Kind::Primitive($prim),
            };
        }
    };
}

leaf!(bool, Primitive::Bool, encode_bool, decode_bool, "bool");
leaf!(u8, Primitive::U8, encode_u8, decode_u8, "u8");
leaf!(u16, Primitive::U16, encode_u16, decode_u16, "u16");
leaf!(u32, Primitive::U32, encode_u32, decode_u32, "u32");
leaf!(u64, Primitive::U64, encode_u64, decode_u64, "u64");
leaf!(u128, Primitive::U128, encode_u128, decode_u128, "u128");
leaf!(i8, Primitive::I8, encode_i8, decode_i8, "i8");
leaf!(i16, Primitive::I16, encode_i16, decode_i16, "i16");
leaf!(i32, Primitive::I32, encode_i32, decode_i32, "i32");
leaf!(i64, Primitive::I64, encode_i64, decode_i64, "i64");
leaf!(i128, Primitive::I128, encode_i128, decode_i128, "i128");
leaf!(f32, Primitive::F32, encode_f32, decode_f32, "f32");
leaf!(f64, Primitive::F64, encode_f64, decode_f64, "f64");
leaf!(char, Primitive::Char, encode_char, decode_char, "char");

impl Encode for () {
    fn encode<E: Encoder + ?Sized>(&self, e: &mut E) -> Result<(), E::Error> {
        e.encode_unit()
    }
}
impl Decode for () {
    fn decode<D: Decoder + ?Sized>(d: &mut D) -> Result<Self, D::Error>
    where
        D::Error: From<Error>,
    {
        d.decode_unit()
    }
}
impl crate::DataStruct for () {
    const SCHEMA: &'static Schema = &Schema {
        name: "()",
        kind: Kind::Primitive(Primitive::Unit),
    };
}

// Encode-only: Decode for borrowed str needs GAT, String needs alloc
impl Encode for str {
    fn encode<E: Encoder + ?Sized>(&self, e: &mut E) -> Result<(), E::Error> {
        e.encode_str(self)
    }
}
impl<'a> Encode for &'a str {
    fn encode<E: Encoder + ?Sized>(&self, e: &mut E) -> Result<(), E::Error> {
        e.encode_str(self)
    }
}
pub(crate) const STR_SCHEMA: Schema = Schema {
    name: "str",
    kind: Kind::Primitive(Primitive::Str),
};

fn option_schema_inner<T: crate::DataStruct>() -> &'static Schema {
    T::SCHEMA
}

impl<T: Encode> Encode for Option<T> {
    fn encode<E: Encoder + ?Sized>(&self, e: &mut E) -> Result<(), E::Error> {
        match self {
            None => e.encode_none(),
            Some(v) => {
                e.encode_some()?;
                v.encode(e)
            }
        }
    }
}

impl<T: Decode> Decode for Option<T> {
    fn decode<D: Decoder + ?Sized>(d: &mut D) -> Result<Self, D::Error>
    where
        D::Error: From<Error>,
    {
        if d.decode_option_tag()? {
            Ok(Some(T::decode(d)?))
        } else {
            Ok(None)
        }
    }
}

impl<T: crate::DataStruct> crate::DataStruct for Option<T> {
    const SCHEMA: &'static Schema = &Schema {
        name: "Option",
        kind: Kind::Option(option_schema_inner::<T>),
    };
}

impl<T: Encode, const N: usize> Encode for [T; N] {
    fn encode<E: Encoder + ?Sized>(&self, e: &mut E) -> Result<(), E::Error> {
        for v in self.iter() {
            v.encode(e)?;
        }
        Ok(())
    }
}

// Default+Copy: fill array without Vec and without unsafe
impl<T: Decode, const N: usize> Decode for [T; N]
where
    T: Default + Copy,
{
    fn decode<D: Decoder + ?Sized>(d: &mut D) -> Result<Self, D::Error>
    where
        D::Error: From<Error>,
    {
        let mut tmp = [T::default(); N];
        for v in tmp.iter_mut() {
            *v = T::decode(d)?;
        }
        Ok(tmp)
    }
}

fn array_elem_schema<T: crate::DataStruct>() -> &'static Schema {
    T::SCHEMA
}

impl<T: crate::DataStruct + Default + Copy, const N: usize> crate::DataStruct for [T; N] {
    const SCHEMA: &'static Schema = &Schema {
        name: "array",
        kind: Kind::Array {
            elem: array_elem_schema::<T>,
            len: N,
        },
    };
}

macro_rules! tuple_impl {
    ($($T:ident $idx:tt),+) => {
        impl<$($T: Encode),+> Encode for ($($T,)+) {
            fn encode<E: Encoder + ?Sized>(&self, e: &mut E) -> Result<(), E::Error> {
                $(self.$idx.encode(e)?;)+
                Ok(())
            }
        }
        impl<$($T: Decode),+> Decode for ($($T,)+) {
            fn decode<D: Decoder + ?Sized>(d: &mut D) -> Result<Self, D::Error>
            where
                D::Error: From<Error>,
            {
                Ok(($($T::decode(d)?,)+))
            }
        }
    };
}

tuple_impl!(T0 0);
tuple_impl!(T0 0, T1 1);
tuple_impl!(T0 0, T1 1, T2 2);
tuple_impl!(T0 0, T1 1, T2 2, T3 3);

/// Passthrough helper for stub formats.
pub fn map_eof<E>(r: Result<bool, E>, _eof: E) -> Result<bool, E> {
    r
}
