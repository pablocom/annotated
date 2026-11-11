use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::Ident;

use crate::validated_struct::{FieldRule, ValidatedField, ValidatedStruct};

struct Locals {
    at: Ident,
    found: Ident,
    field_at: Ident,
    value: Ident,
}

impl Locals {
    fn new() -> Self {
        let local = |name| Ident::new(name, Span::mixed_site());
        Self {
            at: local("at"),
            found: local("found"),
            field_at: local("field_at"),
            value: local("value"),
        }
    }
}

pub fn validate_impl(validated_struct: &ValidatedStruct) -> TokenStream {
    let locals = Locals::new();
    let Locals { at, found, .. } = &locals;
    let field_checks = validated_struct
        .fields
        .iter()
        .filter(|field| !field.rules.is_empty())
        .map(|field| field_checks(field, validated_struct.is_reporting_all, &locals));
    let input = validated_struct.input;
    let name = &input.ident;
    let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();
    quote! {
        impl #impl_generics ::annotated::Validate for #name #type_generics #where_clause {
            fn validate_into(&self, #at: &::annotated::Location<'_>, #found: &mut ::annotated::FoundViolations) {
                #(#field_checks)*
            }
        }
    }
}

fn field_checks(field: &ValidatedField, is_reporting_all: bool, locals: &Locals) -> TokenStream {
    let Locals {
        at, field_at, value, ..
    } = locals;
    let ValidatedField { member, name, .. } = field;
    let checks = field
        .rules
        .iter()
        .map(|rule| rule_check(rule, field.is_optional, locals));
    let run_checks = if is_reporting_all {
        quote!(#(#checks;)*)
    } else {
        quote!(let _ = #(#checks)&&*;)
    };
    quote! {
        {
            let #field_at = #at.field(#name);
            let #value = &self.#member;
            #run_checks
        }
    }
}

fn rule_check(rule: &FieldRule, is_optional_field: bool, locals: &Locals) -> TokenStream {
    let Locals {
        found, field_at, value, ..
    } = locals;
    let FieldRule {
        rule: constructed_rule,
        message,
        span,
        ..
    } = rule;
    let message = if let Some(message) = message {
        quote!(::core::option::Option::Some(#message))
    } else {
        quote!(::core::option::Option::None)
    };
    let check = quote_spanned! {*span=>
        ::annotated::__private::passes(::annotated::Rule::check(&#constructed_rule, #value), &#field_at, #found, #message)
    };
    if !is_optional_field || rule.is_required {
        return check;
    }
    quote! {
        match #value {
            ::core::option::Option::Some(#value) => #check,
            ::core::option::Option::None => true,
        }
    }
}
