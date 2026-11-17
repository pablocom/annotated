//! The derive macros of [`annotated`](https://docs.rs/annotated), which re-exports and documents them.

// Quotes generated code at a place in the user's code, such as a field's type, so that an error in it points there.
// The tokens resolve names as the rest of the generated code does, whatever hygiene the place's own tokens have: an
// item passed to a `macro_rules!` brings the macro's, under which a generated `self` isn't the method's. Lints also
// take the tokens for generated code, which the user can't change. `quote_spanned!` itself is disallowed in
// `clippy.toml`, so that no generated code takes the hygiene of the user's tokens.
macro_rules! quote_at {
    ($place:expr=> $($tokens:tt)*) => {{
        let generated_at_the_place = proc_macro2::Span::call_site().located_at($place);
        #[allow(clippy::disallowed_macros)]
        let tokens = quote::quote_spanned!(generated_at_the_place=> $($tokens)*);
        tokens
    }};
}

mod bounds;
mod described_rules;
mod deserialize_enum_expansion;
mod deserialize_expansion;
mod deserialized_enum;
mod deserialized_struct;
mod expansion;
mod generated;
mod options;
mod pattern;
mod receiver;
mod reserved_names;
mod serde_names;
mod validated_enum;
mod validated_struct;
mod written_rule;

use proc_macro::TokenStream;
use syn::{Data, DeriveInput, Error, parse_macro_input};

use deserialized_enum::DeserializedEnum;
use deserialized_struct::DeserializedStruct;
use generated::CratePaths;
use options::Options;
use serde_names::UnreadableAttribute;
use validated_enum::ValidatedEnum;
use validated_struct::ValidatedStruct;

/// Derives `annotated::Validate` from the `#[validate(...)]` rules on a struct's fields.
#[proc_macro_derive(Validate, attributes(validate, serde, annotated))]
pub fn derive_validate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    validate_impl(input).unwrap_or_else(Error::into_compile_error).into()
}

fn validate_impl(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    reserved_names::OF_VALIDATE.refuse_those_in(&input)?;
    let input = receiver::without_self(input)?;
    let paths = CratePaths::of_validate(&Options::of(&input)?);
    match &input.data {
        Data::Enum(data) => ValidatedEnum::from_input(&input, data, UnreadableAttribute::IsLeftOut)
            .map(|validated_enum| expansion::validate_enum_impl(&validated_enum, &paths)),
        Data::Struct(_) | Data::Union(_) => ValidatedStruct::from_input(
            &input,
            "`Validate` can only be derived for a struct with named fields or an enum",
            UnreadableAttribute::IsLeftOut,
        )
        .map(|validated_struct| expansion::validate_impl(&validated_struct, &paths)),
    }
}

/// Derives `annotated::Deserialize`, and serde's `Deserialize` through it. Reading the struct then collects every
/// missing field and broken rule in it.
#[proc_macro_derive(Deserialize, attributes(validate, serde, annotated))]
pub fn derive_deserialize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    deserialize_impl(input).unwrap_or_else(Error::into_compile_error).into()
}

fn deserialize_impl(input: DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    reserved_names::OF_DESERIALIZE.refuse_those_in(&input)?;
    let input = receiver::without_self(input)?;
    let options = Options::of(&input)?;
    match &input.data {
        Data::Enum(data) => DeserializedEnum::from_input(&input, data, &options).map(|deserialized_enum| {
            let paths = CratePaths::of_deserialize(&options, deserialized_enum.serde_path.as_ref());
            deserialize_enum_expansion::deserialize_enum_impl(&deserialized_enum, &paths)
        }),
        Data::Struct(_) | Data::Union(_) => {
            DeserializedStruct::from_input(&input, &options).map(|deserialized_struct| {
                let paths = CratePaths::of_deserialize(&options, deserialized_struct.serde.serde_path.as_ref());
                deserialize_expansion::deserialize_impl(&deserialized_struct, &paths)
            })
        }
    }
}
