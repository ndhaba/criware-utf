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

/// Error returned when reading or writing a table fails
///
#[derive(Debug, Error)]
pub enum Error {
    ///
    /// If a data blob is not the correct size
    ///
    /// This is only used in the implementation of [`Value`] for `[u8; N]`
    ///
    #[error("wrong size")]
    BlobWrongSize,
    ///
    /// If a string or data blob is unable to be read from a table
    ///
    /// This means the table is malformed
    ///
    #[error("string/blob not found")]
    DataNotFound,
    ///
    /// If the entire content of the table is unable to be read from a stream
    ///
    #[error("reached end of file early (at {0})")]
    EOF(String),
    ///
    /// If the flag associated with the column's storage method is invalid
    /// (table is malformed)
    ///
    #[error("invalid column storage flag: 0x{0:02}")]
    InvalidColumnStorage(u8),
    ///
    /// If the flag associated with the column's data type is invalid
    /// (table is malformed)
    ///
    #[error("invalid column type flag: 0x{0:02}")]
    InvalidColumnType(u8),
    ///
    /// If an I/O error happens
    ///
    /// This does not include end-of-file errors. There's a variant for that
    /// specifically.
    ///
    #[error("i/o error: {0}")]
    IOError(std::io::Error),
    ///
    /// Generic error for any malformed data in the header of a table
    ///
    #[error("malformed header")]
    MalformedHeader,
    ///
    /// If a string stored in a table is unable to be decoded
    ///
    #[error("error when decoding utf8 string: {0}")]
    StringMalformed(std::str::Utf8Error),
    ///
    /// Occurs when writing
    ///
    /// For a rowed optional value, the value in each row must ALL either be
    /// `Some` or `None`. If this condition is violated, this error is
    /// returned.
    ///
    #[error("optional column conflict: \"{0}\" (values must be all Some or all None)")]
    OptionalColumnConflict(&'static str),
    ///
    /// If a conversion from a primitive to another value (or vice versa) fails
    ///
    #[error("failed to convert {0} to {1}: {2}")]
    ValueConversion(&'static str, &'static str, Box<dyn std::error::Error>),
    ///
    /// If the name of a column is not what was expected
    ///
    /// This indicates the table doesn't follow the expected schema. The table
    /// may still be perfectly valid.
    ///
    #[error("wrong column name: \"{0}\" (expected \"{1}\"")]
    WrongColumnName(String, &'static str),
    ///
    /// If the type of data stored in a column is not what was expected.
    ///
    /// This indicates the table doesn't follow the expected schema. The table
    /// may still be perfectly valid.
    ///
    #[error("wrong column type flag: 0x{0:02} (expected 0x{1:02})")]
    WrongColumnType(u8, u8),
    ///
    /// If the method a column stores data is not what was expected
    ///
    /// This indicates the table doesn't follow the expected schema. The table
    /// may still be perfectly valid.
    ///
    #[error("wrong column storage flag: 0x{0:02} (expected {1})")]
    WrongColumnStorage(u8, &'static str),
    ///
    /// Generic error for a table not following a schema. The table may still
    /// be valid.
    ///
    #[error("wrong table schema")]
    WrongTableSchema,
}

/// A typedef of the result returned by much of the crate.
///
pub type Result<T> = std::result::Result<T, Error>;
