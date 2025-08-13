use thiserror::Error;

pub(crate) mod reader;
pub(crate) mod table;
pub(crate) mod value;

pub use crate::reader::UTFReader;
pub use crate::table::UTFTable;
pub use crate::value::{UTFPrimitive, UTFValue, utf_size_of};

#[derive(Debug, Error)]
pub enum Error {
    #[error("blob not found")]
    BlobNotFound,
    #[error("reached end of file early (at {0})")]
    EOF(String),
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
    #[error("wrong column storage flag: 0x{0:02} (expected 0x{1:02})")]
    WrongColumnStorage(u8, u8),
    #[error("wrong table name: \"{0}\" (expected \"{1}\")")]
    WrongTableName(String, &'static str),
    #[error("wrong table schema")]
    WrongTableSchema,
}

pub type Result<T> = std::result::Result<T, Error>;
