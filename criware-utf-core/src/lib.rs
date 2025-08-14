use thiserror::Error;

mod reader;
mod schema;
mod table;
mod value;

pub use crate::reader::Reader;
pub use crate::schema::{Schema, SchemaColumn};
pub use crate::table::Table;
pub use crate::value::{Primitive, Value, ValueKind, utf_size_of};

#[derive(Debug, Error)]
pub enum Error {
    #[error("blob not found")]
    BlobNotFound,
    #[error("reached end of file early (at {0})")]
    EOF(String),
    #[error("invalid column storage flag: 0x{0:02}")]
    InvalidColumnStorage(u8),
    #[error("invalid column type flag: 0x{0:02}")]
    InvalidColumnType(u8),
    #[error("i/o error: {0}")]
    IOError(std::io::Error),
    #[error("malformed header")]
    MalformedHeader,
    #[error("error when decoding utf8 string: {0}")]
    StringMalformed(std::str::Utf8Error),
    #[error("string not found")]
    StringNotFound,
    #[error("failed to convert {0} to {1}: {2}")]
    ValueConversion(&'static str, &'static str, Box<dyn std::error::Error>),
    #[error("wrong column name: \"{0}\" (expected \"{1}\"")]
    WrongColumnName(String, &'static str),
    #[error("wrong column type flag: 0x{0:02} (expected 0x{1:02})")]
    WrongColumnType(u8, u8),
    #[error("wrong column storage flag: 0x{0:02} (expected {1})")]
    WrongColumnStorage(u8, &'static str),
    #[error("wrong table schema")]
    WrongTableSchema,
}

pub type Result<T> = std::result::Result<T, Error>;
