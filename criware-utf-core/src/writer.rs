use std::{any::type_name, borrow::Cow, collections::HashMap, io::Write};

use crate::{Error, Result, Value, reader::IOErrorHelper, value::sealed::Primitive};

/// Extra contextual info for accurating recreating read tables when writing
///
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

/// Abstraction layer for writing UTF tables
///
pub struct Writer<'a> {
    column_data: Vec<u8>,
    row_data: Vec<u8>,
    strings: HashMap<Cow<'a, str>, u32>,
    string_data: Vec<u8>,
    blobs: Vec<u8>,
    field_count: u16,
}

impl<'a> Writer<'a> {
    pub fn new(table_name: &'a str) -> Writer<'a> {
        let mut writer = Writer {
            column_data: Vec::new(),
            row_data: Vec::new(),
            strings: HashMap::new(),
            string_data: Vec::new(),
            blobs: Vec::new(),
            field_count: 0,
        };
        writer.strings.insert(Cow::Borrowed("<NULL>"), 0);
        writer.strings.insert(Cow::Borrowed(table_name), 7);
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
    pub fn push_constant_column<T: Value>(&mut self, name: &'a str, value: &'a T) -> Result<()> {
        self.write_primitive::<u8>(false, Cow::Owned(0x30 | (T::Primitive::TYPE_FLAG as u8)));
        self.write_primitive(false, Cow::Borrowed(name));
        self.write_raw_value(false, value)?;
        self.field_count += 1;
        Ok(())
    }
    pub fn push_constant_column_opt<T: Value>(
        &mut self,
        name: &'a str,
        value: &'a Option<T>,
    ) -> Result<()> {
        match value {
            Some(value) => self.push_constant_column(name, value),
            None => {
                self.write_primitive::<u8>(
                    false,
                    Cow::Owned(0x10 | (T::Primitive::TYPE_FLAG as u8)),
                );
                self.write_primitive::<str>(false, Cow::Borrowed(name));
                self.field_count += 1;
                Ok(())
            }
        }
    }
    pub fn push_rowed_column<T: Value>(&mut self, name: &'a str) -> Result<()> {
        self.write_primitive::<u8>(false, Cow::Owned(0x50 | (T::Primitive::TYPE_FLAG as u8)));
        self.write_primitive::<str>(false, Cow::Borrowed(name));
        self.field_count += 1;
        Ok(())
    }
    pub fn push_rowed_column_opt<T: Value>(&mut self, name: &'a str, included: bool) -> Result<()> {
        if included {
            self.push_rowed_column::<T>(name)
        } else {
            self.write_primitive::<u8>(false, Cow::Owned(0x10 | (T::Primitive::TYPE_FLAG as u8)));
            self.write_primitive::<str>(false, Cow::Borrowed(name));
            self.field_count += 1;
            Ok(())
        }
    }
    pub fn write_primitive<T: Primitive + ?Sized>(&mut self, rowed: bool, value: Cow<'a, T>) {
        let destination = if rowed {
            &mut self.row_data
        } else {
            &mut self.column_data
        };
        destination.extend_from_slice(
            T::write(
                value,
                &mut self.strings,
                &mut self.string_data,
                &mut self.blobs,
            )
            .as_ref(),
        );
    }
    pub fn write_raw_value<T: Value>(&mut self, rowed: bool, value: &'a T) -> Result<()> {
        match T::to_primitive(value) {
            Ok(prim) => {
                self.write_primitive(rowed, prim);
                Ok(())
            }
            Err(error) => {
                return Err(Error::ValueConversion(
                    type_name::<T>(),
                    type_name::<T::Primitive>(),
                    error,
                ));
            }
        }
    }
}
