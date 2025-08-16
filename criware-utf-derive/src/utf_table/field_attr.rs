use quote::format_ident;
use syn::{DataStruct, Field, Ident, Type, Visibility, spanned::Spanned};

use crate::{Result, utils::*};

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum ColumnStorageType {
    Constant,
    Rowed,
}

pub struct Column {
    pub field_ident: Ident,
    pub column_name: String,
    pub storage_type: ColumnStorageType,
    pub optional: bool,
    pub ty: Type,
    pub vis: Visibility,
    pub variable_ident: Ident,
    pub condition_ident: Ident,
}

fn parse_column(field: &Field, idx: usize) -> Result<Column> {
    let mut storage_type = None;
    let mut optional = false;
    let mut column_name = None;
    for attr in &field.attrs {
        let name = get_attribute_name(&attr)?;
        macro_rules! set_storage_type {
            ($name:expr => $var:ident) => {{
                use_meta_path(&attr, $name)?;
                if storage_type.is_some() {
                    syn_error!(attr.span(), "Multiple storage type attributes");
                } else {
                    storage_type = Some(ColumnStorageType::$var);
                }
            }};
        }
        match name.as_str() {
            "constant" => set_storage_type!("constant" => Constant),
            "rowed" => set_storage_type!("rowed" => Rowed),
            "optional" => {
                if optional {
                    syn_error!(attr.span(), "Duplicate attribute");
                } else {
                    optional = true;
                }
            }
            "column_name" => {
                if column_name.is_some() {
                    syn_error!(attr.span(), "Duplicate attribute");
                } else {
                    column_name = Some(string_from_expr(get_name_value(
                        attr,
                        "column_name",
                        "\"{name}\"",
                    )?)?);
                }
            }
            _ => syn_error!(attr.span(), "Unknown attribute"),
        }
    }
    let field_name = field.ident.clone().unwrap();
    Ok(Column {
        field_ident: field_name.clone(),
        column_name: column_name
            .unwrap_or(snake_case_to_upper_camel(field_name.to_string().as_str())),
        storage_type: storage_type.unwrap_or(ColumnStorageType::Rowed),
        optional,
        ty: field.ty.clone(),
        vis: field.vis.clone(),
        variable_ident: format_ident!("__v{idx}"),
        condition_ident: format_ident!("__c{idx}"),
    })
}

pub struct Columns {
    pub has_constant: bool,
    pub has_row: bool,
    pub has_optional_row: bool,
    pub columns: Vec<Column>,
}

pub fn parse_columns(struct_input: &DataStruct) -> Result<Columns> {
    let mut has_constant = false;
    let mut has_row = false;
    let mut has_optional_row = false;
    let mut columns = Vec::new();
    let mut idx = 0;
    for field in &struct_input.fields {
        let column = parse_column(&field, idx)?;
        match column.storage_type {
            ColumnStorageType::Constant => {
                has_constant = true;
            }
            ColumnStorageType::Rowed => {
                has_row = true;
                if column.optional {
                    has_optional_row = true;
                }
            }
        }
        columns.push(column);
        idx += 1;
    }
    Ok(Columns {
        has_constant,
        has_row,
        has_optional_row,
        columns,
    })
}
