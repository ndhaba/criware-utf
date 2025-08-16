use std::{
    collections::HashMap,
    io::{Cursor, Read},
};

use crate::{
    Error, Result, Value,
    value::sealed::{Primitive, StorageMethod},
};

#[inline(always)]
pub(crate) fn is_valid_value_flag(half: u8) -> bool {
    half <= 8 || half == 0xa || half == 0xb
}
#[inline(always)]
pub(crate) fn is_valid_storage_flag(half: u8) -> bool {
    half == 0x10 || half == 0x30 || half == 0x50
}

macro_rules! handle_type_flag {
    ($type_flag:ident) => {
        if $type_flag != <T::Primitive as Primitive>::TYPE_FLAG as u8 {
            if is_valid_value_flag($type_flag) {
                return Err(Error::WrongColumnType(
                    $type_flag,
                    <T::Primitive as Primitive>::TYPE_FLAG as u8,
                ));
            } else {
                return Err(Error::InvalidColumnType($type_flag));
            }
        }
    };
}
macro_rules! handle_storage_flag {
    ($flag:ident == $err:literal: $($name:literal => $ret:expr), *) => {
        $(
            if $flag == $name {
                $ret
            }
        ) else *
        else if is_valid_storage_flag($flag) {
            return Err(Error::WrongColumnStorage($flag, $err));
        } else {
            return Err(Error::InvalidColumnStorage($flag));
        }
    };
}

pub(crate) trait IOErrorHelper<T> {
    fn io(self, message: &str) -> Result<T>;
}
impl IOErrorHelper<()> for std::io::Result<()> {
    fn io(self, message: &str) -> Result<()> {
        match self {
            Ok(value) => Ok(value),
            Err(error) => match error.kind() {
                std::io::ErrorKind::UnexpectedEof => {
                    return Err(Error::EOF(message.to_owned()));
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
    table_name_index: u32,
    field_count: u16,
}

impl Reader {
    pub fn new(reader: &mut dyn Read) -> Result<Reader> {
        let table_size = {
            let mut header = [0u8; 8];
            reader.read_exact(&mut header).io("@UTF header")?;
            let (magic, size): ([u8; 4], u32) = unsafe { std::mem::transmute(header) };
            if &magic != b"@UTF" {
                return Err(Error::MalformedHeader);
            }
            u32::from_be(size)
        };
        let (row_offset, string_offset, blob_offset, table_name, field_count, row_size, row_count) = {
            if table_size < 24 {
                return Err(Error::EOF("@UTF header".to_string()));
            }
            let mut header = [0u8; 24];
            reader.read_exact(&mut header).io("@UTF header")?;
            let result: (u32, u32, u32, u32, u16, u16, u32) =
                unsafe { std::mem::transmute(header) };
            (
                u32::from_be(result.0),
                u32::from_be(result.1),
                u32::from_be(result.2),
                u32::from_be(result.3),
                u16::from_be(result.4),
                u16::from_be(result.5),
                u32::from_be(result.6),
            )
        };
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
        if !strings.contains_key(&table_name) {
            return Err(Error::MalformedHeader);
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
            table_name_index: table_name,
            field_count,
        })
    }
    pub fn field_count(&self) -> u16 {
        self.field_count
    }
    pub fn table_name<'a>(&'a self) -> &'a str {
        self.strings.get(&self.table_name_index).unwrap().as_str()
    }
    pub fn more_column_data(&self) -> bool {
        (self.column_buffer.position() as usize) < self.column_buffer_size
    }
    pub fn more_row_data(&self) -> bool {
        (self.row_buffer.position() as usize) < self.row_buffer_size
    }
    pub fn read_column_constant<T: Value>(&mut self, name: &'static str) -> Result<T> {
        let flag: u8 = self.read_raw_value(false)?;
        let column_name: String = self.read_raw_value(false)?;
        if column_name != name {
            return Err(Error::WrongColumnName(column_name, name));
        }
        let type_flag = flag & 0x0f;
        let storage_flag = flag & 0xf0;
        handle_type_flag!(type_flag);
        handle_storage_flag!(storage_flag == "0x30": 0x30 => self.read_raw_value(false))
    }
    pub fn read_column_constant_opt<T: Value>(&mut self, name: &'static str) -> Result<Option<T>> {
        let flag: u8 = self.read_raw_value(false)?;
        let column_name: String = self.read_raw_value(false)?;
        if column_name != name {
            return Err(Error::WrongColumnName(column_name, name));
        }
        let type_flag = flag & 0x0f;
        let storage_flag = flag & 0xf0;
        handle_type_flag!(type_flag);
        handle_storage_flag!(storage_flag == "0x30": 0x10 => Ok(None), 0x30 => Ok(Some(self.read_raw_value(false)?)))
    }
    pub fn read_column_rowed<T: Value>(&mut self, name: &'static str) -> Result<()> {
        let flag: u8 = self.read_raw_value(false)?;
        let column_name: String = self.read_raw_value(false)?;
        if column_name != name {
            return Err(Error::WrongColumnName(column_name, name));
        }
        let type_flag = flag & 0x0f;
        let storage_flag = flag & 0xf0;
        handle_type_flag!(type_flag);
        handle_storage_flag!(storage_flag == "0x50": 0x50 => Ok(()))
    }
    pub fn read_column_rowed_opt<T: Value>(&mut self, name: &'static str) -> Result<bool> {
        let flag: u8 = self.read_raw_value(false)?;
        let column_name: String = self.read_raw_value(false)?;
        if column_name != name {
            return Err(Error::WrongColumnName(column_name, name));
        }
        let type_flag = flag & 0x0f;
        let storage_flag = flag & 0xf0;
        handle_type_flag!(type_flag);
        handle_storage_flag!(storage_flag == "0x50": 0x10 => Ok(false), 0x50 => Ok(true))
    }
    pub fn read_raw_value<T: Value>(&mut self, row: bool) -> Result<T> {
        let mut buffer: <T::Primitive as Primitive>::Buffer = Default::default();
        let reader = if row {
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
        let primitive: <T as Value>::Primitive = match <T::Primitive as Primitive>::STORAGE_TYPE {
            StorageMethod::Number => <T::Primitive as Primitive>::parse_number(buffer),
            StorageMethod::String => {
                match <T::Primitive as Primitive>::parse_string(buffer, &self.strings) {
                    Some(string) => string,
                    None => return Err(Error::StringNotFound),
                }
            }
            StorageMethod::Blob => {
                match <T::Primitive as Primitive>::parse_blob(buffer, &self.blobs) {
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
