use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Visibility};

use crate::utf_table::{
    field_attr::{Column, ColumnStorageType, Columns},
    main_attr::StructInfo,
};

fn generate_value_struct(
    ident: &Ident,
    columns: &Vec<Column>,
    vis: &Visibility,
    storage_type: ColumnStorageType,
) -> TokenStream {
    let mut fields = Vec::new();
    for column in columns {
        if column.storage_type != storage_type {
            continue;
        }
        let name = &column.field_ident;
        let ty = &column.ty;
        let vis = &column.vis;
        fields.push(if column.optional.is_some() {
            quote! {
                #vis #name: ::std::option::Option<#ty>
            }
        } else {
            quote! {
                #vis #name: #ty
            }
        });
    }
    quote! {
        #vis struct #ident {
            #(#fields),*
        }
    }
}

pub fn generate_structs(struct_info: &StructInfo, columns: &Columns) -> TokenStream {
    let mut structs = Vec::new();
    let mut components = Vec::new();
    if columns.has_constant {
        let ident = &struct_info.constants_ident;
        structs.push(generate_value_struct(
            ident,
            &columns.columns,
            &struct_info.vis,
            ColumnStorageType::Constant,
        ));
        components.push(quote! {
            constants: #ident
        });
    }
    if columns.has_row {
        let ident = &struct_info.row_ident;
        structs.push(generate_value_struct(
            ident,
            &columns.columns,
            &struct_info.vis,
            ColumnStorageType::Rowed,
        ));
        components.push(quote! {
            rows: ::std::vec::Vec<#ident>
        });
    }
    if columns.has_optional_row {
        components.push(quote! {
            write_context: ::criware_utf_core::WriteContext
        });
    }
    let core_ident = &struct_info.table_ident;
    let vis = &struct_info.vis;
    structs.push(quote! {
        #vis struct #core_ident {
            #(#components),*
        }
    });
    quote! {
        #(#structs)*
    }
}
