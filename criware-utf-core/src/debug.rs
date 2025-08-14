use crate::{Error, Reader, Result};

#[derive(Debug, Clone, Copy)]
pub enum ValueKind {
    U8 = 0,
    S8 = 1,
    U16 = 2,
    S16 = 3,
    U32 = 4,
    S32 = 5,
    U64 = 6,
    S64 = 7,
    F32 = 8,
    STR = 0xa,
    BLOB = 0xb,
}

#[derive(Debug, Clone)]
pub enum Column {
    Zero(String, ValueKind),
    Constant(String, ValueKind),
    Rowed(String, ValueKind),
}

#[derive(Debug)]
pub struct Schema {
    pub table_name: String,
    pub columns: Box<[Column]>,
}

impl Reader {
    fn get_column(&mut self) -> Result<Column> {
        let flag: u8 = self.read_raw_value(false)?;
        let column_name: String = self.read_raw_value(false)?;
        let value_kind = match flag & 0x0f {
            0 => ValueKind::U8,
            1 => ValueKind::S8,
            2 => ValueKind::U16,
            3 => ValueKind::S16,
            4 => ValueKind::U32,
            5 => ValueKind::S32,
            6 => ValueKind::U64,
            7 => ValueKind::S64,
            8 => ValueKind::F32,
            0xa => ValueKind::STR,
            0xb => ValueKind::BLOB,
            v => return Err(Error::InvalidColumnType(v)),
        };
        match flag & 0xf0 {
            0x10 => Ok(Column::Zero(column_name, value_kind)),
            0x30 => {
                match value_kind {
                    ValueKind::U8 | ValueKind::S8 => {
                        self.read_raw_value::<u8>(false)?;
                    }
                    ValueKind::U16 | ValueKind::S16 => {
                        self.read_raw_value::<u16>(false)?;
                    }
                    ValueKind::U32 | ValueKind::S32 | ValueKind::F32 | ValueKind::STR => {
                        self.read_raw_value::<u32>(false)?;
                    }
                    ValueKind::U64 | ValueKind::S64 | ValueKind::BLOB => {
                        self.read_raw_value::<u64>(false)?;
                    }
                };
                Ok(Column::Constant(column_name, value_kind))
            }
            0x50 => Ok(Column::Rowed(column_name, value_kind)),
            v => Err(Error::InvalidColumnStorage(v)),
        }
    }
}

impl crate::table::Table for Schema {
    fn read(reader: &mut impl std::io::Read) -> Result<Self> {
        let mut reader = Reader::new(reader)?;
        let mut columns = Vec::new();
        while reader.more_column_data() {
            columns.push(reader.get_column()?);
        }
        Ok(Schema {
            table_name: reader.table_name().to_owned(),
            columns: columns.into_boxed_slice(),
        })
    }
}
