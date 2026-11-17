use proc_macro2::TokenStream;
use quote::quote;

// Everything a derive generates sits in an anonymous `const` of its own, as with serde's derive, so that the items its
// impls share, such as the `static` of a regular expression, add no name to the user's module.
pub fn in_a_scope_of_its_own(shared_items: &TokenStream, impls: &TokenStream) -> TokenStream {
    quote! {
        #[doc(hidden)]
        const _: () = {
            #shared_items
            #impls
        };
    }
}
