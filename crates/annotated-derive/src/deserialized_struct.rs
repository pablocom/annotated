use syn::punctuated::Punctuated;
use syn::{Attribute, DeriveInput, Error, GenericParam, Meta, MetaNameValue, Token};

use crate::serde_names::SerdeStruct;
use crate::validated_struct::{FieldRule, ValidatedStruct, nested_rules_in};

pub struct DeserializedStruct<'a> {
    pub validated: ValidatedStruct<'a>,
    pub serde: SerdeStruct,
    pub collects_type_errors: bool,
}

struct AcceptedSerdeAttributes {
    names: &'static [&'static str],
    refusal: &'static str,
}

const ON_A_STRUCT: AcceptedSerdeAttributes = AcceptedSerdeAttributes {
    names: &[
        "rename_all",
        "rename",
        "deny_unknown_fields",
        "expecting",
        "default",
        "into",
    ],
    refusal: "with `#[derive(Deserialize)]`, a struct's `#[serde(...)]` takes `rename_all`, `rename`, \
              `deny_unknown_fields`, `expecting`, `default` and those that only change serializing",
};

const ON_A_FIELD: AcceptedSerdeAttributes = AcceptedSerdeAttributes {
    names: &[
        "rename",
        "alias",
        "default",
        "skip_deserializing",
        "skip",
        "skip_serializing",
        "skip_serializing_if",
        "serialize_with",
        "getter",
    ],
    refusal: "with `#[derive(Deserialize)]`, a field's `#[serde(...)]` takes `rename`, `alias`, `default`, \
              `skip_deserializing`, `skip` and those that only change serializing",
};

impl<'a> DeserializedStruct<'a> {
    pub fn from_input(input: &'a DeriveInput) -> syn::Result<Self> {
        let validated = ValidatedStruct::from_input(input, "Deserialize")?;
        refuse_lifetime_parameters(input)?;
        let collects_type_errors = collects_type_errors(&input.attrs)?;
        refuse_serde_attributes_other_than(&ON_A_STRUCT, &input.attrs)?;
        for field in &validated.fields {
            refuse_options_on_a_field(field.attributes)?;
            refuse_serde_attributes_other_than(&ON_A_FIELD, field.attributes)?;
            refuse_nested_more_than_once(&field.rules)?;
        }
        let serde = SerdeStruct::from_input(input)?;
        Ok(Self {
            validated,
            serde,
            collects_type_errors,
        })
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

fn collects_type_errors(attributes: &[Attribute]) -> syn::Result<bool> {
    let mut collects_type_errors = false;
    for option in options_in(attributes)? {
        match option {
            Meta::Path(path) if path.is_ident("collect_type_errors") => collects_type_errors = true,
            with_a_value if with_a_value.path().is_ident("collect_type_errors") => {
                return Err(Error::new_spanned(with_a_value, "`collect_type_errors` takes no value"));
            }
            unknown => {
                return Err(Error::new_spanned(
                    unknown.path(),
                    "`#[annotated(...)]` takes one option, `collect_type_errors`",
                ));
            }
        }
    }
    Ok(collects_type_errors)
}

fn refuse_options_on_a_field(attributes: &[Attribute]) -> syn::Result<()> {
    let Some(option) = options_in(attributes)?.into_iter().next() else {
        return Ok(());
    };
    Err(Error::new_spanned(
        option.path(),
        "`#[annotated(...)]` takes no options on a field: `collect_type_errors` goes on the struct",
    ))
}

fn options_in(attributes: &[Attribute]) -> syn::Result<Vec<Meta>> {
    let mut options = Vec::new();
    for attribute in attributes_named("annotated", attributes) {
        options.extend(attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?);
    }
    Ok(options)
}

fn refuse_serde_attributes_other_than(accepted: &AcceptedSerdeAttributes, attributes: &[Attribute]) -> syn::Result<()> {
    for attribute in attributes_named("serde", attributes) {
        let serde_attributes = attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
        let refused = serde_attributes.iter().find(|serde_attribute| {
            let is_accepted = accepted
                .names
                .iter()
                .any(|accepted_name| serde_attribute.path().is_ident(accepted_name));
            !is_accepted && !is_bound_for_serializing_only(serde_attribute)
        });
        if let Some(refused) = refused {
            return Err(Error::new_spanned(refused.path(), accepted.refusal));
        }
    }
    Ok(())
}

fn is_bound_for_serializing_only(serde_attribute: &Meta) -> bool {
    let Meta::List(bound) = serde_attribute else {
        return false;
    };
    if !bound.path.is_ident("bound") {
        return false;
    }
    bound
        .parse_args_with(Punctuated::<MetaNameValue, Token![,]>::parse_terminated)
        .is_ok_and(|bounds| bounds.iter().all(|bound| bound.path.is_ident("serialize")))
}

fn attributes_named<'a>(name: &'a str, attributes: &'a [Attribute]) -> impl Iterator<Item = &'a Attribute> {
    attributes
        .iter()
        .filter(move |attribute| attribute.path().is_ident(name))
}

fn refuse_nested_more_than_once(rules: &[FieldRule]) -> syn::Result<()> {
    let Some(&second_nested) = nested_rules_in(rules).get(1) else {
        return Ok(());
    };
    Err(Error::new(
        second_nested,
        "`#[derive(Deserialize)]` reads a field's value once, so `nested` can appear once in its rules",
    ))
}
