use syn::{DataEnum, DeriveInput, Error, Fields, Ident, Variant};

use crate::serde_names::{SerdeEnum, SerdeField, Tag};
use crate::validated_struct::{Place, StructCheck, ValidatedField, ValidatedStruct};
use crate::written_rule::{StructRule, written_enum_rules, written_variant_rules};

pub struct ValidatedEnum<'a> {
    pub input: &'a DeriveInput,
    pub variants: Vec<ValidatedVariant<'a>>,
    pub is_reporting_all: bool,
    pub enum_checks: Vec<StructCheck>,
}

pub struct ValidatedVariant<'a> {
    pub ident: &'a Ident,
    pub content_place: Place,
    pub content: ValidatedStruct<'a>,
}

impl<'a> ValidatedEnum<'a> {
    pub fn from_input(input: &'a DeriveInput, data: &'a DataEnum) -> syn::Result<Self> {
        let serde_enum = SerdeEnum::from_input(input)?;
        let enum_rules = written_enum_rules(&input.attrs)?;
        let is_reporting_all = enum_rules.iter().any(|rule| matches!(rule, StructRule::ReportAll));
        let variants = data
            .variants
            .iter()
            .zip(serde_enum.variants)
            .map(|(variant, serde_variant)| {
                let content = variant_content(input, variant, serde_variant.fields, is_reporting_all)?;
                Ok(ValidatedVariant {
                    ident: &variant.ident,
                    content_place: content_place(&serde_variant.name, serde_variant.is_untagged, &serde_enum.tag),
                    content,
                })
            })
            .collect::<syn::Result<_>>()?;
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

fn content_place(variant_name: &str, is_variant_untagged: bool, tag: &Tag) -> Place {
    match tag {
        _ if is_variant_untagged => Place::OfItsParent,
        Tag::External => Place::Named(variant_name.to_owned()),
        Tag::Adjacent { content, .. } => Place::Named(content.clone()),
        Tag::Internal { .. } | Tag::Untagged => Place::OfItsParent,
    }
}

fn variant_content<'a>(
    input: &'a DeriveInput,
    variant: &'a Variant,
    serde_fields: Vec<SerdeField>,
    is_reporting_all: bool,
) -> syn::Result<ValidatedStruct<'a>> {
    let fields = validated_fields_of(variant, serde_fields)?;
    let struct_checks = checks_across_the_fields_of(variant, &input.ident, &fields)?;
    Ok(ValidatedStruct {
        input,
        fields,
        is_reporting_all,
        struct_checks,
    })
}

fn validated_fields_of(variant: &Variant, serde_fields: Vec<SerdeField>) -> syn::Result<Vec<ValidatedField<'_>>> {
    let is_newtype = matches!(&variant.fields, Fields::Unnamed(fields) if fields.unnamed.len() == 1);
    variant
        .fields
        .iter()
        .zip(serde_fields)
        .enumerate()
        .map(|(index, (field, serde_field))| {
            let place = match &field.ident {
                Some(_) => Place::of_a_named_field(&serde_field),
                None if is_newtype => Place::OfItsParent,
                None => Place::Indexed(index),
            };
            ValidatedField::from_field(field, index, serde_field, place)
        })
        .collect()
}

fn checks_across_the_fields_of(
    variant: &Variant,
    enum_name: &Ident,
    fields: &[ValidatedField],
) -> syn::Result<Vec<StructCheck>> {
    let variant_rules = written_variant_rules(&variant.attrs)?;
    if let (Fields::Unit, Some(rule)) = (&variant.fields, variant_rules.first()) {
        return Err(Error::new(
            rule.span,
            "a unit variant has no fields for a rule to check: `custom(...)` goes on the enum",
        ));
    }
    let variant_name = format!("{enum_name}::{}", variant.ident);
    variant_rules
        .into_iter()
        .map(|custom| StructCheck::from_custom(custom, &variant_name, fields))
        .collect()
}
