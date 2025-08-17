use crate::Result;

/// A UTF table that can be read, written, and constructed from nothing
///
pub trait Table: Sized {
    fn new() -> Self;
    fn read(reader: &mut dyn std::io::Read) -> Result<Self>;
    fn write(&self, writer: &mut dyn std::io::Write) -> Result<()>;
}
