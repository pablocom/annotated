use proc_macro2::{Group, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::DeriveInput;

use crate::expansion::spanned;

// As serde's derive does, `Self` in the input becomes the type it names: the code that reads and checks a field runs
// in a visitor of the derive's own, where `Self` is the visitor. Rules are tokens in an attribute, so `Self` is
// replaced in the item's tokens, field types, bounds and rules alike, and only when the item names it.
pub fn without_self(input: DeriveInput) -> syn::Result<DeriveInput> {
    let tokens = input.to_token_stream();
    if !mentions_self(&tokens) {
        return Ok(input);
    }
    let name = &input.ident;
    let (_, type_generics, _) = input.generics.split_for_impl();
    let turbofish = type_generics.as_turbofish();
    let the_type = quote!(#name #turbofish);
    syn::parse2(replaced(tokens, &the_type))
}

fn mentions_self(tokens: &TokenStream) -> bool {
    tokens.clone().into_iter().any(|token| match token {
        TokenTree::Ident(ident) => ident == "Self",
        TokenTree::Group(group) => mentions_self(&group.stream()),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}

// The turbofish form names the type in an expression as well as in a type: `Member::<T>::MAX_AGE`, `Vec<Member::<T>>`.
fn replaced(tokens: TokenStream, the_type: &TokenStream) -> TokenStream {
    tokens
        .into_iter()
        .flat_map(|token| match token {
            TokenTree::Ident(ident) if ident == "Self" => spanned(the_type, ident.span()),
            TokenTree::Group(group) => {
                let mut replaced_group = Group::new(group.delimiter(), replaced(group.stream(), the_type));
                replaced_group.set_span(group.span());
                TokenTree::Group(replaced_group).into_token_stream()
            }
            other => other.into_token_stream(),
        })
        .collect()
}
