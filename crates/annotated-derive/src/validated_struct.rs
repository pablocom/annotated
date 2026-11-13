use std::fmt::Display;

use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, format_ident, quote_spanned};
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DataStruct, DeriveInput, Error, Expr, ExprLit, Field, Fields, GenericArgument, Ident, Index, Lit,
    Member, Path, PathArguments, Type, TypePath,
};

use crate::pattern;
use crate::serde_names::{SerdeField, SerdeStruct};
use crate::written_rule::{
    NamedArgument, RuleCall, StructCustom, StructRule, WrittenRule, written_rules, written_struct_rules,
};

pub struct ValidatedStruct<'a> {
    pub input: &'a DeriveInput,
    pub fields: Vec<ValidatedField<'a>>,
    pub has_default: bool,
    pub is_reporting_all: bool,
    pub struct_checks: Vec<StructCheck>,
}

pub struct StructCheck {
    pub check: Expr,
    pub field: Option<usize>,
    pub message: Option<Expr>,
    pub span: Span,
}

pub struct ValidatedField<'a> {
    pub member: Member,
    pub field_type: &'a Type,
    pub attributes: &'a [Attribute],
    pub name: String,
    pub place: Place,
    pub is_flattened: bool,
    pub checked_value: CheckedValue,
    pub is_optional: bool,
    pub rules: Vec<FieldRule>,
}

// Where the value that a field's rules check comes from.
pub enum CheckedValue {
    // As the input holds it, or the field's default when it has one and the input lacks the field.
    FromTheInput { has_default: bool },
    // What a function of the user's made of the input, or the default of a skipped field.
    NotInTheInput,
}

// Where a value is in the input, from where its parent is.
pub enum Place {
    Named(String),
    Indexed(usize),
    // A flattened field's keys sit in its parent's own map, and a newtype variant's value is the variant's content.
    OfItsParent,
}

pub enum FieldRule {
    Check(RuleCheck),
    Nested {
        span: Span,
    },
    Each {
        element_rules: Vec<FieldRule>,
        are_elements_optional: bool,
        span: Span,
    },
}

pub struct RuleCheck {
    pub rule: TokenStream,
    pub is_required: bool,
    pub message: Option<Expr>,
    pub span: Span,
}

impl<'a> ValidatedStruct<'a> {
    pub fn from_input(input: &'a DeriveInput, refusal: &str) -> syn::Result<Self> {
        let Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) = &input.data
        else {
            return Err(Error::new_spanned(&input.ident, refusal));
        };
        let serde_struct = SerdeStruct::from_input(input)?;
        let fields = fields
            .named
            .iter()
            .zip(serde_struct.fields)
            .enumerate()
            .map(|(index, (field, serde_field))| {
                let place = Place::of_a_named_field(&serde_field);
                ValidatedField::from_field(field, index, serde_field, place)
            })
            .collect::<syn::Result<Vec<_>>>()?;
        let struct_rules = written_struct_rules(&input.attrs)?;
        let is_reporting_all = struct_rules.iter().any(|rule| matches!(rule, StructRule::ReportAll));
        let struct_checks = struct_rules
            .into_iter()
            .filter_map(|rule| match rule {
                StructRule::Custom(custom) => Some(StructCheck::from_custom(*custom, &input.ident, &fields)),
                StructRule::ReportAll => None,
            })
            .collect::<syn::Result<_>>()?;
        Ok(Self {
            input,
            fields,
            has_default: serde_struct.default.is_some(),
            is_reporting_all,
            struct_checks,
        })
    }
}

impl ValidatedStruct<'_> {
    pub fn is_checked_by_a_struct_rule(&self, field: usize) -> bool {
        self.struct_checks.iter().any(|check| check.field == Some(field))
    }

    pub fn is_checked_by_a_later_struct_rule(&self, field: usize, position: usize) -> bool {
        self.struct_checks[position + 1..]
            .iter()
            .any(|check| check.field == Some(field))
    }
}

impl StructCheck {
    pub fn from_custom(custom: StructCustom, owner: &dyn Display, fields: &[ValidatedField]) -> syn::Result<Self> {
        let field = custom
            .field
            .map(|field| position_of(&field, owner, fields))
            .transpose()?;
        Ok(Self {
            check: custom.check,
            field,
            message: custom.message,
            span: custom.span,
        })
    }
}

