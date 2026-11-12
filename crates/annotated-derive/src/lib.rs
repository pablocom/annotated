//! The derive macros of [`annotated`](https://docs.rs/annotated), which re-exports and documents them.

mod deserialize_expansion;
mod deserialized_struct;
mod expansion;
mod pattern;
mod serde_names;
mod validated_enum;
mod validated_struct;
mod written_rule;

use proc_macro::TokenStream;
use syn::{Data, DeriveInput, Error, parse_macro_input};

use deserialized_struct::DeserializedStruct;
use validated_enum::ValidatedEnum;
use validated_struct::ValidatedStruct;

/// Derives `annotated::Validate` from the `#[validate(...)]` rules on a struct's fields.
#[proc_macro_derive(Validate, attributes(validate, serde))]
pub fn derive_validate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let validate_impl = match &input.data {
        Data::Enum(data) => {
            ValidatedEnum::from_input(&input, data).map(|validated_enum| expansion::validate_enum_impl(&validated_enum))
        }
        Data::Struct(_) | Data::Union(_) => ValidatedStruct::from_input(
            &input,
            "`Validate` can only be derived for a struct with named fields or an enum",
        )
        .map(|validated_struct| expansion::validate_impl(&validated_struct)),
    };
    validate_impl.unwrap_or_else(Error::into_compile_error).into()
}

/// Derives `annotated::Deserialize`, and serde's `Deserialize` through it. Reading the struct then collects every
/// missing field and broken rule in it.
#[proc_macro_derive(Deserialize, attributes(validate, serde, annotated))]
pub fn derive_deserialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    DeserializedStruct::from_input(&input)
        .map_or_else(Error::into_compile_error, |deserialized_struct| {
            deserialize_expansion::deserialize_impl(&deserialized_struct)
        })
        .into()
}
