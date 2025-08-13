use syn::{Expr, Ident, Lit, Meta, spanned::Spanned};

use crate::Result;

pub fn get_attribute_name(attr: &syn::Attribute) -> Result<String> {
    match attr.path().get_ident() {
        Some(ident) => Ok(ident.to_string()),
        None => syn_error!(attr.span(), "Unknown attribute"),
    }
}

pub fn get_name_value<'a>(
    attr: &'a syn::Attribute,
    name: &str,
    value_fmt: &str,
) -> Result<&'a Expr> {
    if let Meta::NameValue(nv) = &attr.meta {
        Ok(&nv.value)
    } else {
        syn_error!(
            attr.span(),
            format!("Incorrect usage of \"{name}\"\n#[{name} = {value_fmt}]")
        )
    }
}

pub fn ident_from_expr(expr: &Expr) -> Result<Ident> {
    if let Expr::Path(path) = expr {
        if let Some(attr) = path.attrs.get(0) {
            syn_error!(attr.span(), "Attributes are not allowed here")
        }
        if let None = path.qself {
            if let Some(ident) = path.path.get_ident() {
                return Ok(ident.clone());
            }
        }
    }
    syn_error!(expr.span(), "Expected a struct value")
}

pub fn string_from_expr(expr: &Expr) -> Result<String> {
    if let Expr::Lit(expr_lit) = expr {
        if let Some(attr) = expr_lit.attrs.get(0) {
            syn_error!(attr.span(), "Attributes are not allowed here")
        }
        if let Lit::Str(str) = &expr_lit.lit {
            return Ok(str.value());
        }
    }
    syn_error!(expr.span(), "Expected a string")
}

pub fn snake_case_to_upper_camel(snake_case: &str) -> String {
    let mut result = String::with_capacity(snake_case.len());
    for word in snake_case.split('_') {
        let mut chars = word.chars();
        if let Some(first) = chars.next() {
            result.extend(first.to_uppercase());
            result.extend(chars);
        }
    }
    result
}

pub fn use_meta_path(attr: &syn::Attribute, name: &str) -> Result<()> {
    if let Meta::Path(_) = attr.meta {
        Ok(())
    } else {
        syn_error!(
            attr.span(),
            format!("Incorrect usage of \"{name}\"\n#[{name}]")
        )
    }
}
