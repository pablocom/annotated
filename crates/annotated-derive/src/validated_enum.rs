use syn::{DataEnum, DeriveInput, Error, Fields, Ident, Variant};

use crate::serde_names::{SerdeEnum, SerdeVariant, Tag};
use crate::validated_struct::{Place, StructCheck, ValidatedField};
use crate::written_rule::{StructRule, written_enum_rules};

pub struct ValidatedEnum<'a> {
    pub input: &'a DeriveInput,
    pub variants: Vec<ValidatedVariant<'a>>,
    pub is_reporting_all: bool,
    pub enum_checks: Vec<StructCheck>,
}

pub struct ValidatedVariant<'a> {
    pub ident: &'a Ident,
    pub fields: Vec<ValidatedField<'a>>,
    pub content_place: Place,
}

impl<'a> ValidatedEnum<'a> {
    pub fn from_input(input: &'a DeriveInput, data: &'a DataEnum) -> syn::Result<Self> {
        let serde_enum = SerdeEnum::from_input(input)?;
        let variants = data
            .variants
            .iter()
            .zip(serde_enum.variants)
            .map(|(variant, serde_variant)| ValidatedVariant::from_variant(variant, serde_variant, &serde_enum.tag))
            .collect::<syn::Result<_>>()?;
        let enum_rules = written_enum_rules(&input.attrs)?;
        let is_reporting_all = enum_rules.iter().any(|rule| matches!(rule, StructRule::ReportAll));
        let enum_checks = enum_rules
            .into_iter()
            .filter_map(|rule| match rule {
                StructRule::Custom(custom) => Some(StructCheck {
                    check: custom.check,
                    field: None,
                    message: custom.message,
                    span: custom.span,
                }),
                StructRule::ReportAll => None,
            })
            .collect();
        Ok(Self {
            input,
            variants,
            is_reporting_all,
            enum_checks,
        })
    }
}

impl<'a> ValidatedVariant<'a> {
    fn from_variant(variant: &'a Variant, serde_variant: SerdeVariant, tag: &Tag) -> syn::Result<Self> {
        refuse_rules_on_the_variant(variant)?;
        let content_place = match tag {
            _ if serde_variant.is_untagged => Place::OfItsParent,
            Tag::External => Place::Named(serde_variant.name),
            Tag::Adjacent { content } => Place::Named(content.clone()),
            Tag::Internal | Tag::Untagged => Place::OfItsParent,
        };
        let is_newtype = matches!(&variant.fields, Fields::Unnamed(fields) if fields.unnamed.len() == 1);
        let fields = variant
            .fields
            .iter()
            .zip(serde_variant.fields)
            .enumerate()
            .map(|(index, (field, serde_field))| {
                let place = match &field.ident {
                    Some(_) => Place::of_a_named_field(&serde_field),
                    None if is_newtype => Place::OfItsParent,
                    None => Place::Indexed(index),
                };
                ValidatedField::from_field(field, index, serde_field, place)
            })
            .collect::<syn::Result<_>>()?;
        Ok(Self {
            ident: &variant.ident,
            fields,
            content_place,
        })
    }
}

fn refuse_rules_on_the_variant(variant: &Variant) -> syn::Result<()> {
    let Some(rules) = variant
        .attrs
        .iter()
        .find(|attribute| attribute.path().is_ident("validate"))
    else {
        return Ok(());
    };
    Err(Error::new_spanned(
        rules.path(),
        "`#[validate(...)]` goes on a variant's fields, or on the enum for `custom` and `report_all`",
    ))
}
