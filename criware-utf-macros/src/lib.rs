//! The procedural macros offered by `criware-utf`.
//!
//! Please do not use this on its own. Use the full `criware-utf` crate.
//!

extern crate proc_macro;
use proc_macro::TokenStream;

pub(crate) type Result<T> = syn::Result<T>;

macro_rules! syn_error {
    ($span:expr, $message:expr) => {
        return Err(syn::Error::new($span, $message))
    };
}

mod utf_table;
mod utils;

/**
Macro for arranging a table and implementing useful `Table` behavior

# General Output Structure

This attribute macro accepts a struct definition, and outputs one or more new
struct definitions, along with an implementation of the `Table` trait.

For example, if the struct is named `ImportantTable`. The final struct
definition may look something like this:

```no_run
# struct ImportantTableConstants {};
# struct ImportantTableRow {};
struct ImportantTable {
    constants: ImportantTableConstants,
    rows: Vec<ImportantTableRow>,
    write_context: criware_utf::WriteContext
}
```

The macro would also define `ImportantTableConstants` and `ImportantTableRow`
as part of its output, containing the constant or row fields respectively. They
are only defined when they are needed.

- If there are no constant columns, `ImportantTableConstants` and the `constants`
field are not included
- If there are no rowed columns, `ImportantTableRow` and the `rows` field are
not included
- If there are no optional rowed columns, the `write_context` field is not
included

## Input/Output Examples

```no_run
# use criware_utf_derive::utf_table;
// Input
#[utf_table]
struct NuTable {
    id: i64,
    blob: Vec<u8>,
    #[constant]
    comment: String
}

// Output
struct NuTableConstants {
    comment: String
}
struct NuTableRow {
    id: i64,
    blob: Vec<u8>
}
struct NuTable {
    constants: NuTableConstants,
    rows: Vec<NuTableRow>
}
impl Table for NuTable {/** ... */}
```

```no_run
# use criware_utf_derive::utf_table;
// Input
#[utf_table(constants = FileTableInfo, row = File)]
struct FileTable {
    id: u64,
    name: String,
    #[column_name = "Crc"]
    #[optional]
    crc32: u32
    #[constant]
    version: String
    #[constant]
    #[optional]
    table_metadata: Vec<u8>
}

// Output
struct FileTableInfo {
    version: String,
    table_metadata: Option<Vec<u8>>
}
struct File {
    id: u64,
    name: String,
    crc32: Option<u32>
}
struct FileTable {
    constants: FileTableInfo,
    rows: Vec<File>,
    write_context: WriteContext
}
impl Table for FileTable {/** ... */}
```

# Attribute Options

This section outlines the optional configuration options that can be included
with the `#[utf_table]` attribute.

These options can be chained together by separating them with a comma. Specifying
any of these options multiple times will cause a compile error.

```no_run
# use criware_utf_derive::utf_table;
#[utf_table(table_name = "TABLE", constants = TableStuff)]
# struct Table {}
```

## `table_name`

By default, the macro assumes the name of the table is the name of the struct.
This can be overwritten by specifying a string literal. The generated read procedure
will check that the name of the table matches, so specifying this is important.

```no_run
# use criware_utf_derive::utf_table;
#[utf_table(table_name = "ActualTableName")]
# struct Table {}
```

## `constants`

If a constant struct is generated, by default its name will be the *name of the
input struct* + "Constants". This name can be overwritten by specifying a type name.
If there is no constant struct generated, this option does nothing.

```no_run
# use criware_utf_derive::utf_table;
#[utf_table(constants = TConsts)]
# struct Table {}
```

## `row`

If a row struct is generated, by default its name will be the *name of the
input struct* + "Row". This name can be overwritten by specifying a type name.
If there is no row struct generated, this option does nothing.

```no_run
# use criware_utf_derive::utf_table;
#[utf_table(row = TRow)]
# struct Table {}
```

# Field Options

This section outlines the optional configuration options for each field within
the input struct.

Multiple may be used on the same field, but duplicate or conflicting options
will result in a compile error.

```no_run
# #[criware_utf_derive::utf_table]
# struct Table {
#[optional]
#[constant]
some_value: i64
# }
```

## `#[column_name = "{name}"]`

By default, the in-table column name associated with a field is an **upper
camel case** conversion of the field name (e.g. "some_column" => "SomeColumn").
This can be overwritten by passing a string literal.

```no_run
# #[criware_utf_derive::utf_table]
# struct Table {
#[column_name = "TheActualColumnName"]
some_value: i64
# }
```

## `#[constant]`

By default, columns are rowed. This marks the column as constant instead.

If this is used in the presence of `#[rowed]`, there will be a compile error.

```no_run
# #[criware_utf_derive::utf_table]
# struct Table {
#[constant]
some_value: i64
# }
```

## `#[rowed]`

Since columns are rowed by default, using this attribute is superfluous.
Nevertheless, this can be manually specified if it helps make the form of each
column clear.

If this is used in the presence of `#[constant]`, there will be a compile error.

```no_run
# #[criware_utf_derive::utf_table]
# struct Table {
#[rowed]
some_value: i64
# }
```

## `#[optional]`

This marks a column as optional.

In the constant or row struct output, the field will have type `Option<T>`
instead of `T` (where `T` is the original type specified in the struct definition).
In the table read procedure, the column storage method may be zero or
constant/rowed (depending on whether the column is marked as constant or rowed).

In the table write procedure, the storage method that is used for the column
is dependent on the column's associated value(s). If the values are all `None`,
the storage method will be zero. If the values are all `Some`, the storage
method will be constant/rowed. **There cannot be a mix of `Some` and `None`**.
If there is, the write procedure will error out.

Furthermore, if the column is rowed and there are *no rows*, the storage method
will depend on other factors.

If the table was read, it will simply copy the storage method it was originally
read as.

If the table is created from scratch, the storage method will default to
zero. This behavior can be overwritten with configuration options on
the `#[optional]` attribute.

### `#[optional(include)]`

This will make the column constant/rowed instead of zero

### `#[optional(exclude)]`

This does nothing, but can make the write procedure's behavior more clear to
readers.
*/
#[proc_macro_attribute]
pub fn utf_table(attr: TokenStream, item: TokenStream) -> TokenStream {
    match utf_table::parse(attr.into(), item.into()) {
        Ok(value) => value.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
