use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Path;

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
pub fn in_a_scope_of_its_own(paths: &CratePaths, shared_items: &TokenStream, impls: &TokenStream) -> TokenStream {
    let annotated = &paths.annotated;
    let serde = paths.serde.as_ref().map(|serde| quote!(use #serde as _serde;));
    quote! {
        #[doc(hidden)]
        const _: () = {
            use #annotated as _annotated;
            #serde
            #shared_items
            #impls
        };
    }
}
