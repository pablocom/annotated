use proc_macro2::{Group, Span, TokenStream, TokenTree};
use quote::{ToTokens, quote};
use syn::{DeriveInput, Expr, Generics, Ident};

use crate::bounds::{Bounds, Used};
use crate::described_rules::rules_function;
use crate::generated::{CratePaths, in_a_scope_of_its_own};
use crate::own_rules::{self, WithoutNested};
use crate::validated_enum::{ValidatedEnum, ValidatedVariant};
use crate::validated_struct::{
    FieldRule, Place, RuleCheck, StructCheck, ValidatedField, ValidatedStruct, nested_rules_in, patterns_compiled_once,
};

#[derive(Clone)]
pub struct Expansion {
    pub at: Ident,
    pub found: Ident,
    element_at: Ident,
    pub value: Ident,
    problem: Ident,
    found_before: Ident,
    is_reporting_all: bool,
    stop: TokenStream,
    nested_values: NestedValues,
    elements_checked_apart: Vec<Ident>,
    is_within_an_element: bool,
}

#[derive(Clone)]
enum NestedValues {
    Validated,
    ReadWithIssuesIn(Ident),
}

pub fn validate_impl(validated_struct: &ValidatedStruct, paths: &CratePaths) -> TokenStream {
    let expansion = Expansion::new(validated_struct.is_reporting_all, quote!(return));
    let Expansion { at, found, .. } = &expansion;
    let field_checks = validated_struct.fields.iter().enumerate().map(|(index, field)| {
        let member = &field.member;
        let value_of_field = quote!(&self.#member);
        if expansion.keeps_validity_of(validated_struct, index) {
            return expansion.field_validity(field, &value_of_field);
        }
        expansion.field_checks(field, &value_of_field)
    });
    let struct_checks = expansion.struct_checks(validated_struct, &quote!(self), &quote!(Self));
    let rules_function = rules_function(validated_struct);
    let input = validated_struct.input;
    let name = &input.ident;
    let generics = generics_validating(input, &validated_struct.fields);
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let (_, type_generics, _) = input.generics.split_for_impl();
    let refusals_of_unchecked_own_rules =
        own_rules::refusals(&input.generics, without_nested(&validated_struct.fields));
    let validate_impl = quote! {
        #[automatically_derived]
        impl #impl_generics _annotated::Validate for #name #type_generics #where_clause {
            fn validate_into(&self, #at: &_annotated::Location<'_>, #found: &mut _annotated::Report) {
                #(#field_checks)*
                #(#struct_checks)*
            }

            #rules_function
        }

        #refusals_of_unchecked_own_rules
    };
    in_a_scope_of_its_own(
        input,
        paths,
        &patterns_compiled_once(&validated_struct.fields),
        &validate_impl,
    )
}

pub fn validate_enum_impl(validated_enum: &ValidatedEnum, paths: &CratePaths) -> TokenStream {
    let expansion = Expansion::new(validated_enum.is_reporting_all, quote!(return));
    let Expansion { at, found, .. } = &expansion;
    let variant_checks = validated_enum
        .variants
        .iter()
        .map(|variant| expansion.variant_checks(variant));
    let enum_checks = expansion.whole_value_checks(&validated_enum.enum_checks, &quote!(self), &quote!(Self));
    let input = validated_enum.input;
    let name = &input.ident;
    let fields: Vec<_> = validated_enum
        .variants
        .iter()
        .flat_map(|variant| &variant.content.fields)
        .collect();
    let generics = generics_validating(input, fields.iter().copied());
    let (impl_generics, _, where_clause) = generics.split_for_impl();
    let (_, type_generics, _) = input.generics.split_for_impl();
    let refusals_of_unchecked_own_rules = own_rules::refusals(&input.generics, without_nested(fields.iter().copied()));
    let validate_impl = quote! {
        #[automatically_derived]
        impl #impl_generics _annotated::Validate for #name #type_generics #where_clause {
            fn validate_into(&self, #at: &_annotated::Location<'_>, #found: &mut _annotated::Report) {
                match *self {
                    #(#variant_checks)*
                }
                #(#enum_checks)*
            }
        }

        #refusals_of_unchecked_own_rules
    };
    in_a_scope_of_its_own(input, paths, &patterns_compiled_once(fields), &validate_impl)
}

