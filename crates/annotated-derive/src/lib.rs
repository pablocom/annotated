//! The derive macros of [`annotated`](https://docs.rs/annotated), which re-exports and documents them.

mod expansion;
mod serde_names;
mod validated_struct;
mod written_rule;

use proc_macro::TokenStream;
use syn::{DeriveInput, Error, parse_macro_input};

use validated_struct::ValidatedStruct;

/// Derives `annotated::Validate` from the `#[validate(...)]` rules on a struct's fields.
#[proc_macro_derive(Validate, attributes(validate, serde))]
pub fn derive_validate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    ValidatedStruct::from_input(&input)
        .map_or_else(Error::into_compile_error, |validated_struct| {
            expansion::validate_impl(&validated_struct)
        })
        .into()
}
