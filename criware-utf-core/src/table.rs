use crate::Result;

/// A UTF table that can be read, written, and constructed from nothing
///
pub trait Table: Sized {
    /**
    Creates a new table with default constant values and no rows

    # Example
    ```
    # use criware_utf::{Table, utf_table};
    #[utf_table]
    struct Tab {
        #[constant]
        constant: i32,
        row_value: i64,
    }

    fn main() {
        let table = Tab::new();
        assert_eq!(table.constants.constant, 0);
        assert_eq!(table.rows.len(), 0);
    }
    ```
     */
    fn new() -> Self;

    /**
    Reads a table from the given stream

    If the table is malformed, or if the table's schema does not match this
    type, then this function will fail.

    # Example
    ```
    # use std::fs::File;
    # use criware_utf::{Table, utf_table};
    #[utf_table]
    struct Tab {
        #[constant]
        constant: i32,
        row_value: i64,
    }

    fn main() -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::open("table.bin")?;
        let table = Tab::read(&mut file)?;
        // ... do something ...
        Ok(())
    }
    ```
     */
    fn read(reader: &mut dyn std::io::Read) -> Result<Self>;

    /**
    Writes a table to the given stream

    # Example
    ```
    # use std::fs::File;
    # use criware_utf::{Table, utf_table};
    #[utf_table]
    struct Tab {
        #[constant]
        constant: i32,
        row_value: i64,
    }

    fn main() -> Result<(), Box<dyn std::error::Error>> {
        let mut file = File::create("table.bin")?;
        let table = Tab::new();
        // ... do something ...
        table.write(&mut file)?;
        Ok(())
    }
    ```
     */
    fn write(&self, writer: &mut dyn std::io::Write) -> Result<()>;
}
