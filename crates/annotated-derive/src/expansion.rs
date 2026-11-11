use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, quote_spanned};
use syn::{Expr, Ident};

use crate::validated_struct::{FieldRule, RuleCheck, StructCheck, ValidatedField, ValidatedStruct};

struct Expansion {
    at: Ident,
    found: Ident,
    field_at: Ident,
    value: Ident,
    found_before: Ident,
    is_reporting_all: bool,
}

pub fn validate_impl(validated_struct: &ValidatedStruct) -> TokenStream {
    let expansion = Expansion::new(validated_struct.is_reporting_all);
    let Expansion { at, found, .. } = &expansion;
    let field_checks = validated_struct.fields.iter().enumerate().map(|(index, field)| {
        if expansion.is_reporting_all || !validated_struct.is_checked_by_a_struct_rule(index) {
            return expansion.field_checks(field);
        }
        expansion.field_validity(field)
    });
    let struct_checks = validated_struct
        .struct_checks
        .iter()
        .enumerate()
        .map(|(position, check)| expansion.struct_check(validated_struct, position, check));
    let input = validated_struct.input;
    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        impl #impl_generics ::annotated::Validate for #name #type_generics #where_clause {
            fn validate_into(&self, #at: &::annotated::Location<'_>, #found: &mut ::annotated::FoundViolations) {
                #(#field_checks)*
                #(#struct_checks)*
            }
        }
    }
}

impl Expansion {
    fn new(is_reporting_all: bool) -> Self {
        let local = |name| Ident::new(name, Span::mixed_site());
        Self {
            at: local("at"),
            found: local("found"),
            field_at: local("field_at"),
            value: local("value"),
            found_before: local("found_before"),
            is_reporting_all,
        }
    }

    fn field_checks(&self, field: &ValidatedField) -> TokenStream {
        if field.rules.is_empty() {
            return TokenStream::new();
        }
        let checks = self.checks(&field.rules, field.is_optional);
        self.at_field(field, &checks)
    }

    fn field_validity(&self, field: &ValidatedField) -> TokenStream {
        let is_valid = validity_of(field);
        if field.rules.is_empty() {
            return quote!(let #is_valid = true;);
        }
        let passes_every_rule = self.passes_every_rule(&field.rules, field.is_optional);
        let checks = self.at_field(field, &passes_every_rule);
        quote!(let #is_valid = #checks;)
    }

    fn at_field(&self, field: &ValidatedField, checks: &TokenStream) -> TokenStream {
        let Self {
            at, field_at, value, ..
        } = self;
        let ValidatedField { member, name, .. } = field;
        quote! {
            {
                let #field_at = #at.field(#name);
                let #value = &self.#member;
                #checks
            }
        }
    }

    fn checks(&self, rules: &[FieldRule], is_value_optional: bool) -> TokenStream {
        if self.is_reporting_all {
            let checks = rules.iter().map(|rule| self.rule_check(rule, is_value_optional));
            return quote!(#(#checks;)*);
        }
        let passes_every_rule = self.passes_every_rule(rules, is_value_optional);
        quote!(let _ = #passes_every_rule;)
    }

    fn passes_every_rule(&self, rules: &[FieldRule], is_value_optional: bool) -> TokenStream {
        let checks = rules.iter().map(|rule| self.rule_check(rule, is_value_optional));
        quote!(#(#checks)&&*)
    }

    fn rule_check(&self, rule: &FieldRule, is_value_optional: bool) -> TokenStream {
        let check = match rule {
            FieldRule::Check(check) => self.value_check(check),
            FieldRule::Nested { span } => self.nested_check(*span),
            FieldRule::Each {
                element_rules,
                are_elements_optional,
                span,
            } => self.each_check(element_rules, *are_elements_optional, *span),
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

    fn nested_check(&self, span: Span) -> TokenStream {
        let Self {
            found,
            field_at,
            value,
            found_before,
            ..
        } = self;
        quote_spanned! {span=>
            {
                let #found_before = #found.len();
                ::annotated::Validate::validate_into(#value, &#field_at, #found);
                #found.len() == #found_before
            }
        }
    }

    fn each_check(&self, element_rules: &[FieldRule], are_elements_optional: bool, span: Span) -> TokenStream {
        let Self {
            found,
            field_at,
            value,
            found_before,
            ..
        } = self;
        let element_checks = self.checks(element_rules, are_elements_optional);
        quote_spanned! {span=>
            {
                let #found_before = #found.len();
                ::annotated::rules::HasElements::for_each_element(#value, &#field_at, |#value, #field_at| {
                    #element_checks
                });
                #found.len() == #found_before
            }
        }
    }

    fn value_check(&self, check: &RuleCheck) -> TokenStream {
        let Self {
            found, field_at, value, ..
        } = self;
        let RuleCheck {
            rule, message, span, ..
        } = check;
        let message = message_override(message.as_ref());
        quote_spanned! {*span=>
            ::annotated::__private::passes(::annotated::Rule::check(&#rule, #value), &#field_at, #found, #message)
        }
    }

    fn struct_check(&self, validated_struct: &ValidatedStruct, position: usize, check: &StructCheck) -> TokenStream {
        let at = &self.at;
        let Some(field_index) = check.field else {
            let passes = self.struct_rule_passes(check, &quote!(#at));
            return quote!(let _ = #passes;);
        };
        let field = &validated_struct.fields[field_index];
        let field_name = &field.name;
        let passes = self.struct_rule_passes(check, &quote!(&#at.field(#field_name)));
        if self.is_reporting_all {
            return quote!(let _ = #passes;);
        }
        let is_valid = validity_of(field);
        if validated_struct.is_checked_by_a_later_struct_rule(field_index, position) {
            return quote!(let #is_valid = #is_valid && #passes;);
        }
        quote!(let _ = #is_valid && #passes;)
    }

    fn struct_rule_passes(&self, check: &StructCheck, reported_at: &TokenStream) -> TokenStream {
        let found = &self.found;
        let StructCheck {
            check, message, span, ..
        } = check;
        let message = message_override(message.as_ref());
        quote_spanned! {*span=>
            ::annotated::__private::passes(
                ::annotated::Rule::check(&::annotated::rules::custom::<Self, _>(#check), self),
                #reported_at,
                #found,
                #message,
            )
        }
    }
}

fn validity_of(field: &ValidatedField) -> Ident {
    format_ident!("{}_is_valid", field.member, span = Span::mixed_site())
}

fn message_override(message: Option<&Expr>) -> TokenStream {
    if let Some(message) = message {
        quote!(::core::option::Option::Some(#message))
    } else {
        quote!(::core::option::Option::None)
    }
}
