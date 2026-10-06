#![cfg_attr(not(test), no_std)]
#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod core;

pub use core::error::Error;
pub use core::schema::{
    schema_of, EnumSchema, Field, Kind, Primitive, Schema, SchemaFn, StructKind, StructSchema,
    TupleElem, Variant, VariantKind,
};
pub use core::traits::{DataStruct, Decode, Decoder, Encode, Encoder, StrScratch};

#[cfg(feature = "derive")]
pub use datafmts_derive::{data_struct, DataStruct};
