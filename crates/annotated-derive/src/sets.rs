use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use syn::{Error, Generics, Type};

use crate::own_rules::ItemParameters;
use crate::validated_struct::{
    CheckedValue, FieldRule, ValidatedField, content_of, element_type, is_map, option_content, ungrouped,
};

// The derive knows a set by its name, as it knows an `Option`.
pub fn is_a_set(written_type: &Type) -> bool {
    let Type::Path(type_path) = ungrouped(written_type) else {
        return false;
    };
    type_path
        .path
        .segments
        .last()
        .is_some_and(|last_segment| last_segment.ident == "HashSet" || last_segment.ident == "BTreeSet")
}

// The collection the `each` rules of a field go through: its own type, or the content of its `Option`.
pub fn collection_of<'a>(field: &ValidatedField<'a>) -> Option<&'a Type> {
    if !field.rules.iter().any(|rule| matches!(rule, FieldRule::Each { .. })) {
        return None;
    }
    Some(option_content(field.field_type).unwrap_or(field.field_type))
}

// `#[derive(Deserialize)]` reads a set as the list of its elements, to check each at its index in the input, only
// where the field holds it. A set inside another value is read by the `Deserialize` of what holds it.
pub fn refuse_each_through_a_set_the_field_does_not_hold(field: &ValidatedField<'_>) -> syn::Result<()> {
    if matches!(field.checked_value, CheckedValue::NotInTheInput) {
        return Ok(());
    }
    refuse_each_through_a_set_inside(&field.rules, field.field_type, true)
}

fn refuse_each_through_a_set_inside(
    rules: &[FieldRule],
    checked_type: &Type,
    is_the_field_s_own: bool,
) -> syn::Result<()> {
    let collection = option_content(checked_type).unwrap_or(checked_type);
    for rule in rules {
        let FieldRule::Each {
            element_rules, span, ..
        } = rule
        else {
            continue;
        };
        if (is_a_set(collection) && !is_the_field_s_own) || is_a_set_behind_a_pointer(collection) {
            return Err(Error::new(
                *span,
                "`#[derive(Deserialize)]` checks a set's elements as it reads them, which it can do only when the set \
                 is the field's own type or an `Option` of it, and this `each` goes through a set inside another value",
            ));
        }
        if let Some(element) = element_type(collection) {
            refuse_each_through_a_set_inside(element_rules, element, false)?;
        }
    }
    Ok(())
}

// `each` goes through the elements of a collection behind a `Box` or a reference, as `element_type` finds them.
fn is_a_set_behind_a_pointer(collection: &Type) -> bool {
    let pointed_to = match ungrouped(collection) {
        Type::Reference(reference) => Some(&*reference.elem),
        _ => content_of("Box", collection),
    };
    pointed_to.is_some_and(|pointed_to| is_a_set(pointed_to) || is_a_set_behind_a_pointer(pointed_to))
}

// A set the derive can't see by its name, behind a type alias say, would be read by its own `Deserialize` and checked
// in the set's order. The code it generates can ask, in a constant: one for each field read from the input whose `each`
// rules go through a collection of a name it doesn't know. A constant can't name a parameter of the item deriving, so
// a type that mentions one is left alone.
pub fn refusals_of_sets_it_can_t_see<'a>(
    generics: &Generics,
    fields_read: impl IntoIterator<Item = &'a ValidatedField<'a>>,
) -> TokenStream {
    let parameters = ItemParameters::of(generics);
    let refusals: Vec<_> = fields_read
        .into_iter()
        .filter_map(collection_of)
        .filter(|collection| may_be_a_set(collection) && !parameters.are_mentioned_in(collection))
        .map(|collection| {
            quote_at! {collection.span()=>
                const _: () = ::core::assert!(
                    !<_annotated::__private::Collection<#collection>>::IS_A_SET,
                    "`#[derive(Deserialize)]` checks a set's elements as it reads them only when it sees the set by \
                     its name: write `HashSet` or `BTreeSet` in this field's type",
                );
            }
        })
        .collect();
    if refusals.is_empty() {
        return TokenStream::new();
    }
    quote! {
        use _annotated::__private::NotASet as _;
        #(#refusals)*
    }
}

// Asking costs compile time, so a type named as a collection other than a set isn't asked about, nor one that isn't
// named at all: an array, a slice or a reference. A type of the user's named like one of them isn't asked about either.
fn may_be_a_set(collection: &Type) -> bool {
    const NOT_A_SET: &[&str] = &["Vec", "VecDeque", "Box"];
    let Type::Path(type_path) = ungrouped(collection) else {
        return false;
    };
    let is_named_as_another_collection = type_path
        .path
        .segments
        .last()
        .is_none_or(|last_segment| NOT_A_SET.iter().any(|name| last_segment.ident == name));
    !is_a_set(collection) && !is_named_as_another_collection && !is_map(type_path)
}
