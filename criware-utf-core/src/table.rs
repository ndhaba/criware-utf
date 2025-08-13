use crate::Result;

pub trait UTFTable: Sized {
    fn read(reader: &mut impl std::io::Read) -> Result<Self>;
}
