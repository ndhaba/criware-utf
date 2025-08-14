use proc_macro2::TokenStream;
use quote::format_ident;
use syn::{
    DataStruct, DeriveInput, Ident, MetaNameValue, Token, Visibility, parse::Parse,
    punctuated::Punctuated, spanned::Spanned,
};

use crate::{
    Result,
    utils::{ident_from_expr, string_from_expr},
};

pub struct TableParams {
    pub constants_ident: Option<Ident>,
    pub rows_ident: Option<Ident>,
    pub table_name: Option<String>,
}

impl Parse for TableParams {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut constants_ident = None;
        let mut rows_ident = None;
        let mut table_name = None;
        for meta in Punctuated::<MetaNameValue, Token![,]>::parse_terminated(&input)? {
            let name = match meta.path.get_ident() {
                Some(ident) => ident.to_string(),
                None => syn_error!(meta.path.span(), "Unknown parameter"),
            };
            macro_rules! branch {
                ($func:ident => $var:expr) => {{
                    if $var.is_some() {
                        syn_error!(meta.span(), "Duplicate parameter")
                    }
                    $var = Some($func(&meta.value)?);
                }};
            }
            match name.as_str() {
                "constants" => branch!(ident_from_expr => constants_ident),
                "row" => branch!(ident_from_expr => rows_ident),
                "table_name" => branch!(string_from_expr => table_name),
                _ => syn_error!(meta.path.span(), "Unknown parameter"),
            }
        }
        Ok(TableParams {
            constants_ident,
            rows_ident,
            table_name,
        })
    }
}

pub struct StructInfo {
    pub table_ident: Ident,
    pub table_name: String,
    pub constants_ident: Ident,
    pub row_ident: Ident,
    pub data: DataStruct,
    pub vis: Visibility,
}

pub fn parse_struct_info(attr: TokenStream, item: TokenStream) -> Result<StructInfo> {
    let derive_input = syn::parse2::<DeriveInput>(item)?;
    if !derive_input.generics.params.is_empty() {
        syn_error!(derive_input.generics.span(), "Generics are not supported");
    }
    let data = match derive_input.data {
        syn::Data::Struct(s) => s,
        syn::Data::Enum(e) => syn_error!(e.enum_token.span(), "Enums are not supported"),
        syn::Data::Union(u) => syn_error!(u.union_token.span(), "Unions are not supported"),
    };
    let params = syn::parse2::<TableParams>(attr)?;
    let constants_ident = params
        .constants_ident
        .unwrap_or(format_ident!("{}Constants", derive_input.ident));
    let row_ident = params
        .rows_ident
        .unwrap_or(format_ident!("{}Row", derive_input.ident));
    let table_name = params.table_name.unwrap_or(derive_input.ident.to_string());
    Ok(StructInfo {
        table_ident: derive_input.ident.clone(),
        table_name,
        constants_ident,
        row_ident,
        data,
        vis: derive_input.vis,
    })
}
