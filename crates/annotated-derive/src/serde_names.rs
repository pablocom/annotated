use proc_macro2::Span;
use serde_derive_internals::ast::{Container, Data};
use serde_derive_internals::{Ctxt, Derive};
use syn::{DeriveInput, Ident};

pub fn field_names(input: &DeriveInput) -> syn::Result<Vec<String>> {
    let context = Ctxt::new();
    let serde_private_module = Ident::new("__private", Span::call_site());
    let names = match Container::from_ast(&context, input, Derive::Deserialize, &serde_private_module) {
        Some(Container {
            data: Data::Struct(_, fields),
            ..
        }) => fields
            .iter()
            .map(|field| field.attrs.name().deserialize_name().value.clone())
            .collect(),
        _ => Vec::new(),
    };
    context.check()?;
    Ok(names)
}
