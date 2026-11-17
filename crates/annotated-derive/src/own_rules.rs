use proc_macro2::{TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::spanned::Spanned;
use syn::{Generics, Ident, Type, TypePath};

use crate::validated_struct::{
    Nesting, ValidatedField, content_of, element_type, is_map, option_content, type_arguments, ungrouped,
};

// A derive can't ask whether a type implements a trait, but the code it generates can, in a constant: one for each
// field without `nested` or `not_nested`, at its type, so that a build reports every field forgotten. A constant
// can't name a parameter of the item deriving, so a type that mentions one is left alone.
pub fn refusals<'a>(
    generics: &Generics,
    fields: impl IntoIterator<Item = (&'a ValidatedField<'a>, WithoutNested)>,
) -> TokenStream {
    let parameters = ItemParameters::of(generics);
    let refusals: Vec<_> = fields
        .into_iter()
        .filter_map(|(field, without_nested)| refusal_of(field, without_nested, &parameters))
        .collect();
    if refusals.is_empty() {
        return TokenStream::new();
    }
    quote! {
        use _annotated::__private::NoOwnRules as _;
        #(#refusals)*
    }
}

#[derive(Clone, Copy)]
pub enum WithoutNested {
    Unchecked,
    ReadBySerde,
}

#[derive(Clone, Copy)]
enum Joining {
    Nested,
    EachNested,
    Either,
}

