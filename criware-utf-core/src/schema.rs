use crate::{Error, Reader, Result, ValueKind};

/// The possible ways a column can store data
///
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnStorageFormat {
    Zero,
    Constant,
    Rowed,
}

/// Representation of a column of a table (data not included)
///
#[derive(Debug, Clone)]
pub struct SchemaColumn {
    pub name: String,
    pub storage_format: ColumnStorageFormat,
    pub value_kind: ValueKind,
}

/// Representation of a table's schema
///
#[derive(Debug)]
pub struct Schema {
    pub table_name: String,
    pub columns: Box<[SchemaColumn]>,
}

impl Reader {
    fn get_column(&mut self) -> Result<SchemaColumn> {
        let flag: u8 = self.read_value(false)?;
        let column_name: String = self.read_value(false)?;
        let value_kind = match flag & 0x0f {
            0 => ValueKind::U8,
            1 => ValueKind::I8,
            2 => ValueKind::U16,
            3 => ValueKind::I16,
            4 => ValueKind::U32,
            5 => ValueKind::I32,
            6 => ValueKind::U64,
            7 => ValueKind::I64,
            8 => ValueKind::F32,
            0xa => ValueKind::STR,
            0xb => ValueKind::BLOB,
            v => return Err(Error::InvalidColumnType(v)),
        };
        match flag & 0xf0 {
            0x10 => Ok(SchemaColumn {
                name: column_name,
                storage_format: ColumnStorageFormat::Zero,
                value_kind,
            }),
            0x30 => {
                match value_kind {
                    ValueKind::U8 | ValueKind::I8 => {
                        self.read_value::<u8>(false)?;
                    }
                    ValueKind::U16 | ValueKind::I16 => {
                        self.read_value::<u16>(false)?;
                    }
                    ValueKind::U32 | ValueKind::I32 | ValueKind::F32 | ValueKind::STR => {
                        self.read_value::<u32>(false)?;
                    }
                    ValueKind::U64 | ValueKind::I64 | ValueKind::BLOB => {
                        self.read_value::<u64>(false)?;
                    }
                };
                Ok(SchemaColumn {
                    name: column_name,
                    storage_format: ColumnStorageFormat::Constant,
                    value_kind,
                })
            }
            0x50 => Ok(SchemaColumn {
                name: column_name,
                storage_format: ColumnStorageFormat::Rowed,
                value_kind,
            }),
            v => Err(Error::InvalidColumnStorage(v)),
        }
    }
}

impl Schema {
    pub fn has_column(&self, name: &str) -> bool {
        for column in &self.columns {
            if column.name == name {
                return true;
            }
        }
        false
    }

    pub fn read(reader: &mut dyn std::io::Read) -> Result<Self> {
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