fn position_of(field: &Expr, owner: &dyn Display, fields: &[ValidatedField]) -> syn::Result<usize> {
    fields
        .iter()
        .position(|validated| is_named_by(&validated.member, field))
        .ok_or_else(|| Error::new_spanned(field, format!("`{owner}` has no field `{}`", field.to_token_stream())))
}

fn is_named_by(member: &Member, written: &Expr) -> bool {
    match (written, member) {
        (Expr::Path(name), Member::Named(member)) => name.path.is_ident(member),
        (
            Expr::Lit(ExprLit {
                lit: Lit::Int(index), ..
            }),
            Member::Unnamed(member),
        ) => index.base10_parse::<u32>().is_ok_and(|index| index == member.index),
        _ => false,
    }
}

impl Place {
    pub fn of_a_named_field(serde_field: &SerdeField) -> Self {
        if serde_field.is_flattened {
            return Self::OfItsParent;
        }
        Self::Named(serde_field.name.clone())
    }
}

impl<'a> ValidatedField<'a> {
    pub fn from_field(field: &'a Field, index: usize, serde_field: SerdeField, place: Place) -> syn::Result<Self> {
        let member = field
            .ident
            .clone()
            .map_or_else(|| Member::Unnamed(Index::from(index)), Member::Named);
        let rules = written_rules(&field.attrs)?
            .into_iter()
            .map(|written| FieldRule::from_written(written, Some(&field.ty)))
            .collect::<syn::Result<_>>()?;
        Ok(Self {
            member,
            field_type: &field.ty,
            attributes: &field.attrs,
            checked_value: CheckedValue::of(&serde_field),
            name: serde_field.name,
            place,
            is_flattened: serde_field.is_flattened,
            is_optional: option_content(&field.ty).is_some(),
            rules,
        })
    }
}

impl CheckedValue {
    fn of(serde_field: &SerdeField) -> Self {
        if serde_field.is_skipped || serde_field.deserialize_with.is_some() {
            return Self::NotInTheInput;
        }
        Self::FromTheInput {
            has_default: serde_field.default.is_some(),
        }
    }
}

pub fn option_content(checked_type: &Type) -> Option<&Type> {
    content_of("Option", checked_type)
}

fn content_of<'a>(wrapper: &str, checked_type: &'a Type) -> Option<&'a Type> {
    let Type::Path(type_path) = checked_type else {
        return None;
    };
    let last_segment = type_path.path.segments.last()?;
    if last_segment.ident != wrapper {
        return None;
    }
    type_arguments(type_path).next()
}

fn element_type(collection: &Type) -> Option<&Type> {
    if let Some(boxed_collection) = content_of("Box", collection) {
        return element_type(boxed_collection);
    }
    match collection {
        Type::Array(array) => Some(&array.elem),
        Type::Slice(slice) => Some(&slice.elem),
        Type::Reference(reference) => element_type(&reference.elem),
        Type::Path(map) if is_map(map) => type_arguments(map).nth(1),
        Type::Path(collection) => type_arguments(collection).next(),
        _ => None,
    }
}

fn is_map(type_path: &TypePath) -> bool {
    type_path
        .path
        .segments
        .last()
        .is_some_and(|last_segment| last_segment.ident.to_string().ends_with("Map"))
}

fn type_arguments(type_path: &TypePath) -> impl Iterator<Item = &Type> {
    let arguments = match type_path
        .path
        .segments
        .last()
        .map(|last_segment| &last_segment.arguments)
    {
        Some(PathArguments::AngleBracketed(arguments)) => Some(&arguments.args),
        _ => None,
    };
    arguments.into_iter().flatten().filter_map(|argument| match argument {
        GenericArgument::Type(argument) => Some(argument),
        _ => None,
    })
}

pub fn nested_rules_in(rules: &[FieldRule]) -> Vec<Span> {
    rules
        .iter()
        .flat_map(|rule| match rule {
            FieldRule::Nested { span } => vec![*span],
            FieldRule::Each { element_rules, .. } => nested_rules_in(element_rules),
            FieldRule::Check(_) => Vec::new(),
        })
        .collect()
}

impl<'a> ValidatedField<'a> {
    pub fn nested_value_type(&self) -> Option<&'a Type> {
        nested_value_type_in(&self.rules, self.field_type)
    }
}

