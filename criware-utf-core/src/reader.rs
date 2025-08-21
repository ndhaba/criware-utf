use std::{
    collections::HashMap,
    io::{Cursor, Read},
};

use crate::{Error, IOErrorHelper, Result, Value, ValueKind, value::sealed::Primitive};

#[inline(always)]
pub(crate) fn is_valid_value_flag(half: u8) -> bool {
    half <= 8 || half == 0xa || half == 0xb
}
#[inline(always)]
pub(crate) fn is_valid_storage_flag(half: u8) -> bool {
    half == 0x10 || half == 0x30 || half == 0x50
}

macro_rules! handle_type_flag {
    ($type_flag:path => $expected:path) => {
        if $type_flag != $expected as u8 {
            if is_valid_value_flag($type_flag) {
                return Err(Error::WrongColumnType($type_flag, $expected as u8));
            } else {
                return Err(Error::InvalidColumnType($type_flag));
            }
        }
    };
}

/// Abstraction layer for reading UTF tables
///
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
    /**
    Creates a new `Reader`

    Preliminary validity checks are performed as well.

    # Example
    ```no_run
    # use std::fs::File;
    # use criware_utf_core::Reader;
    let mut file = File::open("random-table.bin")?;
    let reader = Reader::new(&mut file)?;
    ```
     */
    pub fn new(reader: &mut dyn Read) -> Result<Reader> {
        let table_size = {
            let mut header = [0u8; 8];
            reader.read_exact(&mut header).io("@UTF header")?;
            if &header[0..4] != b"@UTF" {
                return Err(Error::MalformedHeader);
            }
            u32::from_be_bytes(header[4..8].try_into().unwrap())
        };
        if table_size < 24 {
            return Err(Error::EOF("@UTF header".to_string()));
        }
        let mut header = [0u8; 24];
        reader.read_exact(&mut header).io("@UTF header")?;
        let row_offset = u32::from_be_bytes(header[0..4].try_into().unwrap());
        let string_offset = u32::from_be_bytes(header[4..8].try_into().unwrap());
        let blob_offset = u32::from_be_bytes(header[8..12].try_into().unwrap());
        let table_name = u32::from_be_bytes(header[12..16].try_into().unwrap());
        let field_count = u16::from_be_bytes(header[16..18].try_into().unwrap());
        let row_size = u16::from_be_bytes(header[18..20].try_into().unwrap());
        let row_count = u32::from_be_bytes(header[20..24].try_into().unwrap());
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

    /**
    Returns the number of columns in the table being read

    # Example
    ```no_run
    # use std::fs::File;
    # use criware_utf_core::Reader;
    let mut file = File::open("random-table.bin")?;
    let reader = Reader::new(&mut file)?;
    assert_eq!(reader.field_count(), 7u16);
    ```
     */
    pub fn field_count(&self) -> u16 {
        self.field_count
    }

    /**
    Returns the name of the table being read

    # Example
    ```no_run
    # use std::fs::File;
    # use criware_utf_core::Reader;
    let mut file = File::open("random-table.bin")?;
    let reader = Reader::new(&mut file)?;
    assert_eq!(reader.table_name(), "ImportantTable");
    ```
     */
    pub fn table_name<'a>(&'a self) -> &'a str {
        self.strings.get(&self.table_name_index).unwrap().as_str()
    }

    /**
    Returns [`true`] if there is more data in the column data section, or
    [`false`] otherwise.

    # Example
    ```no_run
    # use std::fs::File;
    # use criware_utf_core::Reader;
    let mut file = std::fs::File::open("random-table.bin")?;
    let reader = criware_utf_core::Reader::new(&mut file)?;
    // ... column reading code ...
    if reader.more_column_data() {
        panic!();
    }
    ```
     */
    pub fn more_column_data(&self) -> bool {
        (self.column_buffer.position() as usize) < self.column_buffer_size
    }

    /**
    Returns [`true`] if there is more data in the row data section, or
    [`false`] otherwise.

    # Example
    ```no_run
    # use std::fs::File;
    # use criware_utf_core::Reader;
    let mut file = std::fs::File::open("random-table.bin")?;
    let reader = criware_utf_core::Reader::new(&mut file)?;
    // ... column reading code ...
    while reader.more_row_data() {
        // ... row reading code ...
    }
    ```
     */
    pub fn more_row_data(&self) -> bool {
        (self.row_buffer.position() as usize) < self.row_buffer_size
    }

    fn read_constant_column_private<T: Value>(
        &mut self,
        name: &'static str,
        optional: bool,
    ) -> Result<Option<T>> {
        let flag = self.read_primitive::<u8>(false)?;
        let column_name = self.read_primitive::<str>(false)?;
        if column_name != name {
            return Err(Error::WrongColumnName(column_name, name));
        }
        let type_flag = flag & 0x0f;
        let storage_flag = flag & 0xf0;
        handle_type_flag!(type_flag => T::Primitive::TYPE_FLAG);
        if storage_flag == 0x30 {
            Ok(Some(self.read_value(false)?))
        } else if optional && storage_flag == 0x10 {
            Ok(None)
        } else if is_valid_storage_flag(storage_flag) {
            return Err(Error::WrongColumnStorage(storage_flag, "0x30"));
        } else {
            return Err(Error::InvalidColumnStorage(storage_flag));
        }
    }

    /**
    Attempts to read a constant column with the given name and type.

    If the column matches, the column's value is returned. If the next column
    stored does not match, an error is returned.

    # Example
    ```no_run
    # use std::fs::File;
    # use criware_utf_core::Reader;
    # let mut file = std::fs::File::open("random-table.bin")?;
    # let reader = criware_utf_core::Reader::new(&mut file)?;
    let file_count: u64 = reader.read_constant_column("FileCount")?;
    let version: String = reader.read_constant_column("Version")?;
    ```
     */
    pub fn read_constant_column<T: Value>(&mut self, name: &'static str) -> Result<T> {
        Ok(self.read_constant_column_private(name, false)?.unwrap())
    }

    /**
    Attempts to read an optional constant column with the given name and type.

    If the name and type of value of the next column stored does not match, this
    function will return an error. The storage method of the column may be
    constant or zero. If it's constant, the column's value is returned.

    # Example
    ```no_run
    # use std::fs::File;
    # use criware_utf_core::Reader;
    # let mut file = std::fs::File::open("random-table.bin")?;
    # let reader = criware_utf_core::Reader::new(&mut file)?;
    let file_count: u64 = reader.read_constant_column("FileCount")?;
    let version: String = reader.read_constant_column("Version")?;
    let crc32: Option<u32> = reader.read_constant_column_opt::<u32>("Crc")?;
    ```
     */
    pub fn read_constant_column_opt<T: Value>(&mut self, name: &'static str) -> Result<Option<T>> {
        self.read_constant_column_private(name, true)
    }

    fn read_rowed_column_private(
        &mut self,
        name: &'static str,
        kind: ValueKind,
        optional: bool,
    ) -> Result<bool> {
        let flag = self.read_primitive::<u8>(false)?;
        let column_name = self.read_primitive::<str>(false)?;
        if column_name != name {
            return Err(Error::WrongColumnName(column_name, name));
        }
        let type_flag = flag & 0x0f;
        let storage_flag = flag & 0xf0;
        handle_type_flag!(type_flag => kind);
        if storage_flag == 0x50 {
            Ok(true)
        } else if optional && storage_flag == 0x10 {
            Ok(false)
        } else if is_valid_storage_flag(storage_flag) {
            return Err(Error::WrongColumnStorage(storage_flag, "0x50"));
        } else {
            return Err(Error::InvalidColumnStorage(storage_flag));
        }
    }

    /**
    Attempts to read a rowed column with the given name and type.

    If the next column stored does not match, an error is returned.

    # Example
    ```no_run
    # use std::fs::File;
    # use criware_utf_core::Reader;
    # let mut file = std::fs::File::open("random-table.bin")?;
    # let reader = criware_utf_core::Reader::new(&mut file)?;
    reader.read_constant_column::<i32>("ID")?;
    reader.read_constant_column::<String>("Name")?;
    ```
     */
    pub fn read_rowed_column<T: Value>(&mut self, name: &'static str) -> Result<()> {
        self.read_rowed_column_private(name, T::Primitive::TYPE_FLAG, false)?;
        Ok(())
    }

    /**
    Attempts to read an optional rowed column with the given name and type.

    If the name and type of value of the next column stored does not match, this
    function will return an error. The storage method of the column may be
    rowed or zero. [`true`] denotes that the column is rowed, [`false`] denotes
    the column is zero.

    # Example
    ```no_run
    # use std::fs::File;
    # use criware_utf_core::Reader;
    # let mut file = std::fs::File::open("random-table.bin")?;
    # let reader = criware_utf_core::Reader::new(&mut file)?;
    let crc_included: bool = reader.read_rowed_column_opt::<u32>("Crc")?;
    if crc_included {
        println!("CRC32 checksums are included with each file!");
    } else {
        println!("No checksums found");
    }
    ```
     */
    pub fn read_rowed_column_opt<T: Value>(&mut self, name: &'static str) -> Result<bool> {
        self.read_rowed_column_private(name, T::Primitive::TYPE_FLAG, true)
    }

    fn read_primitive<T: Primitive + ?Sized>(&mut self, row: bool) -> Result<T::Owned> {
        let mut buffer: T::Buffer = Default::default();
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
                        std::any::type_name::<T>()
                    )));
                }
                _ => return Err(Error::IOError(error)),
            },
        };
        match <T as Primitive>::parse(buffer, &self.strings, &self.blobs) {
            Some(prim) => Ok(prim),
            None => Err(Error::DataNotFound),
        }
    }

    /**
    Attempts to read a value from the column or row buffer.

    # Example
    ```no_run
    # use std::fs::File;
    # use criware_utf_core::Reader;
    # let mut file = std::fs::File::open("random-table.bin")?;
    # let reader = criware_utf_core::Reader::new(&mut file)?;
    while reader.more_row_data() {
        let name: String = reader.read_value(true)?;
        let crc32: u32 = reader.read_value(true)?;
        // ...
    }
    ```
     */
    pub fn read_value<T: Value>(&mut self, row: bool) -> Result<T> {
        T::from_primitive(self.read_primitive::<T::Primitive>(row)?).map_err(|error| {
            Error::ValueConversion(
                std::any::type_name::<T::Primitive>(),
                std::any::type_name::<T>(),
                error,
            )
        })
    }
}
