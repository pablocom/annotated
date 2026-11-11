use proc_macro2::Span;
use syn::punctuated::Punctuated;
use syn::{Attribute, DeriveInput, Error, GenericParam, Meta, Token};

use crate::serde_names::SerdeStruct;
use crate::validated_struct::{FieldRule, ValidatedStruct};

pub struct DeserializedStruct<'a> {
    pub validated: ValidatedStruct<'a>,
    pub serde: SerdeStruct,
}

struct AcceptedSerdeAttributes {
    names: &'static [&'static str],
    refusal: &'static str,
}

const ON_A_STRUCT: AcceptedSerdeAttributes = AcceptedSerdeAttributes {
    names: &["rename_all", "rename", "deny_unknown_fields", "expecting"],
    refusal: "with `#[derive(Deserialize)]`, a struct's `#[serde(...)]` takes `rename_all`, `rename`, \
              `deny_unknown_fields` and `expecting`",
};

const ON_A_FIELD: AcceptedSerdeAttributes = AcceptedSerdeAttributes {
    names: &["rename", "alias"],
    refusal: "with `#[derive(Deserialize)]`, a field's `#[serde(...)]` takes `rename` and `alias`",
};

impl<'a> DeserializedStruct<'a> {
    pub fn from_input(input: &'a DeriveInput) -> syn::Result<Self> {
        let validated = ValidatedStruct::from_input(input, "Deserialize")?;
        refuse_lifetime_parameters(input)?;
        refuse_options(&input.attrs)?;
        refuse_serde_attributes_other_than(&ON_A_STRUCT, &input.attrs)?;
        for field in &validated.fields {
            refuse_options(field.attributes)?;
            refuse_serde_attributes_other_than(&ON_A_FIELD, field.attributes)?;
            refuse_nested_values(&field.rules)?;
        }
        let serde = SerdeStruct::from_input(input)?;
        Ok(Self { validated, serde })
    }
}

fn refuse_lifetime_parameters(input: &DeriveInput) -> syn::Result<()> {
    let Some(lifetime) = input.generics.params.iter().find_map(|parameter| match parameter {
        GenericParam::Lifetime(lifetime) => Some(lifetime),
        _ => None,
    }) else {
        return Ok(());
    };
    Err(Error::new_spanned(
        lifetime,
        "`Deserialize` can't be derived for a struct with lifetime parameters: borrowing from the input isn't \
         supported",
    ))
}

fn refuse_options(attributes: &[Attribute]) -> syn::Result<()> {
    for attribute in attributes_named("annotated", attributes) {
        let options = attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        if let Some(option) = options.first() {
            return Err(Error::new_spanned(
                option.path(),
                "`#[annotated(...)]` takes no options yet",
            ));
        }
    }
    Ok(())
}

fn refuse_serde_attributes_other_than(accepted: &AcceptedSerdeAttributes, attributes: &[Attribute]) -> syn::Result<()> {
    for attribute in attributes_named("serde", attributes) {
        let serde_attributes = attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        let refused = serde_attributes.iter().find(|serde_attribute| {
            !accepted
                .names
                .iter()
                .any(|accepted_name| serde_attribute.path().is_ident(accepted_name))
        });
        if let Some(refused) = refused {
            return Err(Error::new_spanned(refused.path(), accepted.refusal));
        }
    }
    Ok(())
}

fn attributes_named<'a>(name: &'a str, attributes: &'a [Attribute]) -> impl Iterator<Item = &'a Attribute> {
    attributes
        .iter()
        .filter(move |attribute| attribute.path().is_ident(name))
}

fn refuse_nested_values(rules: &[FieldRule]) -> syn::Result<()> {
    let Some(nested) = nested_rule_in(rules) else {
        return Ok(());
    };
    Err(Error::new(
        nested,
        "`#[derive(Deserialize)]` can't read a value checked with `nested` yet",
    ))
}

fn nested_rule_in(rules: &[FieldRule]) -> Option<Span> {
    rules.iter().find_map(|rule| match rule {
        FieldRule::Nested { span } => Some(*span),
        FieldRule::Each { element_rules, .. } => nested_rule_in(element_rules),
        FieldRule::Check(_) => None,
    })
}
