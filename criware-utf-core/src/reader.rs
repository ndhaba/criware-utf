use std::{
    collections::HashMap,
    io::{Cursor, Read},
};

use crate::{
    Error, Result, Value,
    value::sealed::{Primitive, StorageMethod},
};

trait IOErrorHelper<T> {
    fn io(self, message: impl AsRef<str>) -> Result<T>;
}
impl<T> IOErrorHelper<T> for std::io::Result<T> {
    #[inline(always)]
    fn io(self, message: impl AsRef<str>) -> Result<T> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => match error.kind() {
                std::io::ErrorKind::UnexpectedEof => {
                    return Err(Error::EOF(message.as_ref().to_owned()));
                }
                _ => return Err(Error::IOError(error)),
            },
        }
    }
}

pub struct Reader {
    column_buffer: Cursor<Vec<u8>>,
    column_buffer_size: usize,
    row_buffer: Cursor<Vec<u8>>,
    row_buffer_size: usize,
    strings: HashMap<u32, String>,
    blobs: Vec<u8>,
}

impl Reader {
    pub fn new(
        reader: &mut impl Read,
        table_name: &'static str,
        field_count: u16,
    ) -> Result<Reader> {
        let table_size = {
            let mut header = [0u8; 8];
            reader.read_exact(&mut header).io("@UTF header")?;
            let (magic, size): ([u8; 4], u32) = unsafe { std::mem::transmute(header) };
            if &magic != b"@UTF" {
                return Err(Error::MalformedHeader);
            }
            u32::from_be(size)
        };
        let (row_offset, string_offset, blob_offset, string_name, f_count, row_size, row_count) = {
            if table_size < 24 {
                return Err(Error::EOF("@UTF header".to_string()));
            }
            let mut header = [0u8; 24];
            reader.read_exact(&mut header).io("@UTF header")?;
            let (ro, so, bo, sn, fc, rs, rc) = unsafe { std::mem::transmute(header) };
            (
                u32::from_be(ro),
                u32::from_be(so),
                u32::from_be(bo),
                u32::from_be(sn),
                u16::from_be(fc),
                u16::from_be(rs),
                u32::from_be(rc),
            )
        };
        if field_count != f_count {
            return Err(Error::WrongTableSchema);
        }
        if 24 > row_offset
            || row_offset > string_offset
            || string_offset > blob_offset
            || blob_offset > table_size
            || (row_size as u32 * row_count) != string_offset - row_offset
        {
            return Err(Error::MalformedHeader);
        }
        let (column_buffer, column_buffer_size) = {
            let mut buffer = vec![0u8; row_offset as usize - 24];
            reader.read_exact(&mut buffer).io("UTF column data")?;
            let len = buffer.len();
            (Cursor::new(buffer), len)
        };
        let (row_buffer, row_buffer_size) = {
            let mut buffer = vec![0u8; (string_offset - row_offset) as usize];
            reader.read_exact(&mut buffer).io("UTF row data")?;
            let len = buffer.len();
            (Cursor::new(buffer), len)
        };
        let strings = {
            let mut buffer = vec![0u8; (blob_offset - string_offset) as usize];
            reader.read_exact(&mut buffer).io("UTF string data")?;
            let mut strings = HashMap::new();
            let mut start = 0;
            let mut index = 0;
            while index < buffer.len() {
                if buffer[index] == 0 {
                    match std::str::from_utf8(&buffer[(start as usize)..index]) {
                        Ok(value) => strings.insert(start, value.to_owned()),
                        Err(error) => return Err(Error::StringMalformed(error)),
                    };
                    start = (index + 1) as u32;
                }
                index += 1;
            }
            strings
        };
        if !strings.contains_key(&string_name) || strings.get(&string_name).unwrap() != table_name {
            return Err(Error::WrongTableName(
                strings
                    .get(&string_name)
                    .unwrap_or(&String::from("{unknown}"))
                    .to_owned(),
                table_name,
            ));
        }
        let mut blobs = vec![0u8; (table_size - blob_offset) as usize];
        reader.read_exact(&mut blobs).io("UTF blob data")?;
        Ok(Reader {
            column_buffer,
            column_buffer_size,
            row_buffer,
            row_buffer_size,
            strings,
            blobs,
        })
    }
    pub fn more_column_data(&self) -> bool {
        (self.column_buffer.position() as usize) < self.column_buffer_size
    }
    pub fn more_row_data(&self) -> bool {
        (self.row_buffer.position() as usize) < self.row_buffer_size
    }
    pub fn read_column_constant<T: Value>(&mut self, name: &'static str) -> Result<T> {
        let flag: u8 = self.read_value(false)?;
        let column_name: String = self.read_value(false)?;
        if column_name != name {
            return Err(Error::WrongColumnName(column_name, name));
        }
        if flag & 0xf0 != 0x30 {
            return Err(Error::WrongColumnStorage(flag & 0xf0, 0x30));
        }
        if flag & 0x0f != <T::Primitive as Primitive>::TYPE_FLAG {
            return Err(Error::WrongColumnType(
                flag & 0x0f,
                <T::Primitive as Primitive>::TYPE_FLAG,
            ));
        }
        self.read_value(false)
    }
    pub fn read_column_zero(&mut self, name: &'static str) -> Result<()> {
        let flag: u8 = self.read_value(false)?;
        let column_name: String = self.read_value(false)?;
        if column_name != name {
            return Err(Error::WrongColumnName(column_name, name));
        }
        if flag & 0xf0 != 0x10 {
            return Err(Error::WrongColumnStorage(flag & 0xf0, 0x10));
        }
        Ok(())
    }
    pub fn read_column_rowed<T: Value>(&mut self, name: &'static str) -> Result<()> {
        let flag: u8 = self.read_value(false)?;
        let column_name: String = self.read_value(false)?;
        if column_name != name {
            return Err(Error::WrongColumnName(column_name, name));
        }
        if flag & 0xf0 != 0x50 {
            return Err(Error::WrongColumnStorage(flag & 0xf0, 0x50));
        }
        if flag & 0x0f != <T::Primitive as Primitive>::TYPE_FLAG {
            return Err(Error::WrongColumnType(
                flag & 0x0f,
                <T::Primitive as Primitive>::TYPE_FLAG,
            ));
        }
        Ok(())
    }
    pub fn read_row_value<T: Value>(&mut self) -> Result<T> {
        self.read_value(true)
    }
    fn read_value<T: Value>(&mut self, rowed: bool) -> Result<T> {
        let mut buffer: <T::Primitive as Primitive>::Buffer = Default::default();
        let reader = if rowed {
            &mut self.row_buffer
        } else {
            &mut self.column_buffer
        };
        match reader.read_exact(buffer.as_mut()) {
            Ok(()) => (),
            Err(error) => match error.kind() {
                std::io::ErrorKind::UnexpectedEof => {
                    return Err(Error::EOF(format!(
                        "reading {} value",
                        std::any::type_name::<T::Primitive>()
                    )));
                }
                _ => return Err(Error::IOError(error)),
            },
        };
        let primitive = match <T::Primitive as Primitive>::STORAGE_TYPE {
            StorageMethod::Number => unsafe { <T::Primitive as Primitive>::parse_number(buffer) },
            StorageMethod::String => {
                match unsafe { <T::Primitive as Primitive>::parse_string(buffer, &self.strings) } {
                    Some(string) => string,
                    None => return Err(Error::StringNotFound),
                }
            }
            StorageMethod::Blob => {
                match unsafe { <T::Primitive as Primitive>::parse_blob(buffer, &self.blobs) } {
                    Some(blob) => blob,
                    None => return Err(Error::BlobNotFound),
                }
            }
        };
        T::from_utf_value(primitive).map_err(|error| {
            Error::ValueConversion(
                std::any::type_name::<T::Primitive>(),
                std::any::type_name::<T>(),
                error,
            )
        })
    }
}
