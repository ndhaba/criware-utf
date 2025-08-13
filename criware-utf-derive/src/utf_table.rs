use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    Result,
    utf_table::{
        field_attr::parse_columns, impls::generate_table_impl_block, main_attr::parse_struct_info,
        structs::generate_structs,
    },
};

mod field_attr;
mod impls;
mod main_attr;
mod structs;

pub fn parse(attr: TokenStream, item: TokenStream) -> Result<TokenStream> {
    let struct_info = parse_struct_info(attr, item)?;
    let columns = parse_columns(&struct_info.data)?;
    let structs = generate_structs(&struct_info, &columns);
    let table_impl = generate_table_impl_block(&struct_info, &columns);
    Ok(quote! {
        #structs
        #table_impl
    })
}