fn nested_value_type_in<'a>(rules: &[FieldRule], checked_type: &'a Type) -> Option<&'a Type> {
    let checked_type = option_content(checked_type).unwrap_or(checked_type);
    rules.iter().find_map(|rule| match rule {
        FieldRule::Nested { .. } => Some(checked_type),
        FieldRule::Each { element_rules, .. } => {
            element_type(checked_type).and_then(|element| nested_value_type_in(element_rules, element))
        }
        FieldRule::Check(_) => None,
    })
}

impl ValidatedField<'_> {
    pub fn local(&self, purpose: &str) -> Ident {
        match &self.member {
            Member::Named(name) => format_ident!("{}_{}", name, purpose, span = Span::mixed_site()),
            Member::Unnamed(index) => format_ident!("field_{}_{}", index.index, purpose, span = Span::mixed_site()),
        }
    }

    pub fn has_nested_values(&self) -> bool {
        !nested_rules_in(&self.rules).is_empty()
    }

    pub fn joins_nested_issues_element_by_element(&self) -> bool {
        self.has_nested_values() && !self.rules.iter().any(FieldRule::checks_only_nested_values)
    }
}

impl FieldRule {
    pub fn checks_only_nested_values(&self) -> bool {
        match self {
            Self::Nested { .. } => true,
            Self::Each { element_rules, .. } => element_rules.iter().all(Self::checks_only_nested_values),
            Self::Check(_) => false,
        }
    }

    fn from_written(written: WrittenRule, checked_type: Option<&Type>) -> syn::Result<Self> {
        match written {
            WrittenRule::Each { span, element_rules } => Self::each(element_rules, checked_type, span),
            WrittenRule::Call(call) if call.path.is_ident("nested") => Self::nested(&call),
            WrittenRule::Call(call) => Ok(Self::Check(RuleCheck::from_call(*call)?)),
        }
    }

    fn each(element_rules: Vec<WrittenRule>, checked_type: Option<&Type>, span: Span) -> syn::Result<Self> {
        let collection = checked_type.map(|checked_type| option_content(checked_type).unwrap_or(checked_type));
        let element = collection.and_then(element_type);
        Ok(Self::Each {
            element_rules: element_rules
                .into_iter()
                .map(|written| Self::from_written(written, element))
                .collect::<syn::Result<_>>()?,
            are_elements_optional: element.and_then(option_content).is_some(),
            span,
        })
    }

    fn nested(call: &RuleCall) -> syn::Result<Self> {
        if call.has_arguments() {
            return Err(Error::new_spanned(&call.path, "`nested` takes no arguments"));
        }
        Ok(Self::Nested { span: call.path.span() })
    }
}

impl RuleCheck {
    fn from_call(call: RuleCall) -> syn::Result<Self> {
        let rule_name = &call.path.segments.last().expect("a path has a segment").ident;
        let span = rule_name.span();
        if call.path.is_ident("pattern") {
            return Ok(Self {
                rule: pattern::pattern_rule(&call, span)?,
                is_required: false,
                message: call.message,
                span,
            });
        }
        let constructor = match call.path.get_ident() {
            Some(built_in) => quote_spanned!(span=> ::annotated::rules::#built_in),
            None => call.path.to_token_stream(),
        };
        let constructor_value = &call.value;
        let builder_calls = call
            .builder_arguments
            .iter()
            .map(|argument| builder_call(&call.path, argument))
            .collect::<syn::Result<Vec<_>>>()?;
        Ok(Self {
            rule: quote_spanned!(span=> #constructor(#constructor_value) #(#builder_calls)*),
            is_required: call.path.is_ident("required"),
            message: call.message,
            span,
        })
    }
}

fn builder_call(rule: &Path, NamedArgument { name, value }: &NamedArgument) -> syn::Result<TokenStream> {
    if rule.is_ident("length") && name == "unit" {
        return length_unit_call(value);
    }
    Ok(quote_spanned!(name.span()=> .#name(#value)))
}

fn length_unit_call(unit: &Expr) -> syn::Result<TokenStream> {
    let counting_method = match unit {
        Expr::Lit(ExprLit {
            lit: Lit::Str(unit), ..
        }) => match unit.value().as_str() {
            "chars" => Some("in_chars"),
            "bytes" => Some("in_bytes"),
            "graphemes" => Some("in_graphemes"),
            _ => None,
        },
        _ => None,
    };
    let counting_method =
        counting_method.ok_or_else(|| Error::new_spanned(unit, r#"`unit` is "chars", "bytes" or "graphemes""#))?;
    let counting_method = Ident::new(counting_method, unit.span());
    Ok(quote_spanned!(unit.span()=> .#counting_method()))
}
