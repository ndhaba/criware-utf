/*!
Implementation of the UTF table format used in CRIWARE CPK files.

This crate primarily offers the [`utf_table`] macro, which automatically
creates a table read and write procedure based on a schema provided as a
struct definition (see the [`Table`] trait for more info on what is provided,
and the macro itself for info on how to write the schema).

```
#[utf_table]
struct Table {
    #[column_name = "ColumnName"]
    #[rowed]
    rowed_value: i64,
    #[constant]
    constant_value: String,
    #[optional]
    #[rowed]
    rowed_optional_value: i8,
}
```

The [`Schema`] type (unrelated to `utf_table`) allows for retrieving
the structure of a table *without its contents*. This can be useful
for debugging, or any situation where a table may be one of many possible
schemas (see example).

The [`Reader`] and [`Writer`] types are also available for use in custom
read/write procedures, but they are, in their current state, highly
specialized for the [`utf_table`] macro, so using them is
**not recommended**.

# Examples

This section demonstrates important features this crate provides. Each example
can be dropped in to a project and compile (and run if you had the necessary
table files).

All of these examples demonstrate high-level functionality. For more precise
examples and explanations, consult the page for the relevant type/macro.

## Example: Basic table read/write
```
use criware_utf::{Table, utf_table};

#[utf_table]
struct ImportantTable {
    #[column_name = "ID"]
    id: i64,
    file_name: String,
    #[constant]
    comment: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut orig = std::fs::File::open("important-table.bin")?;
    let mut new = std::fs::File::create("more-important-table.bin")?;
    let mut table = ImportantTable::read(&mut orig)?;
    for row in &table.rows {
        println!("\"{}\" (id {})", row.file_name, row.id);
    }
    table.constants.comment = format!("\"{}\" -loser", table.constants.comment);
    table.write(&mut new)?;
    Ok(())
}
```

## Example: Reading one of many schemas
```
use std::io::{Seek, SeekFrom};

use criware_utf::{Schema, Table, utf_table};

#[utf_table(table_name = "CoolTable")]
struct CoolTableV1 {
    id: i64,
    name: String,
}

#[utf_table(table_name = "CoolTable")]
struct CoolTableV2 {
    id: i64,
    name: String,
    #[column_name = "Crc32"]
    crc: u32,
}

enum CoolTable {
    V1(CoolTableV1),
    V2(CoolTableV2),
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut file = std::fs::File::open("table.bin")?;
    let schema = Schema::read(&mut file)?;
    let table = {
        file.seek(SeekFrom::Start(0))?;
        if schema.has_column("Crc32") {
            CoolTable::V2(CoolTableV2::read(&mut file)?)
        } else {
            CoolTable::V1(CoolTableV1::read(&mut file)?)
        }
    };
    // ... do something ...
    Ok(())
}
```
*/

pub use criware_utf_core::*;

#[macro_use]
#[allow(unused_imports)]
extern crate criware_utf_macros;

pub use criware_utf_macros::utf_table;
