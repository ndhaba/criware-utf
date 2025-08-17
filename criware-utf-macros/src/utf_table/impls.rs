use proc_macro2::TokenStream;
use quote::quote;

use crate::utf_table::{field_attr::Columns, main_attr::StructInfo};

mod read {
    use proc_macro2::TokenStream;
    use quote::{format_ident, quote};

    use crate::utf_table::{
        field_attr::{Column, ColumnStorageType, Columns},
        main_attr::StructInfo,
    };

    fn read_column(column: &Column) -> TokenStream {
        match column.storage_type {
            ColumnStorageType::Constant => {
                let column_name = &column.column_name;
                let var_ident = &column.variable_ident;
                if column.optional.is_some() {
                    quote! {
                        let #var_ident = reader.read_column_constant_opt(#column_name)?;
                    }
                } else {
                    quote! {
                        let #var_ident = reader.read_column_constant(#column_name)?;
                    }
                }
            }
            ColumnStorageType::Rowed => {
                let column_name = &column.column_name;
                let ty = &column.ty;
                if column.optional.is_some() {
                    let cond_ident = &column.condition_ident;
                    quote! {
                        let #cond_ident = reader.read_column_rowed_opt::<#ty>(#column_name)?;
                    }
                } else {
                    quote! {
                        reader.read_column_rowed::<#ty>(#column_name)?;
                    }
                }
            }
        }
    }

    fn field_init(column: &Column) -> TokenStream {
        let field_ident = &column.field_ident;
        let var_ident = &column.variable_ident;
        quote! {
            #field_ident: #var_ident
        }
    }

    fn read_columns(struct_info: &StructInfo, columns: &Columns) -> TokenStream {
        let decls = columns.columns.iter().map(read_column);
        if columns.has_constant {
            let field_inits = columns
                .columns
                .iter()
                .filter(|c| c.storage_type == ColumnStorageType::Constant)
                .map(field_init);
            let constants_ident = &struct_info.constants_ident;
            quote! {
                #(#decls)*
                let constants = #constants_ident { #(#field_inits),* };
            }
        } else {
            quote! {
                #(#decls)*
            }
        }
    }

