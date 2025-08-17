use std::{any::type_name, collections::HashMap, io::Write};

use crate::{
    Error, Result, Value,
    reader::IOErrorHelper,
    value::sealed::{Primitive, StorageMethod},
};

pub struct WriteContext(HashMap<&'static str, bool>);

impl WriteContext {
    pub fn new() -> Self {
        WriteContext(HashMap::new())
    }
    pub fn is_included(&self, column_name: &str) -> bool {
        match self.0.get(column_name) {
            Some(v) => *v,
            None => true,
        }
    }
    pub fn set_inclusion_state(&mut self, column_name: &'static str, included: bool) {
        self.0.insert(column_name, included);
    }
}

pub struct Writer {
    column_data: Vec<u8>,
    row_data: Vec<u8>,
    strings: HashMap<String, u32>,
    string_data: Vec<u8>,
    blobs: Vec<u8>,
    field_count: u16,
}

impl Writer {
    pub fn new(table_name: &'static str) -> Writer {
        let mut writer = Writer {
            column_data: Vec::new(),
            row_data: Vec::new(),
            strings: HashMap::new(),
            string_data: Vec::new(),
            blobs: Vec::new(),
            field_count: 0,
        };
        writer.strings.insert("<NULL>".to_string(), 0);
        writer.strings.insert(table_name.to_string(), 7);
        writer.string_data.extend_from_slice(b"<NULL>\0");
        writer.string_data.extend_from_slice(table_name.as_bytes());
        writer.string_data.push(0u8);
        writer
    }
    pub fn end(&self, writer: &mut dyn Write, row_size: u16, row_count: u32) -> Result<()> {
        if self.row_data.len() != (row_size as usize) * (row_count as usize) {
            return Err(Error::MalformedHeader);
        }
        let zeroes = [0u8; 8];
        let row_offset = self.column_data.len() as u32 + 24;
        let string_offset = row_offset + self.row_data.len() as u32;
        let mut blob_offset = string_offset + self.string_data.len() as u32;
        let blob_offset_remainder = 8 - (blob_offset & 7);
        blob_offset += blob_offset_remainder;
        let table_name: u32 = 7;
        let table_size = blob_offset + self.blobs.len() as u32;
        writer.write_all(b"@UTF").io("@UTF header")?;
        writer
            .write_all(&table_size.to_be_bytes())
            .io("@UTF header")?;
        writer
            .write_all(&row_offset.to_be_bytes())
            .io("@UTF header")?;
        writer
            .write_all(&string_offset.to_be_bytes())
            .io("@UTF header")?;
        writer
            .write_all(&blob_offset.to_be_bytes())
            .io("@UTF header")?;
        writer
            .write_all(&table_name.to_be_bytes())
            .io("@UTF header")?;
        writer
            .write_all(&self.field_count.to_be_bytes())
            .io("@UTF header")?;
        writer
            .write_all(&row_size.to_be_bytes())
            .io("@UTF header")?;
        writer
            .write_all(&row_count.to_be_bytes())
            .io("@UTF header")?;
        writer.write_all(&self.column_data).io("UTF column data")?;
        writer.write_all(&self.row_data).io("UTF row data")?;
        writer.write_all(&self.string_data).io("UTF string data")?;
        writer
            .write_all(&zeroes[0..(blob_offset_remainder as usize)])
            .io("UTF string data")?;
        writer.write_all(&self.blobs).io("UTF blobs")?;
        Ok(())
    }
    pub fn push_constant_column<T: Value>(&mut self, name: &'static str, value: &T) -> Result<()> {
        self.write_raw_value::<u8>(false, &(0x30 | (T::Primitive::TYPE_FLAG as u8)))?;
        self.write_static_str(false, name);
        self.write_raw_value(false, value)?;
        self.field_count += 1;
        Ok(())
    }
    pub fn push_constant_column_opt<T: Value>(
        &mut self,
        name: &'static str,
        value: &Option<T>,
    ) -> Result<()> {
        match value {
            Some(value) => self.push_constant_column(name, value),
            None => {
                self.write_raw_value::<u8>(false, &(0x10 | (T::Primitive::TYPE_FLAG as u8)))?;
                self.write_static_str(false, name);
                self.field_count += 1;
                Ok(())
            }
        }
    }
    pub fn push_rowed_column<T: Value>(&mut self, name: &'static str) -> Result<()> {
        self.write_raw_value::<u8>(false, &(0x50 | (T::Primitive::TYPE_FLAG as u8)))?;
        self.write_static_str(false, name);
        self.field_count += 1;
        Ok(())
    }
    pub fn push_rowed_column_opt<T: Value>(
        &mut self,
        name: &'static str,
        included: bool,
    ) -> Result<()> {
        if included {
            self.push_rowed_column::<T>(name)
        } else {
            self.write_raw_value::<u8>(false, &(0x10 | (T::Primitive::TYPE_FLAG as u8)))?;
            self.write_static_str(false, name);
            self.field_count += 1;
            Ok(())
        }
    }
    pub fn write_raw_value<T: Value>(&mut self, rowed: bool, value: &T) -> Result<()> {
        let destination = if rowed {
            &mut self.row_data
        } else {
            &mut self.column_data
        };
        let primitive = match T::to_utf_value(value) {
            Ok(prim) => prim,
            Err(error) => {
                return Err(Error::ValueConversion(
                    type_name::<T>(),
                    type_name::<T::Primitive>(),
                    error,
                ));
            }
        };
        let buffer = match <T::Primitive as Primitive>::STORAGE_TYPE {
            StorageMethod::Number => primitive.write_number(),
            StorageMethod::String => {
                primitive.write_string(&mut self.strings, &mut self.string_data)
            }
            StorageMethod::Blob => primitive.write_blob(&mut self.blobs),
        };
        destination.extend_from_slice(buffer.as_ref());
        Ok(())
    }
    #[doc(hidden)]
    fn write_static_str(&mut self, rowed: bool, string: &'static str) {
        let destination = if rowed {
            &mut self.row_data
        } else {
            &mut self.column_data
        };
        let buffer = match self.strings.get(string) {
            Some(idx) => (*idx).to_be_bytes(),
            None => {
                let position = self.string_data.len() as u32;
                self.string_data.extend_from_slice(string.as_bytes());
                self.string_data.push(0u8);
                self.strings.insert(string.to_owned(), position);
                position.to_be_bytes()
            }
        };
        destination.extend_from_slice(&buffer);
    }
}
