use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Attribute, Data, DeriveInput, Meta, Path};

use crate::options::Options;

// The generated code names annotated as `_annotated` and serde as `_serde`, and its scope binds them: annotated to
// `::annotated`, or to the path `#[annotated(crate = "...")]` gives, so that a crate re-exporting annotated can offer
// the derives, and serde to annotated's own, or to the path `#[serde(crate = "...")]` gives.
pub struct CratePaths {
    annotated: TokenStream,
    serde: Option<TokenStream>,
}

impl CratePaths {
    // `Validate`'s code names no serde.
    pub fn of_validate(options: &Options) -> Self {
        Self {
            annotated: annotated_path(options),
            serde: None,
        }
    }

    pub fn of_deserialize(options: &Options, serde_path: Option<&Path>) -> Self {
        Self {
            annotated: annotated_path(options),
            serde: Some(serde_path.map_or_else(|| quote!(_annotated::__private::serde), ToTokens::to_token_stream)),
        }
    }
}

fn annotated_path(options: &Options) -> TokenStream {
    options
        .crate_path
        .as_ref()
        .map_or_else(|| quote!(::annotated), ToTokens::to_token_stream)
}

// Everything a derive generates sits in an anonymous `const` of its own, as with serde's derive, so that the names it
// binds there, the crates' and those of the items its impls share, such as the `static` of a regular expression, add
// none to the user's module.
//
// As serde's derive does, the code allows the use of a deprecated item when the type or one of its variants is
// deprecated: the code has to name them, and the user can't change that.
pub fn in_a_scope_of_its_own(
    input: &DeriveInput,
    paths: &CratePaths,
    shared_items: &TokenStream,
    impls: &TokenStream,
) -> TokenStream {
    let annotated = &paths.annotated;
    let serde = paths.serde.as_ref().map(|serde| quote!(use #serde as _serde;));
    let allowing_deprecated_items = is_deprecated(input).then(|| quote!(#[allow(deprecated)]));
    quote! {
        #[doc(hidden)]
        #allowing_deprecated_items
        const _: () = {
            use #annotated as _annotated;
            #serde
            #shared_items
            #impls
        };
    }
}

fn is_deprecated(input: &DeriveInput) -> bool {
    let is_a_variant_deprecated = match &input.data {
        Data::Enum(data) => data.variants.iter().any(|variant| has_deprecated(&variant.attrs)),
        Data::Struct(_) | Data::Union(_) => false,
    };
    has_deprecated(&input.attrs) || is_a_variant_deprecated
}

// `#[deprecated]`, or `#[allow(deprecated)]`, which says the user knows.
fn has_deprecated(attributes: &[Attribute]) -> bool {
    attributes.iter().any(|attribute| {
        if attribute.path().is_ident("deprecated") {
            return true;
        }
        let Meta::List(list) = &attribute.meta else {
            return false;
        };
        let mut allows_deprecated = false;
        if list.path.is_ident("allow") {
            let _ = list.parse_nested_meta(|meta| {
                allows_deprecated |= meta.path.is_ident("deprecated");
                Ok(())
            });
        }
        allows_deprecated
    })
}
