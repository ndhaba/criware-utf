use std::{any::type_name, borrow::Cow, collections::HashMap, io::Write};

use crate::{Error, Result, Value, ValueKind, reader::IOErrorHelper, value::sealed::Primitive};

/**
Extra contextual info for accurating recreating read tables when writing

This entirely exists to handle the edge case where you read a table, remove
all of its rows, and then try to write it. If there are optional rowed values,
there is no clear way to know if a column should be written as zero or rowed.

This is used by the `utf_table` macro. When a table is read, a context is
created with the state of the columns. When a table is created, a context is
created and configured based on the schema provided to the macro.

It is untested whether or not this approach holds, so this type is **subject
to removal**.
 */
pub struct WriteContext(HashMap<&'static str, bool>);

impl WriteContext {
    ///
    /// Creates a new write context
    ///
    pub fn new() -> Self {
        WriteContext(HashMap::new())
    }
    ///
    /// Returns [`true`] if the given column should be included (rowed), or
    /// [`false`] if it should be excluded (zero)
    ///
    pub fn is_included(&self, column_name: &str) -> bool {
        match self.0.get(column_name) {
            Some(v) => *v,
            None => true,
        }
    }
    ///
    /// Sets the inclusion state of a column. [`true`] denotes rowed, [`false`]
    /// denotes zero
    ///
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
    /**
    Creates a new `Writer`

    # Example
    ```no_run
    # use criware_utf_core::Writer;
    let writer = Writer::new("ImportantTable");
    ```
     */
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

    /**
    Verifies the amount of data written to the row buffer, and writes the final
    UTF table to the given stream.

    # Example
    ```no_run
    # use std::fs::File;
    # use criware_utf_core::Writer;
    let mut file = File::create("important-table.bin")?;
    let writer = Writer::new("ImportantTable");
    // ... table writing code ...
    writer.end(file, 12, 1000)?:
    ```
     */
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

    fn push_constant_column_private<T: Value>(
        &mut self,
        name: &'a str,
        value: Option<&'a T>,
    ) -> Result<()> {
        let flag = if value.is_some() { 0x30 } else { 0x10 };
        self.write_primitive::<u8>(false, Cow::Owned(flag | (T::Primitive::TYPE_FLAG as u8)));
        self.write_primitive(false, Cow::Borrowed(name));
        if let Some(value) = value {
            self.write_value(false, value)?;
        }
        self.field_count += 1;
        Ok(())
    }

    /**
    Adds a new constant column with the given value

    # Example
    ```no_run
    # use criware_utf_core::Writer;
    let file_count = 5000u64;
    let comment = "This is my comment".to_owned();
    {
        let writer = Writer::new("ImportantTable");
        writer.push_constant_column("FileCount", file_count)?;
        writer.push_constant_column::<String>("Comment", &comment)?;
    }
    ```
     */
    pub fn push_constant_column<T: Value>(&mut self, name: &'a str, value: &'a T) -> Result<()> {
        self.push_constant_column_private(name, Some(value))
    }

    /**
    Adds a new optional constant column with the given value

    # Example
    ```no_run
    # use criware_utf_core::Writer;
    let crc32: Option<u32> = Some(0);
    let writer = Writer::new("ImportantTable");
    writer.push_constant_column_opt("Crc", &crc32)?;
    ```
     */
    pub fn push_constant_column_opt<T: Value>(
        &mut self,
        name: &'a str,
        value: &'a Option<T>,
    ) -> Result<()> {
        self.push_constant_column_private::<T>(name, value.into())
    }

    fn push_rowed_column_private(&mut self, name: &'a str, included: bool, kind: ValueKind) {
        let storage_flag = if included { 0x50 } else { 0x10 };
        self.write_primitive::<u8>(false, Cow::Owned(storage_flag | (kind as u8)));
        self.write_primitive::<str>(false, Cow::Borrowed(name));
        self.field_count += 1;
    }

    /**
    Adds a new rowed column

    # Example
    ```no_run
    # use criware_utf_core::Writer;
    let writer = Writer::new("ImportantTable");
    writer.push_rowed_column::<u64>("ID");
    ```
     */
    pub fn push_rowed_column<T: Value>(&mut self, name: &'a str) {
        self.push_rowed_column_private(name, true, T::Primitive::TYPE_FLAG)
    }

    /**
    Adds a new optional rowed column

    # Example
    ```no_run
    # use criware_utf_core::{Writer, WriteContext};
    let context = WriteContext::new();
    // ...
    let writer = Writer::new("ImportantTable");
    writer.push_rowed_column_opt::<u64>("ID", context.is_included("ID"));
    ```
     */
    pub fn push_rowed_column_opt<T: Value>(&mut self, name: &'a str, included: bool) {
        self.push_rowed_column_private(name, included, T::Primitive::TYPE_FLAG)
    }

    fn write_primitive<T: Primitive + ?Sized>(&mut self, rowed: bool, value: Cow<'a, T>) {
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

    /**
    Writes a value directly into the column or row buffer

    # Example
    ```no_run
    # use criware_utf_core::{Writer, WriteContext};
    # let rows = Vec::new();
    # let writer = Writer::new("ImportantTable");
    for row in rows {
        writer.write_value::<u64>(true, &row.id)?;
        writer.write_value(true, &row.name)?;
    }
    ```
     */
    pub fn write_value<T: Value>(&mut self, rowed: bool, value: &'a T) -> Result<()> {
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
