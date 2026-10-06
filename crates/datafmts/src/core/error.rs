use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    Eof,
    InvalidValue,
    InvalidVariant(u32),
    Custom(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Eof => f.write_str("unexpected end of input"),
            Self::InvalidValue => f.write_str("invalid value"),
            Self::InvalidVariant(i) => {
                f.write_str("invalid variant index ")?;
                fmt::Debug::fmt(i, f)
            }
            Self::Custom(s) => f.write_str(s),
        }
    }
}

impl core::error::Error for Error {}
