use syn::{DataEnum, DeriveInput, Fields, Ident};

use crate::deserialized_struct::{
    AcceptedSerdeAttributes, DeserializedStruct, Form, collects_type_errors, refuse_flatten, refuse_options_on,
    refuse_serde_attributes_other_than, refuse_what_fields_can_t_take,
};
use crate::serde_names::{SerdeEnum, SerdeStruct, Tag};
use crate::validated_enum::ValidatedEnum;
use crate::validated_struct::{Place, StructCheck, ValidatedStruct};

pub struct DeserializedEnum<'a> {
    pub input: &'a DeriveInput,
    pub variants: Vec<DeserializedVariant<'a>>,
    pub name: String,
    pub tag: Tag,
    pub is_denying_unknown_fields: bool,
    pub expecting: Option<String>,
    pub borrowed_lifetimes: Vec<syn::Lifetime>,
    pub enum_checks: Vec<StructCheck>,
    pub is_reporting_all: bool,
    pub collects_type_errors: bool,
}

pub struct DeserializedVariant<'a> {
    pub ident: &'a Ident,
    pub names_read: Vec<String>,
    pub is_untagged: bool,
    pub content_place: Place,
    pub content: Option<DeserializedStruct<'a>>,
}

const ON_AN_ENUM: AcceptedSerdeAttributes = AcceptedSerdeAttributes {
    names: &[
        "rename_all",
        "rename_all_fields",
        "rename",
        "deny_unknown_fields",
        "expecting",
        "into",
        "tag",
        "content",
        "untagged",
    ],
    refusal: "with `#[derive(Deserialize)]`, an enum's `#[serde(...)]` takes `tag`, `content`, `untagged`, \
              `rename_all`, `rename_all_fields`, `rename`, `deny_unknown_fields`, `expecting` and those that only \
              change serializing",
};

const ON_A_VARIANT: AcceptedSerdeAttributes = AcceptedSerdeAttributes {
    names: &[
        "rename",
        "alias",
        "rename_all",
        "untagged",
        "skip_serializing",
        "serialize_with",
    ],
    refusal: "with `#[derive(Deserialize)]`, a variant's `#[serde(...)]` takes `rename`, `alias`, `rename_all`, \
              `untagged` and those that only change serializing",
};

impl<'a> DeserializedEnum<'a> {
    pub fn from_input(input: &'a DeriveInput, data: &'a DataEnum) -> syn::Result<Self> {
        let validated = ValidatedEnum::from_input(input, data)?;
        let collects_type_errors = collects_type_errors(&input.attrs)?;
        refuse_serde_attributes_other_than(&ON_AN_ENUM, &input.attrs)?;
        for variant in &data.variants {
            refuse_options_on("a variant", "enum", &variant.attrs)?;
            refuse_serde_attributes_other_than(&ON_A_VARIANT, &variant.attrs)?;
        }
        for variant in &validated.variants {
            refuse_what_fields_can_t_take(&variant.fields, "enum")?;
        }
        let serde = SerdeEnum::from_input(input)?;
        if serde.is_denying_unknown_fields {
            for variant in &validated.variants {
                refuse_flatten(&variant.fields, "an enum")?;
            }
        }
        let ValidatedEnum {
            variants: validated_variants,
            is_reporting_all,
            enum_checks,
            ..
        } = validated;
        let SerdeEnum {
            name,
            tag,
            variants: serde_variants,
            is_denying_unknown_fields,
            expecting,
            borrowed_lifetimes,
        } = serde;
        let is_untagged = matches!(tag, Tag::Untagged);
        let variants = data
            .variants
            .iter()
            .zip(validated_variants)
            .zip(serde_variants)
            .map(|((variant, validated_variant), serde_variant)| {
                let form = match &variant.fields {
                    Fields::Unit => None,
                    Fields::Named(_) => Some(Form::Struct),
                    Fields::Unnamed(fields) if fields.unnamed.len() == 1 => Some(Form::Newtype),
                    Fields::Unnamed(_) => Some(Form::Tuple),
                };
                let content = form.map(|form| {
                    let kind = if form == Form::Struct { "struct" } else { "tuple" };
                    DeserializedStruct {
                        validated: ValidatedStruct {
                            input,
                            fields: validated_variant.fields,
                            is_reporting_all,
                            struct_checks: Vec::new(),
                        },
                        serde: SerdeStruct {
                            name: name.clone(),
                            fields: serde_variant.fields,
                            is_denying_unknown_fields,
                            expecting: Some(format!("{kind} variant {}::{}", input.ident, variant.ident)),
                            default: None,
                            borrowed_lifetimes: borrowed_lifetimes.clone(),
                        },
                        collects_type_errors,
                        variant: Some(&variant.ident),
                        form,
                    }
                });
                DeserializedVariant {
                    ident: &variant.ident,
                    names_read: serde_variant.names_read,
                    is_untagged: is_untagged || serde_variant.is_untagged,
                    content_place: validated_variant.content_place,
                    content,
                }
            })
            .collect();
        Ok(Self {
            input,
            variants,
            name,
            tag,
            is_denying_unknown_fields,
            expecting,
            borrowed_lifetimes,
            enum_checks,
            is_reporting_all,
            collects_type_errors,
        })
    }
}

impl DeserializedVariant<'_> {
    // As with serde, a unit variant reads without content, and so does a newtype variant whose value is skipped or is
    // an `Option` serde reads.
    pub fn needs_content(&self) -> bool {
        let Some(content) = &self.content else {
            return false;
        };
        if content.form != Form::Newtype {
            return true;
        }
        let field = &content.validated.fields[0];
        let serde_field = &content.serde.fields[0];
        let is_an_option_serde_reads = field.is_optional && serde_field.deserialize_with.is_none();
        !serde_field.is_skipped && !is_an_option_serde_reads
    }
}
