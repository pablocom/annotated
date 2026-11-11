use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote_spanned};
use syn::spanned::Spanned;
use syn::{Data, DataStruct, DeriveInput, Error, Expr, ExprLit, Field, Fields, Ident, Lit, Path, Type};

use crate::serde_names;
use crate::written_rule::{NamedArgument, WrittenRule, validate_attributes, written_rules};

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

pub struct FieldRule {
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
            .map(FieldRule::from_written)
            .collect::<syn::Result<_>>()?;
        Ok(Self {
            member,
            name,
            is_optional: is_option(&field.ty),
            rules,
        })
    }
}

fn is_option(field_type: &Type) -> bool {
    let Type::Path(type_path) = field_type else {
        return false;
    };
    type_path
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "Option")
}

impl FieldRule {
    fn from_written(written: WrittenRule) -> syn::Result<Self> {
        let rule_name = &written.path.segments.last().expect("a path has a segment").ident;
        let span = rule_name.span();
        let constructor = match written.path.get_ident() {
            Some(built_in) => quote_spanned!(span=> ::annotated::rules::#built_in),
            None => written.path.to_token_stream(),
        };
        let constructor_value = &written.value;
        let builder_calls = written
            .builder_arguments
            .iter()
            .map(|argument| builder_call(&written.path, argument))
            .collect::<syn::Result<Vec<_>>>()?;
        Ok(Self {
            rule: quote_spanned!(span=> #constructor(#constructor_value) #(#builder_calls)*),
            is_required: written.path.is_ident("required"),
            message: written.message,
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
