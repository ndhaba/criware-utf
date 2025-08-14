use crate::Result;

pub trait Table: Sized {
    fn read(reader: &mut impl std::io::Read) -> Result<Self>;
}
