use syn::punctuated::Punctuated;
use syn::{Attribute, DeriveInput, Error, ExprPath, Ident, Meta, Token};

use crate::options::Options;
use crate::serde_names::{SerdeStruct, UnreadableAttribute};
use crate::sets::refuse_each_through_a_set_the_field_does_not_hold;
use crate::validated_struct::{FieldRule, ValidatedField, ValidatedStruct, nested_rules_in};

pub struct DeserializedStruct<'a> {
    pub validated: ValidatedStruct<'a>,
    pub serde: SerdeStruct,
    pub collects_type_errors: bool,
    pub variant: Option<&'a Ident>,
    pub form: Form,
    // A struct, and a struct variant serde reads with its tag or by name, also takes its fields from a sequence. An
    // untagged struct variant, or one of an adjacently tagged enum, takes them from a map alone, as with serde.
    pub takes_a_sequence: bool,
    pub read_whole_by: Option<ExprPath>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Form {
    Struct,
    Tuple,
    Newtype,
}

pub struct AcceptedSerdeAttributes {
    pub names: &'static [&'static str],
    pub refusal: &'static str,
}

const ON_A_STRUCT: AcceptedSerdeAttributes = AcceptedSerdeAttributes {
    names: &[
        "rename_all",
        "rename",
        "deny_unknown_fields",
        "expecting",
        "default",
        "bound",
        "crate",
        "into",
    ],
    refusal: "with `#[derive(Deserialize)]`, a struct's `#[serde(...)]` takes `rename_all`, `rename`, \
              `deny_unknown_fields`, `expecting`, `default`, `bound`, `crate` and those that only change serializing",
};

impl<'a> DeserializedStruct<'a> {
    pub fn from_input(input: &'a DeriveInput, options: &Options) -> syn::Result<Self> {
        let validated = ValidatedStruct::from_input(
            input,
            "`Deserialize` can only be derived for a struct with named fields or an enum",
            UnreadableAttribute::IsAnError,
        )?;
        refuse_serde_attributes_other_than(&ON_A_STRUCT, &input.attrs)?;
        refuse_what_fields_can_t_take(&validated.fields)?;
        refuse_each_through_a_set_they_do_not_hold(&validated.fields)?;
        let serde = SerdeStruct::from_input(input, UnreadableAttribute::IsAnError)?;
        if serde.is_denying_unknown_fields {
            refuse_flatten(&validated.fields, "a struct")?;
        }
        Ok(Self {
            validated,
            serde,
            collects_type_errors: options.collects_type_errors,
            variant: None,
            form: Form::Struct,
            takes_a_sequence: true,
            read_whole_by: None,
        })
    }
}

pub fn refuse_what_fields_can_t_take(fields: &[ValidatedField]) -> syn::Result<()> {
    for field in fields {
        refuse_nested_more_than_once(&field.rules)?;
    }
    Ok(())
}

pub fn refuse_each_through_a_set_they_do_not_hold(fields: &[ValidatedField]) -> syn::Result<()> {
    for field in fields {
        refuse_each_through_a_set_the_field_does_not_hold(field)?;
    }
    Ok(())
}

pub fn refuse_serde_attributes_other_than(
    accepted: &AcceptedSerdeAttributes,
    attributes: &[Attribute],
) -> syn::Result<()> {
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

pub fn refuse_flatten(fields: &[ValidatedField], denying_unknown_fields: &str) -> syn::Result<()> {
    for field in fields.iter().filter(|field| field.is_flattened) {
        for attribute in attributes_named("serde", field.attributes) {
            let serde_attributes = attribute.parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)?;
            if let Some(flatten) = serde_attributes.iter().find(|meta| meta.path().is_ident("flatten")) {
                return Err(Error::new_spanned(
                    flatten.path(),
                    format!(
                        "with `#[derive(Deserialize)]`, {denying_unknown_fields} that denies unknown fields can't have \
                         a flattened field: a key meant for the flattened field can't be told from an unknown one"
                    ),
                ));
            }
        }
    }
    Ok(())
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
