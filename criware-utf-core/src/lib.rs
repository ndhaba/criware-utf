use thiserror::Error;

mod reader;
mod schema;
mod table;
mod value;
mod writer;

pub use crate::reader::Reader;
pub use crate::schema::{ColumnStorageFormat, Schema, SchemaColumn};
pub use crate::table::Table;
pub use crate::value::{Primitive, Value, ValueKind, utf_size_of};
pub use crate::writer::{WriteContext, Writer};

/// Error returned when a table can't be read or written
///
#[derive(Debug, Error)]
pub enum Error {
    #[error("blob not found")]
    BlobNotFound,
    #[error("conversion not implemented")]
    ConversionNotImplemented,
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
    #[error("optional column conflict: \"{0}\" (values must be all Some or all None)")]
    OptionalColumnConflict(&'static str),
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

/// A typedef of the result returned by much of the crate.
///
pub type Result<T> = std::result::Result<T, Error>;
