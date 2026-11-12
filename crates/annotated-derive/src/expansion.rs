use proc_macro2::{Group, Span, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned};
use syn::{Expr, Ident};

use crate::validated_struct::{FieldRule, RuleCheck, StructCheck, ValidatedField, ValidatedStruct, nested_rules_in};

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
    is_within_an_element: bool,
}

#[derive(Clone)]
enum NestedValues {
    Validated,
    ReadWithIssuesIn(Ident),
}

pub fn validate_impl(validated_struct: &ValidatedStruct) -> TokenStream {
    let expansion = Expansion::new(validated_struct.is_reporting_all, quote!(return));
    let Expansion { at, found, .. } = &expansion;
    let field_checks = validated_struct.fields.iter().enumerate().map(|(index, field)| {
        let member = field.member;
        let value_of_field = quote!(&self.#member);
        if expansion.keeps_validity_of(validated_struct, index) {
            return expansion.field_validity(field, &value_of_field);
        }
        expansion.field_checks(field, &value_of_field)
    });
    let struct_checks = expansion.struct_checks(validated_struct, &quote!(self), &quote!(Self));
    let input = validated_struct.input;
    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        impl #impl_generics ::annotated::Validate for #name #type_generics #where_clause {
            fn validate_into(&self, #at: &::annotated::Location<'_>, #found: &mut ::annotated::Report) {
                #(#field_checks)*
                #(#struct_checks)*
            }
        }
    }
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

    fn has_read_nested_values(&self) -> bool {
        matches!(self.nested_values, NestedValues::ReadWithIssuesIn(_))
    }

    fn within_an_element(&self) -> Self {
        Self {
            stop: quote!(return false),
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
        let at = &self.at;
        let name = &field.name;
        quote!(&#at.field(#name))
    }

    fn checks(&self, rules: &[FieldRule], is_value_optional: bool, value_at: &TokenStream) -> TokenStream {
        if self.is_reporting_all {
            let checks = rules
                .iter()
                .map(|rule| self.rule_check(rule, is_value_optional, value_at));
            return quote!(#(let _ = #checks;)*);
        }
        let passes_every_rule = self.passes_every_rule(rules, is_value_optional, value_at);
        quote!(let _ = #passes_every_rule;)
    }

    fn passes_every_rule(&self, rules: &[FieldRule], is_value_optional: bool, value_at: &TokenStream) -> TokenStream {
        let checks = rules
            .iter()
            .map(|rule| self.rule_check(rule, is_value_optional, value_at));
        quote!((#(#checks)&&*))
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
            return quote_spanned! {span=>
                {
                    let #found_before = #found.len();
                    ::annotated::Validate::validate_into(#value, #value_at, #found);
                    #found.len() == #found_before
                }
            };
        };
        let after_an_issue = self.after_an_issue();
        if self.is_within_an_element {
            return quote! {
                if #read_issues.join_those_at(#value_at, #found) {
                    #after_an_issue
                } else {
                    true
                }
            };
        }
        quote! {
            if #read_issues.is_empty() {
                true
            } else {
                #found.append(&mut #read_issues);
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
            found,
            element_at,
            value,
            found_before,
            ..
        } = self;
        let within_an_element = self.within_an_element();
        let element_checks = within_an_element.checks(element_rules, are_elements_optional, &quote!(#element_at));
        let after_an_issue = self.after_an_issue();
        let (skipping_the_element_s_read_issues, joining_the_read_issues_left) = match &self.nested_values {
            NestedValues::ReadWithIssuesIn(read_issues) if !nested_rules_in(element_rules).is_empty() => (
                quote!(#read_issues.skip_those_at(#element_at);),
                quote!(#read_issues.join_those_at(#value_at, #found);),
            ),
            _ => (TokenStream::new(), TokenStream::new()),
        };
        quote_spanned! {span=>
            {
                let #found_before = #found.len();
                ::annotated::rules::HasElements::for_each_element(#value, #value_at, |#value, #element_at| {
                    #element_checks
                    #skipping_the_element_s_read_issues
                    true
                });
                #joining_the_read_issues_left
                if #found.len() == #found_before {
                    true
                } else {
                    #after_an_issue
                }
            }
        }
    }

    fn value_check(&self, check: &RuleCheck, value_at: &TokenStream) -> TokenStream {
        let RuleCheck {
            rule, message, span, ..
        } = check;
        let value = &self.value;
        let outcome = quote_spanned!(*span=> ::annotated::Rule::check(&#rule, #value));
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
        quote_spanned! {span=>
            match #outcome {
                ::core::result::Result::Ok(()) => true,
                ::core::result::Result::Err(#problem) => {
                    ::annotated::__private::record_issue(#found, #reported_at, #problem, #message);
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
        let at = &self.at;
        let Some(field_index) = check.field else {
            let passes = self.struct_rule_passes(check, checked, &quote!(#at));
            return quote!(let _ = #passes;);
        };
        let field = &validated_struct.fields[field_index];
        let field_name = &field.name;
        let passes = self.struct_rule_passes(check, checked, &quote!(&#at.field(#field_name)));
        if self.is_reporting_all {
            return quote!(let _ = #passes;);
        }
        let is_valid = validity_of(field);
        if validated_struct.is_checked_by_a_later_struct_rule(field_index, position) {
            return quote!(let #is_valid = #is_valid && #passes;);
        }
        quote!(let _ = #is_valid && #passes;)
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
        let value_type = spanned(value_type, *span);
        let outcome = quote_spanned!(*span=>
            ::annotated::Rule::check(&::annotated::rules::custom::<#value_type, _>(#check), #value)
        );
        self.passes_or_records(&outcome, reported_at, message.as_ref(), *span)
    }
}

struct CheckedStruct<'a> {
    value: &'a TokenStream,
    value_type: &'a TokenStream,
}

pub fn validity_of(field: &ValidatedField) -> Ident {
    format_ident!("{}_is_valid", field.member, span = Span::mixed_site())
}

fn spanned(tokens: &TokenStream, span: Span) -> TokenStream {
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

fn message_override(message: Option<&Expr>) -> TokenStream {
    if let Some(message) = message {
        quote!(::core::option::Option::Some(#message))
    } else {
        quote!(::core::option::Option::None)
    }
}
