use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;

use crate::utf_table::{
    field_attr::{Column, ColumnStorageType, Columns},
    main_attr::StructInfo,
};

fn generate_column_decl(column: &Column) -> TokenStream {
    match column.storage_type {
        ColumnStorageType::Zero => {
            let column_name = &column.column_name;
            quote! {
                reader.read_column_zero(#column_name)?;
            }
        }
        ColumnStorageType::Constant => {
            let column_name = &column.column_name;
            let field_ident = &column.field_ident;
            quote! {
                let #field_ident = reader.read_column_constant(#column_name)?;
            }
        }
        ColumnStorageType::Rowed => {
            let column_name = &column.column_name;
            let ty = &column.ty;
            quote! {
                reader.read_column_rowed::<#ty>(#column_name)?;
            }
        }
    }
}

fn generate_column_decls(struct_info: &StructInfo, columns: &Columns) -> TokenStream {
    let decls: Vec<TokenStream> = columns.columns.iter().map(generate_column_decl).collect();
    if columns.has_constant {
        let field_idents: Vec<&Ident> = columns
            .columns
            .iter()
            .filter(|c| c.storage_type == ColumnStorageType::Constant)
            .map(|c| &c.field_ident)
            .collect();
        let constants_ident = &struct_info.constants_ident;
        quote! {
            let constants: #constants_ident = {
                #(#decls)*
                #constants_ident { #(#field_idents),* }
            };
        }
    } else {
        quote! {
            #(#decls)*
        }
    }
}

fn generate_row_value_decl(column: &Column) -> TokenStream {
    let field_ident = &column.field_ident;
    quote! {
        let #field_ident = reader.read_row_value()?;
    }
}

fn generate_row_read(struct_info: &StructInfo, columns: &Columns) -> TokenStream {
    if columns.has_row {
        let row_ident = &struct_info.row_ident;
        let field_idents: Vec<&Ident> = columns
            .columns
            .iter()
            .filter(|c| c.storage_type == ColumnStorageType::Rowed)
            .map(|c| &c.field_ident)
            .collect();
        let decls: Vec<TokenStream> = columns
            .columns
            .iter()
            .filter(|c| c.storage_type == ColumnStorageType::Rowed)
            .map(generate_row_value_decl)
            .collect();
        quote! {
            let mut rows = ::std::vec::Vec::new();
            while reader.more_row_data() {
                #(#decls)*
                rows.push(#row_ident { #(#field_idents),* });
            }
        }
    } else {
        TokenStream::new()
    }
}

pub fn generate_read_fn(struct_info: &StructInfo, columns: &Columns) -> TokenStream {
    let table_ident = &struct_info.table_ident;
    let table_name = &struct_info.table_name;
    let field_count = columns.columns.len() as u16;
    let column_decls = generate_column_decls(struct_info, columns);
    let row_reading_code = generate_row_read(struct_info, columns);
    let mut components = Vec::new();
    if columns.has_constant {
        components.push(format_ident!("constants"));
    }
    if columns.has_row {
        components.push(format_ident!("rows"));
    }
    quote! {
        fn read(reader: &mut impl ::std::io::Read) -> ::std::result::Result<Self, ::criware_utf_core::Error> {
            let mut reader = ::criware_utf_core::Reader::new(reader)?;
            if reader.field_count() != #field_count || reader.table_name() != #table_name {
                return ::std::result::Result::Err(::criware_utf_core::Error::WrongTableSchema);
            }
            #column_decls
            #row_reading_code
            ::std::result::Result::Ok(#table_ident { #(#components),* })
        }
    }
}

pub fn generate_table_impl_block(struct_info: &StructInfo, columns: &Columns) -> TokenStream {
    let ident = &struct_info.table_ident;
    let read_fn = generate_read_fn(struct_info, columns);
    quote! {
        impl ::criware_utf_core::Table for #ident {
            #read_fn
        }
    }
}
