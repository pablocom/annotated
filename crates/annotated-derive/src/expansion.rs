use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
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
    let field_checks = validated_struct
        .fields
        .iter()
        .filter(|field| !field.rules.is_empty())
        .map(|field| expansion.field_checks(field));
    let struct_checks = validated_struct
        .struct_checks
        .iter()
        .map(|check| expansion.struct_check(check));
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
        let Self {
            at, field_at, value, ..
        } = self;
        let ValidatedField { member, name, .. } = field;
        let checks = self.checks(&field.rules, field.is_optional);
        quote! {
            {
                let #field_at = #at.field(#name);
                let #value = &self.#member;
                #checks
            }
        }
    }

    fn checks(&self, rules: &[FieldRule], is_value_optional: bool) -> TokenStream {
        let checks = rules.iter().map(|rule| self.rule_check(rule, is_value_optional));
        if self.is_reporting_all {
            return quote!(#(#checks;)*);
        }
        quote!(let _ = #(#checks)&&*;)
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

    fn struct_check(&self, check: &StructCheck) -> TokenStream {
        let Self { at, found, .. } = self;
        let StructCheck {
            check,
            field_name,
            message,
            span,
        } = check;
        let reported_at = if let Some(field_name) = field_name {
            quote!(&#at.field(#field_name))
        } else {
            quote!(#at)
        };
        let message = message_override(message.as_ref());
        quote_spanned! {*span=>
            let _ = ::annotated::__private::passes(
                ::annotated::Rule::check(&::annotated::rules::custom::<Self, _>(#check), self),
                #reported_at,
                #found,
                #message,
            );
        }
    }
}

fn message_override(message: Option<&Expr>) -> TokenStream {
    if let Some(message) = message {
        quote!(::core::option::Option::Some(#message))
    } else {
        quote!(::core::option::Option::None)
    }
}