// `Validate` checks a value as it is, so nothing checks the rules of its own a field's value has without `nested`.
fn without_nested<'a>(
    fields: impl IntoIterator<Item = &'a ValidatedField<'a>>,
) -> impl Iterator<Item = (&'a ValidatedField<'a>, WithoutNested)> {
    fields.into_iter().map(|field| (field, WithoutNested::Unchecked))
}

// As serde's derive bounds the type parameters a field reads, the type parameters of each nested value are bounded by
// `Validate`.
fn generics_validating<'a>(input: &DeriveInput, fields: impl IntoIterator<Item = &'a ValidatedField<'a>>) -> Generics {
    let mut bounds = Bounds::implemented_here(&input.generics, &input.ident);
    for field in fields.into_iter().filter(|field| field.has_nested_values()) {
        let nested_values = field.nested_value_type().unwrap_or(field.field_type);
        bounds.add(&Used::whole(nested_values), &quote!(_annotated::Validate));
    }
    let bounds = bounds.into_predicates();
    let mut generics = input.generics.clone();
    if !bounds.is_empty() {
        generics.make_where_clause().predicates.extend(bounds);
    }
    generics
}

impl Expansion {
    pub fn new(is_reporting_all: bool, stop: TokenStream) -> Self {
        let local = |name| Ident::new(name, Span::mixed_site());
        Self {
            at: local("at"),
            found: local("found"),
            element_at: local("element_at"),
            value: local("value"),
            problem: local("problem"),
            found_before: local("found_before"),
            is_reporting_all,
            stop,
            nested_values: NestedValues::Validated,
            elements_checked_apart: Vec::new(),
            is_within_an_element: false,
        }
    }

    pub fn stopping_with(&self, stop: TokenStream) -> Self {
        Self { stop, ..self.clone() }
    }

    pub fn reading_nested_issues_from(&self, read_issues: &Ident) -> Self {
        Self {
            nested_values: NestedValues::ReadWithIssuesIn(read_issues.clone()),
            ..self.clone()
        }
    }

    // `buffers` holds, for each `each` rule that checks more than nested values, in the order written, the issues its
    // elements had when they were checked apart, before the field's checks.
    pub fn joining_elements_checked_apart_into(&self, buffers: Vec<Ident>) -> Self {
        Self {
            elements_checked_apart: buffers,
            ..self.clone()
        }
    }

    fn has_read_nested_values(&self) -> bool {
        matches!(self.nested_values, NestedValues::ReadWithIssuesIn(_))
    }

    fn within_an_element(&self) -> Self {
        Self {
            stop: quote!(return false),
            elements_checked_apart: Vec::new(),
            is_within_an_element: true,
            ..self.clone()
        }
    }

    pub fn keeps_validity_of(&self, validated_struct: &ValidatedStruct, field: usize) -> bool {
        !self.is_reporting_all && validated_struct.is_checked_by_a_struct_rule(field)
    }

    pub fn field_checks(&self, field: &ValidatedField, value_of_field: &TokenStream) -> TokenStream {
        if field.rules.is_empty() {
            return TokenStream::new();
        }
        self.with_value(value_of_field, &self.field_rule_checks(field))
    }

