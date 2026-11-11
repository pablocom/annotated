use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote_spanned};
use syn::spanned::Spanned;
use syn::{
    Data, DataStruct, DeriveInput, Error, Expr, ExprLit, Field, Fields, GenericArgument, Ident, Lit, Path,
    PathArguments, Type, TypePath,
};

use crate::written_rule::{NamedArgument, RuleCall, WrittenRule, validate_attributes, written_rules};
use crate::{pattern, serde_names};

pub struct ValidatedStruct<'a> {
    pub input: &'a DeriveInput,
    pub fields: Vec<ValidatedField<'a>>,
    pub is_reporting_all: bool,
}

pub struct ValidatedField<'a> {
    pub member: &'a Ident,
    pub name: String,
    pub is_optional: bool,
    pub rules: Vec<FieldRule>,
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
    pub fn from_input(input: &'a DeriveInput) -> syn::Result<Self> {
        let Data::Struct(DataStruct {
            fields: Fields::Named(fields),
            ..
        }) = &input.data
        else {
            return Err(Error::new_spanned(
                &input.ident,
                "`Validate` can only be derived for a struct with named fields",
            ));
        };
        let fields = fields
            .named
            .iter()
            .zip(serde_names::field_names(input)?)
            .map(|(field, name)| ValidatedField::from_field(field, name))
            .collect::<syn::Result<_>>()?;
        Ok(Self {
            input,
            fields,
            is_reporting_all: is_reporting_all(input)?,
        })
    }
}

fn is_reporting_all(input: &DeriveInput) -> syn::Result<bool> {
    let mut is_reporting_all = false;
    for attribute in validate_attributes(&input.attrs) {
        attribute.parse_nested_meta(|option| {
            if !option.path.is_ident("report_all") {
                return Err(option.error("a struct's `#[validate(...)]` takes `report_all`"));
            }
            is_reporting_all = true;
            Ok(())
        })?;
    }
    Ok(is_reporting_all)
}

impl<'a> ValidatedField<'a> {
    fn from_field(field: &'a Field, name: String) -> syn::Result<Self> {
        let member = field.ident.as_ref().expect("a named field has a name");
        let rules = written_rules(&field.attrs)?
            .into_iter()
            .map(|written| FieldRule::from_written(written, Some(&field.ty)))
            .collect::<syn::Result<_>>()?;
        Ok(Self {
            member,
            name,
            is_optional: option_content(&field.ty).is_some(),
            rules,
        })
    }
}

fn option_content(checked_type: &Type) -> Option<&Type> {
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
    last_type_argument(type_path)
}

fn element_type(collection: &Type) -> Option<&Type> {
    if let Some(boxed_collection) = content_of("Box", collection) {
        return element_type(boxed_collection);
    }
    match collection {
        Type::Array(array) => Some(&array.elem),
        Type::Slice(slice) => Some(&slice.elem),
        Type::Reference(reference) => element_type(&reference.elem),
        Type::Path(type_path) => last_type_argument(type_path),
        _ => None,
    }
}

fn last_type_argument(type_path: &TypePath) -> Option<&Type> {
    let PathArguments::AngleBracketed(arguments) = &type_path.path.segments.last()?.arguments else {
        return None;
    };
    arguments
        .args
        .iter()
        .filter_map(|argument| match argument {
            GenericArgument::Type(argument) => Some(argument),
            _ => None,
        })
        .next_back()
}

impl FieldRule {
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