    fn read_row_value(column: &Column) -> TokenStream {
        let var_ident = &column.variable_ident;
        if column.optional.is_some() {
            let cond_ident = &column.condition_ident;
            quote! {
                let #var_ident = if #cond_ident {
                    Some(reader.read_raw_value(true)?)
                } else {
                    None
                };
            }
        } else {
            quote! {
                let #var_ident = reader.read_raw_value(true)?;
            }
        }
    }

    fn read_rows(struct_info: &StructInfo, columns: &Columns) -> TokenStream {
        if columns.has_row {
            let row_ident = &struct_info.row_ident;
            let field_idents = columns
                .columns
                .iter()
                .filter(|c| c.storage_type == ColumnStorageType::Rowed)
                .map(field_init);
            let decls = columns
                .columns
                .iter()
                .filter(|c| c.storage_type == ColumnStorageType::Rowed)
                .map(read_row_value);
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

    fn context(columns: &Columns) -> TokenStream {
        if columns.has_optional_row {
            let context_additions = columns
                .columns
                .iter()
                .filter(|column| {
                    column.optional.is_some() && column.storage_type == ColumnStorageType::Rowed
                })
                .map(|column| {
                    let column_name = &column.column_name;
                    let cond_ident = &column.condition_ident;
                    quote! {
                        write_context.set_inclusion_state(#column_name, #cond_ident);
                    }
                });
            quote! {
                let mut write_context = ::criware_utf::WriteContext::new();
                #(#context_additions)*
            }
        } else {
            TokenStream::new()
        }
    }

    pub fn fn_read(struct_info: &StructInfo, columns: &Columns) -> TokenStream {
        let table_ident = &struct_info.table_ident;
        let table_name = &struct_info.table_name;
        let field_count = columns.columns.len() as u16;
        let column_code = read_columns(struct_info, columns);
        let row_code = read_rows(struct_info, columns);
        let context_code = context(&columns);
        let mut components = Vec::new();
        if columns.has_constant {
            components.push(format_ident!("constants"));
        }
        if columns.has_row {
            components.push(format_ident!("rows"));
        }
        if columns.has_optional_row {
            components.push(format_ident!("write_context"));
        }
        quote! {
            fn read(reader: &mut dyn ::std::io::Read) -> ::std::result::Result<Self, ::criware_utf::Error> {
                let mut reader = ::criware_utf::Reader::new(reader)?;
                if reader.field_count() != #field_count || reader.table_name() != #table_name {
                    return ::std::result::Result::Err(::criware_utf::Error::WrongTableSchema);
                }
                #column_code
                #context_code
                #row_code
                ::std::result::Result::Ok(#table_ident { #(#components),* })
            }
        }
    }
}

mod new {
    use proc_macro2::{Span, TokenStream};
    use quote::quote;
    use syn::Ident;

    use crate::utf_table::{
        field_attr::{ColumnStorageType, Columns},
        main_attr::StructInfo,
    };

    fn constants(struct_info: &StructInfo, columns: &Columns) -> TokenStream {
        let cond_ident = &struct_info.constants_ident;
        let components = columns.columns.iter().filter(|column| column.storage_type == ColumnStorageType::Constant).map(|column| {
            let field_ident = &column.field_ident;
            match &column.optional  {
                Some(included) => {
                    if *included {
                        quote! {
                            #field_ident: ::std::option::Option::Some(::std::default::Default::default())
                        }
                    } else {
                        quote! {
                            #field_ident: ::std::option::Option::None
                        }
                    }
                }
                None => quote! {
                    #field_ident: ::std::default::Default::default()
                }
            }
        });
        quote! {
            let constants = #cond_ident {
                #(#components),*
            };
        }
    }

    pub fn fn_new(struct_info: &StructInfo, columns: &Columns) -> TokenStream {
        let mut components = Vec::new();
        let constants = {
            if columns.has_constant {
                components.push(Ident::new("constants", Span::call_site()));
                constants(struct_info, columns)
            } else {
                TokenStream::new()
            }
        };
        let rows = {
            if columns.has_row {
                components.push(Ident::new("rows", Span::call_site()));
                quote! {
                    let rows = ::std::vec::Vec::new();
                }
            } else {
                TokenStream::new()
            }
        };
        let write_context = {
            if columns.has_optional_row {
                components.push(Ident::new("write_context", Span::call_site()));
                let inclusions = columns
                    .columns
                    .iter()
                    .filter(|column| {
                        column.storage_type == ColumnStorageType::Rowed && column.optional.is_some()
                    })
                    .map(|column| {
                        let column_name = &column.column_name;
                        let included = column.optional.unwrap();
                        quote! {
                            write_context.set_inclusion_state(#column_name, #included);
                        }
                    });
                quote! {
                    let mut write_context = ::criware_utf::WriteContext::new();
                    #(#inclusions)*
                }
            } else {
                TokenStream::new()
            }
        };
        quote! {
            fn new() -> Self {
                #constants
                #rows
                #write_context
                Self {#(#components),*}
            }
        }
    }
}

mod write {
    use proc_macro2::{Span, TokenStream};
    use quote::quote;
    use syn::Ident;

    use crate::utf_table::{
        field_attr::{Column, ColumnStorageType, Columns},
        main_attr::StructInfo,
    };

    fn push_column(column: &Column) -> TokenStream {
        let column_name = &column.column_name;
        let field_ident = &column.field_ident;
        if column.storage_type == ColumnStorageType::Constant {
            let fn_ident = Ident::new(
                if column.optional.is_some() {
                    "push_constant_column_opt"
                } else {
                    "push_constant_column"
                },
                Span::call_site(),
            );
            quote! {
                table_writer.#fn_ident(#column_name, &self.constants.#field_ident)?;
            }
        } else {
            let ty = &column.ty;
            let cond_ident = &column.condition_ident;
            if column.optional.is_some() {
                quote! {
                    let #cond_ident = if self.rows.is_empty() {
                        self.write_context.is_included(#column_name)
                    } else {
                        self.rows[0].#field_ident.is_some()
                    };
                    table_writer.push_rowed_column_opt::<#ty>(#column_name, #cond_ident)?;
                }
            } else {
                quote! {
                    table_writer.push_rowed_column::<#ty>(#column_name)?;
                }
            }
        }
    }

    fn write_row_value(column: &Column) -> TokenStream {
        let field_ident = &column.field_ident;
        if column.optional.is_some() {
            let cond_ident = &column.condition_ident;
            let name = &column.column_name;
            quote! {
                if #cond_ident != row.#field_ident.is_some() {
                    return ::std::result::Result::Err(::criware_utf::Error::OptionalColumnConflict(#name));
                } else if #cond_ident {
                    table_writer.write_raw_value(true, row.#field_ident.as_ref().unwrap())?;
                }
            }
        } else {
            quote! {
                table_writer.write_raw_value(true, &row.#field_ident)?;
            }
        }
    }

    fn write_rows(columns: &Columns) -> TokenStream {
        if columns.has_row {
            let values = columns
                .columns
                .iter()
                .filter(|column| column.storage_type == ColumnStorageType::Rowed)
                .map(write_row_value);
            quote! {
                for row in &self.rows {
                    #(#values)*
                }
            }
        } else {
            TokenStream::new()
        }
    }

    fn end(columns: &Columns) -> TokenStream {
        if columns.has_row {
            let utf_sizes = columns
                .columns
                .iter()
                .filter(|column| column.storage_type == ColumnStorageType::Rowed)
                .map(|column| {
                    let ty = &column.ty;
                    if column.optional.is_some() {
                        let cond_ident = &column.condition_ident;
                        quote! {
                            if #cond_ident {::criware_utf::utf_size_of::<#ty>()} else {0}
                        }
                    } else {
                        quote! {
                            ::criware_utf::utf_size_of::<#ty>()
                        }
                    }
                });
            quote! {
                table_writer.end(writer, (#(#utf_sizes)+*) as u16, self.rows.len() as u32)
            }
        } else {
            quote! {
                table_writer.end(writer, 0, 0)
            }
        }
    }

    pub fn fn_write(struct_info: &StructInfo, columns: &Columns) -> TokenStream {
        let table_name = &struct_info.table_name;
        let column_code = columns.columns.iter().map(push_column);
        let row_code = write_rows(columns);
        let end_code = end(columns);
        quote! {
            fn write(&self, writer: &mut dyn ::std::io::Write) -> ::std::result::Result<(), ::criware_utf::Error> {
                let mut table_writer = ::criware_utf::Writer::new(#table_name);
                #(#column_code)*
                #row_code
                #end_code
            }
        }
    }
}

pub fn impl_table(struct_info: &StructInfo, columns: &Columns) -> TokenStream {
    let ident = &struct_info.table_ident;
    let new_fn = new::fn_new(struct_info, columns);
    let read_fn = read::fn_read(struct_info, columns);
    let write_fn = write::fn_write(struct_info, columns);
    quote! {
        impl ::criware_utf::Table for #ident {
            #new_fn
            #read_fn
            #write_fn
        }
    }
}