    pub fn field_validity(&self, field: &ValidatedField, value_of_field: &TokenStream) -> TokenStream {
        let is_valid = validity_of(field);
        if field.rules.is_empty() {
            return quote!(let #is_valid = true;);
        }
        let checks = self.with_value(value_of_field, &self.field_passes_every_rule(field));
        quote!(let #is_valid = #checks;)
    }

    fn variant_checks(&self, variant: &ValidatedVariant) -> TokenStream {
        let at = &self.at;
        let ValidatedVariant {
            ident,
            content_place,
            content,
        } = variant;
        let checked_fields: Vec<_> = content.fields.iter().filter(|field| !field.rules.is_empty()).collect();
        let members = checked_fields.iter().map(|field| &field.member);
        let values = checked_fields.iter().map(|field| field.local("value"));
        let field_checks = content.fields.iter().enumerate().map(|(index, field)| {
            let value_of_field = field.local("value").to_token_stream();
            if self.keeps_validity_of(content, index) {
                return self.field_validity(field, &value_of_field);
            }
            self.field_checks(field, &value_of_field)
        });
        let variant_rule_checks = self.struct_checks(content, &quote!(self), &quote!(Self));
        let content_at = match content_place {
            Place::OfItsParent => TokenStream::new(),
            content_place => {
                let content_location = self.location_of(content_place);
                quote!(let #at = #content_location;)
            }
        };
        quote! {
            Self::#ident { #(#members: ref #values,)* .. } => {
                #content_at
                #(#field_checks)*
                #(#variant_rule_checks)*
            }
        }
    }

    pub fn field_rule_checks(&self, field: &ValidatedField) -> TokenStream {
        self.checks(&field.rules, field.is_optional, &self.field_location(field))
    }

    pub fn field_passes_every_rule(&self, field: &ValidatedField) -> TokenStream {
        self.passes_every_rule(&field.rules, field.is_optional, &self.field_location(field))
    }

    fn with_value(&self, value_of_field: &TokenStream, checks: &TokenStream) -> TokenStream {
        let value = &self.value;
        quote! {
            {
                let #value = #value_of_field;
                #checks
            }
        }
    }

    pub fn field_location(&self, field: &ValidatedField) -> TokenStream {
        self.location_of(&field.place)
    }

    fn location_of(&self, place: &Place) -> TokenStream {
        let at = &self.at;
        match place {
            Place::Named(name) => quote!(&#at.field(#name)),
            Place::Indexed(index) => quote!(&#at.index(#index)),
            Place::OfItsParent => quote!(#at),
        }
    }

    fn checks(&self, rules: &[FieldRule], is_value_optional: bool, value_at: &TokenStream) -> TokenStream {
        if self.is_reporting_all {
            let checks = self.rule_checks(rules, is_value_optional, value_at);
            return quote!(#(let _ = #checks;)*);
        }
        let passes_every_rule = self.passes_every_rule(rules, is_value_optional, value_at);
        quote!(let _ = #passes_every_rule;)
    }

    fn passes_every_rule(&self, rules: &[FieldRule], is_value_optional: bool, value_at: &TokenStream) -> TokenStream {
        let checks = self.rule_checks(rules, is_value_optional, value_at);
        quote!((#(#checks)&&*))
    }

    // An `each` whose elements were checked apart joins their issues where it is written.
    fn rule_checks(&self, rules: &[FieldRule], is_value_optional: bool, value_at: &TokenStream) -> Vec<TokenStream> {
        let mut buffers = self.elements_checked_apart.iter();
        rules
            .iter()
            .map(|rule| {
                let buffer = rule.is_each_checking_more_than_nested_values().then(|| buffers.next());
                match buffer.flatten() {
                    Some(buffer) => self.joined(buffer),
                    None => self.rule_check(rule, is_value_optional, value_at),
                }
            })
            .collect()
    }

    fn rule_check(&self, rule: &FieldRule, is_value_optional: bool, value_at: &TokenStream) -> TokenStream {
        let check = match rule {
            FieldRule::Check(check) => self.value_check(check, value_at),
            FieldRule::Nested { span } => self.nested_check(*span, value_at),
            FieldRule::Each { span, .. } if self.has_read_nested_values() && rule.checks_only_nested_values() => {
                self.nested_check(*span, value_at)
            }
            FieldRule::Each {
                element_rules,
                are_elements_optional,
                span,
            } => self.each_check(element_rules, *are_elements_optional, *span, value_at),
        };
        let checks_the_option_itself = matches!(rule, FieldRule::Check(RuleCheck { is_required: true, .. }));
        if !is_value_optional || checks_the_option_itself {
            return check;
        }
        let value = &self.value;
        quote! {
            match #value {
                ::core::option::Option::Some(#value) => #check,
                ::core::option::Option::None => true,
            }
        }
    }

    fn nested_check(&self, span: Span, value_at: &TokenStream) -> TokenStream {
        let Self {
            found,
            value,
            found_before,
            ..
        } = self;
        let NestedValues::ReadWithIssuesIn(read_issues) = &self.nested_values else {
            return quote_at! {span=>
                {
                    let #found_before = #found.len();
                    _annotated::Validate::validate_into(#value, #value_at, #found);
                    #found.len() == #found_before
                }
            };
        };
        if self.is_within_an_element {
            let after_an_issue = self.after_an_issue();
            return quote! {
                if #read_issues.join_those_at(#value_at, #found) {
                    #after_an_issue
                } else {
                    true
                }
            };
        }
        self.joined(read_issues)
    }

    // The issues waiting in `buffer`, appended to `found`, as a failure of the field when there are any.
    fn joined(&self, buffer: &Ident) -> TokenStream {
        let found = &self.found;
        let after_an_issue = self.after_an_issue();
        quote! {
            if #buffer.is_empty() {
                true
            } else {
                #found.append(&mut #buffer);
                #after_an_issue
            }
        }
    }

    fn each_check(
        &self,
        element_rules: &[FieldRule],
        are_elements_optional: bool,
        span: Span,
        value_at: &TokenStream,
    ) -> TokenStream {
        let Self {
            found, found_before, ..
        } = self;
        let elements_checked = self.elements_checked(element_rules, are_elements_optional, span, value_at);
        let after_an_issue = self.after_an_issue();
        quote_at! {span=>
            {
                let #found_before = #found.len();
                #elements_checked
                if #found.len() == #found_before {
                    true
                } else {
                    #after_an_issue
                }
            }
        }
    }

    // Every element of the collection `value` holds checked by the rules of an `each`, into `found`.
    fn elements_checked(
        &self,
        element_rules: &[FieldRule],
        are_elements_optional: bool,
        span: Span,
        value_at: &TokenStream,
    ) -> TokenStream {
        let Self {
            found,
            element_at,
            value,
            ..
        } = self;
        let within_an_element = self.within_an_element();
        let element_checks = within_an_element.checks(element_rules, are_elements_optional, &quote!(#element_at));
        let (skipping_the_element_s_read_issues, joining_the_read_issues_left) = match &self.nested_values {
            NestedValues::ReadWithIssuesIn(read_issues) if !nested_rules_in(element_rules).is_empty() => (
                quote!(#read_issues.skip_those_at(#element_at);),
                quote!(#read_issues.join_those_at(#value_at, #found);),
            ),
            _ => (TokenStream::new(), TokenStream::new()),
        };
        quote_at! {span=>
            _annotated::rules::HasElements::for_each_element(#value, #value_at, |#value, #element_at| {
                #element_checks
                #skipping_the_element_s_read_issues
                true
            });
            #joining_the_read_issues_left
        }
    }

    // The elements of the collection that `collection` refers to, checked by each `each` rule among `rules` that checks
    // more than nested values, into the buffer of its own `elements_checked_apart` holds for it, in the same order.
    pub fn elements_checked_apart(
        &self,
        rules: &[FieldRule],
        collection: &TokenStream,
        value_at: &TokenStream,
    ) -> TokenStream {
        let Self { found, value, .. } = self;
        let checking_into_their_buffers = rules
            .iter()
            .filter(|rule| rule.is_each_checking_more_than_nested_values())
            .filter_map(|rule| match rule {
                FieldRule::Each {
                    element_rules,
                    are_elements_optional,
                    span,
                } => Some((element_rules, *are_elements_optional, *span)),
                FieldRule::Check(_) | FieldRule::Nested { .. } => None,
            })
            .zip(&self.elements_checked_apart)
            .map(|((element_rules, are_elements_optional, span), buffer)| {
                let elements_checked = self.elements_checked(element_rules, are_elements_optional, span, value_at);
                quote! {
                    {
                        let #found = &mut #buffer;
                        #elements_checked
                    }
                }
            });
        quote! {
            {
                let #value = #collection;
                #(#checking_into_their_buffers)*
            }
        }
    }

    fn value_check(&self, check: &RuleCheck, value_at: &TokenStream) -> TokenStream {
        let RuleCheck {
            rule, message, span, ..
        } = check;
        let value = &self.value;
        let outcome = quote_at!(*span=> _annotated::Rule::check(&#rule, #value));
        self.passes_or_records(&outcome, value_at, message.as_ref(), *span)
    }

    fn passes_or_records(
        &self,
        outcome: &TokenStream,
        reported_at: &TokenStream,
        message: Option<&Expr>,
        span: Span,
    ) -> TokenStream {
        let Self { found, problem, .. } = self;
        let message = message_override(message);
        let after_an_issue = self.after_an_issue();
        quote_at! {span=>
            match #outcome {
                ::core::result::Result::Ok(()) => true,
                ::core::result::Result::Err(#problem) => {
                    _annotated::__private::record_issue(#found, #reported_at, #problem, #message);
                    #after_an_issue
                }
            }
        }
    }

    fn after_an_issue(&self) -> TokenStream {
        let stop_if_finished = self.stop_if_finished();
        quote! {
            #stop_if_finished
            false
        }
    }

    pub fn stop_if_finished(&self) -> TokenStream {
        let Self { found, stop, .. } = self;
        quote! {
            if #found.is_finished() {
                #stop;
            }
        }
    }

    pub fn struct_checks(
        &self,
        validated_struct: &ValidatedStruct,
        checked: &TokenStream,
        checked_type: &TokenStream,
    ) -> Vec<TokenStream> {
        let checked = CheckedStruct {
            value: checked,
            value_type: checked_type,
        };
        validated_struct
            .struct_checks
            .iter()
            .enumerate()
            .map(|(position, check)| self.struct_check(validated_struct, position, check, &checked))
            .collect()
    }

    fn struct_check(
        &self,
        validated_struct: &ValidatedStruct,
        position: usize,
        check: &StructCheck,
        checked: &CheckedStruct,
    ) -> TokenStream {
        let Some(field_index) = check.field else {
            return self.whole_value_check(check, checked);
        };
        let field = &validated_struct.fields[field_index];
        let passes = self.struct_rule_passes(check, checked, &self.field_location(field));
        if self.is_reporting_all {
            return quote!(let _ = #passes;);
        }
        let is_valid = validity_of(field);
        if validated_struct.is_checked_by_a_later_struct_rule(field_index, position) {
            return quote!(let #is_valid = #is_valid && #passes;);
        }
        quote!(let _ = #is_valid && #passes;)
    }

    pub fn whole_value_checks(
        &self,
        checks: &[StructCheck],
        checked: &TokenStream,
        checked_type: &TokenStream,
    ) -> Vec<TokenStream> {
        let checked = CheckedStruct {
            value: checked,
            value_type: checked_type,
        };
        checks
            .iter()
            .map(|check| self.whole_value_check(check, &checked))
            .collect()
    }

    fn whole_value_check(&self, check: &StructCheck, checked: &CheckedStruct) -> TokenStream {
        let at = &self.at;
        let passes = self.struct_rule_passes(check, checked, &quote!(#at));
        quote!(let _ = #passes;)
    }

    fn struct_rule_passes(
        &self,
        check: &StructCheck,
        checked: &CheckedStruct,
        reported_at: &TokenStream,
    ) -> TokenStream {
        let StructCheck {
            check, message, span, ..
        } = check;
        let CheckedStruct { value, value_type } = checked;
        let value_type = spanned(value_type, Span::call_site().located_at(*span));
        let outcome = quote_at!(*span=>
            _annotated::Rule::check(&_annotated::rules::custom::<#value_type, _>(#check), #value)
        );
        self.passes_or_records(&outcome, reported_at, message.as_ref(), *span)
    }
}

struct CheckedStruct<'a> {
    value: &'a TokenStream,
    value_type: &'a TokenStream,
}

pub fn validity_of(field: &ValidatedField) -> Ident {
    field.local("is_valid")
}

pub fn spanned(tokens: &TokenStream, span: Span) -> TokenStream {
    tokens
        .clone()
        .into_iter()
        .map(|mut token| {
            if let TokenTree::Group(group) = &token {
                let mut respanned_group = Group::new(group.delimiter(), spanned(&group.stream(), span));
                respanned_group.set_span(span);
                return TokenTree::Group(respanned_group);
            }
            token.set_span(span);
            token
        })
        .collect()
}

pub fn message_override(message: Option<&Expr>) -> TokenStream {
    if let Some(message) = message {
        quote!(::core::option::Option::Some(#message))
    } else {
        quote!(::core::option::Option::None)
    }
}
