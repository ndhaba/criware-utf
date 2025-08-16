use crate::Result;

pub trait Table: Sized {
    fn read(reader: &mut dyn std::io::Read) -> Result<Self>;
    fn write(&self, writer: &mut dyn std::io::Write) -> Result<()>;
}
