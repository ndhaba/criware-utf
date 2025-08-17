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

/// Macro for arranging a table and implementing useful `Table` behavior
///
#[proc_macro_attribute]
pub fn utf_table(attr: TokenStream, item: TokenStream) -> TokenStream {
    match utf_table::parse(attr.into(), item.into()) {
        Ok(value) => value.into(),
        Err(error) => error.to_compile_error().into(),
    }
}