fn refusal_of(
    field: &ValidatedField,
    without_nested: WithoutNested,
    parameters: &ItemParameters,
) -> Option<TokenStream> {
    if !matches!(field.nesting, Nesting::Unsaid) {
        return None;
    }
    let mut asked_about = Vec::new();
    add_the_types_to_ask_about(field.field_type, parameters, &mut asked_about);
    if asked_about.is_empty() {
        return None;
    }
    let rules_of_its_own = match without_nested {
        WithoutNested::Unchecked => quote!(EXIST_FOR_VALIDATE),
        WithoutNested::ReadBySerde => quote!(EXIST_FOR_DESERIALIZE),
    };
    let questions = asked_about
        .iter()
        .map(|asked_about| quote!(<_annotated::__private::OwnRules<#asked_about>>::#rules_of_its_own));
    let has_any_rules_of_its_own = quote!(#(#questions)||*);
    let refusal = refusal_message(without_nested, joining_of(field.field_type));
    Some(quote_at! {field.field_type.span()=>
        const _: () = ::core::assert!(!(#has_any_rules_of_its_own), #refusal);
    })
}

fn add_the_types_to_ask_about<'t>(
    written_type: &'t Type,
    parameters: &ItemParameters,
    asked_about: &mut Vec<&'t Type>,
) {
    match written_type {
        Type::Group(group) => add_the_types_to_ask_about(&group.elem, parameters, asked_about),
        Type::Paren(parenthesized) => add_the_types_to_ask_about(&parenthesized.elem, parameters, asked_about),
        Type::Reference(reference) => add_the_types_to_ask_about(&reference.elem, parameters, asked_about),
        Type::Array(array) => add_the_types_to_ask_about(&array.elem, parameters, asked_about),
        Type::Slice(slice) => add_the_types_to_ask_about(&slice.elem, parameters, asked_about),
        Type::Tuple(tuple) => {
            for element in &tuple.elems {
                add_the_types_to_ask_about(element, parameters, asked_about);
            }
        }
        Type::Path(type_path) => {
            if holds_no_value(type_path) {
                return;
            }
            if may_have_rules_of_its_own(type_path) && !parameters.are_mentioned_in(written_type) {
                asked_about.push(written_type);
            }
            for argument in type_arguments(type_path) {
                add_the_types_to_ask_about(argument, parameters, asked_about);
            }
        }
        _ => {}
    }
}

fn holds_no_value(type_path: &TypePath) -> bool {
    type_path
        .path
        .segments
        .last()
        .is_some_and(|last_segment| last_segment.ident == "PhantomData")
}

// Asking costs compile time: 200 structs of 8 plain fields built in 3.90 s asking about every type, and in 3.63 s
// without these, as before the question existed. A type of the user's named like one of them is not asked about.
fn may_have_rules_of_its_own(type_path: &TypePath) -> bool {
    const WITHOUT_RULES_OF_THEIR_OWN: &[&str] = &[
        "bool", "char", "str", "String", "f32", "f64", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32",
        "u64", "u128", "usize",
    ];
    const WITH_THE_RULES_OF_WHAT_THEY_HOLD: &[&str] = &[
        "Option", "Box", "Rc", "Arc", "Cow", "Vec", "VecDeque", "HashMap", "BTreeMap", "HashSet", "BTreeSet",
    ];
    type_path.path.segments.last().is_none_or(|last_segment| {
        let name = last_segment.ident.to_string();
        !WITHOUT_RULES_OF_THEIR_OWN.contains(&name.as_str())
            && !WITH_THE_RULES_OF_WHAT_THEY_HOLD.contains(&name.as_str())
    })
}

fn joining_of(field_type: &Type) -> Joining {
    let value_type = option_content(field_type).unwrap_or(field_type);
    if is_a_single_value(value_type) {
        return Joining::Nested;
    }
    let element = element_type(value_type).filter(|_| is_a_collection(value_type));
    if element.is_some_and(|element| is_a_single_value(option_content(element).unwrap_or(element))) {
        return Joining::EachNested;
    }
    Joining::Either
}

fn is_a_single_value(written_type: &Type) -> bool {
    match ungrouped(written_type) {
        Type::Reference(reference) => is_a_single_value(&reference.elem),
        Type::Path(type_path) => match pointed_to(written_type) {
            Some(pointed_to) => is_a_single_value(pointed_to),
            None => type_arguments(type_path).next().is_none(),
        },
        _ => false,
    }
}

fn pointed_to(written_type: &Type) -> Option<&Type> {
    ["Box", "Rc", "Arc"]
        .into_iter()
        .find_map(|pointer| content_of(pointer, written_type))
}

// The collections `HasElements` is implemented for, which `each` goes through.
fn is_a_collection(written_type: &Type) -> bool {
    match ungrouped(written_type) {
        Type::Array(_) | Type::Slice(_) => true,
        Type::Reference(reference) => is_a_collection(&reference.elem),
        Type::Path(type_path) => {
            if let Some(boxed) = content_of("Box", written_type) {
                return is_a_collection(boxed);
            }
            let is_a_list = type_path
                .path
                .segments
                .last()
                .is_some_and(|last_segment| last_segment.ident == "Vec" || last_segment.ident == "VecDeque");
            is_a_list || is_map(type_path)
        }
        _ => false,
    }
}

fn refusal_message(without_nested: WithoutNested, joining: Joining) -> &'static str {
    match (without_nested, joining) {
        (WithoutNested::Unchecked, Joining::Nested) => {
            "this field holds a value with rules of its own, and nothing checks them here: add `#[validate(nested)]` \
             to check them, or `#[validate(not_nested)]` to leave them unchecked"
        }
        (WithoutNested::Unchecked, Joining::EachNested) => {
            "this field's elements have rules of their own, and nothing checks them here: add \
             `#[validate(each(nested))]` to check them, or `#[validate(not_nested)]` to leave them unchecked"
        }
        (WithoutNested::Unchecked, Joining::Either) => {
            "this field holds a value with rules of its own, and nothing checks them here: add `#[validate(nested)]`, \
             or `each(nested)` for the elements of a collection, to check them, or `#[validate(not_nested)]` to leave \
             them unchecked"
        }
        (WithoutNested::ReadBySerde, Joining::Nested) => {
            "this field holds a value with rules of its own, and serde would read it here, so its issues would be one \
             error instead of issues at this field: add `#[validate(nested)]` to report them at this field, or \
             `#[validate(not_nested)]` to read it as serde does"
        }
        (WithoutNested::ReadBySerde, Joining::EachNested) => {
            "this field's elements have rules of their own, and serde would read them here, so their issues would be \
             one error instead of issues at this field: add `#[validate(each(nested))]` to report them at this field, \
             or `#[validate(not_nested)]` to read them as serde does"
        }
        (WithoutNested::ReadBySerde, Joining::Either) => {
            "this field holds a value with rules of its own, and serde would read it here, so its issues would be one \
             error instead of issues at this field: add `#[validate(nested)]`, or `each(nested)` for the elements of a \
             collection, to report them at this field, or `#[validate(not_nested)]` to read it as serde does"
        }
    }
}

struct ItemParameters {
    names: Vec<String>,
    lifetimes: Vec<String>,
}

impl ItemParameters {
    fn of(generics: &Generics) -> Self {
        let names = generics
            .type_params()
            .map(|parameter| &parameter.ident)
            .chain(generics.const_params().map(|parameter| &parameter.ident))
            .map(Ident::to_string)
            .collect();
        let lifetimes = generics
            .lifetimes()
            .map(|parameter| parameter.lifetime.ident.to_string())
            .collect();
        Self { names, lifetimes }
    }

    fn are_mentioned_in(&self, written_type: &Type) -> bool {
        if self.names.is_empty() && self.lifetimes.is_empty() {
            return false;
        }
        self.are_mentioned_among(written_type.to_token_stream())
    }

    fn are_mentioned_among(&self, tokens: TokenStream) -> bool {
        let mut follows_an_apostrophe = false;
        for token in tokens {
            match token {
                TokenTree::Punct(punct) if punct.as_char() == '\'' => {
                    follows_an_apostrophe = true;
                    continue;
                }
                TokenTree::Ident(ident) => {
                    let parameters = if follows_an_apostrophe {
                        &self.lifetimes
                    } else {
                        &self.names
                    };
                    if parameters.iter().any(|parameter| ident == parameter) {
                        return true;
                    }
                }
                TokenTree::Group(group) => {
                    if self.are_mentioned_among(group.stream()) {
                        return true;
                    }
                }
                TokenTree::Punct(_) | TokenTree::Literal(_) => {}
            }
            follows_an_apostrophe = false;
        }
        false
    }
}
