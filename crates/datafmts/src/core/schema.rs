#[derive(Debug, Clone, Copy)]
pub struct Schema {
    pub name: &'static str,
    pub kind: Kind,
}

#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Kind {
    Primitive(Primitive),
    Option(SchemaFn),
    Array { elem: SchemaFn, len: usize },
    Struct(&'static StructSchema),
    Enum(&'static EnumSchema),
}

// fn pointer so Generic<T> resolves per T without alloc
pub type SchemaFn = fn() -> &'static Schema;

/// Return static schema for T; used as fn-pointer in Field.
pub fn schema_of<T: crate::DataStruct>() -> &'static Schema {
    T::SCHEMA
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Primitive {
    Bool,
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    Char,
    Str,
    Unit,
}

#[derive(Debug, Clone, Copy)]
pub struct StructSchema {
    pub name: &'static str,
    pub fields: &'static [Field],
    pub kind: StructKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructKind {
    Unit,
    Tuple,
    Named,
}

#[derive(Clone, Copy)]
pub struct Field {
    pub name: &'static str,
    pub schema: SchemaFn,
    pub has_default: bool,
}

impl Field {
    pub fn schema(&self) -> &'static Schema {
        (self.schema)()
    }
}

impl core::fmt::Debug for Field {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Field")
            .field("name", &self.name)
            .field("has_default", &self.has_default)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct EnumSchema {
    pub name: &'static str,
    pub variants: &'static [Variant],
}

#[derive(Debug, Clone, Copy)]
pub struct Variant {
    pub name: &'static str,
    pub index: u32,
    pub kind: VariantKind,
}

#[derive(Debug, Clone, Copy)]
pub enum VariantKind {
    Unit,
    Tuple(&'static [TupleElem]),
    Struct(&'static StructSchema),
}

#[derive(Clone, Copy)]
pub struct TupleElem {
    pub schema: SchemaFn,
    pub has_default: bool,
}

impl TupleElem {
    pub fn schema(&self) -> &'static Schema {
        (self.schema)()
    }
}

impl core::fmt::Debug for TupleElem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("TupleElem")
            .field("has_default", &self.has_default)
            .finish_non_exhaustive()
    }
}
