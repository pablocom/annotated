use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::Ident;

use crate::validated_struct::{CheckedValue, FieldRule, RuleCheck, ValidatedField, ValidatedStruct};

// Nothing for a struct with no field to describe: the trait's own `rules()` already answers that.
pub fn rules_function(validated_struct: &ValidatedStruct) -> TokenStream {
    if !cfg!(feature = "json-schema") {
        return TokenStream::new();
    }
    let description = Description::new();
    let fields: Vec<_> = validated_struct
        .fields
        .iter()
        .filter_map(|field| description.field_rules(field, validated_struct.has_default))
        .collect();
    if fields.is_empty() {
        return TokenStream::new();
    }
    quote! {
        fn rules() -> ::annotated::Rules {
            #[allow(unused_imports)]
            use ::annotated::__private::{
                ElementOfACollection as _, ElementOfAnotherValue as _, SaidByARule as _, SaidByNoRule as _,
            };
            ::annotated::Rules::of_fields([#(#fields),*])
        }
    }
}

struct Description {
    checked: Ident,
    value_rules: Ident,
}

struct ElementRules<'a> {
    rules: Vec<&'a FieldRule>,
    are_elements_optional: bool,
}

impl Description {
    fn new() -> Self {
        let local = |name| Ident::new(name, Span::mixed_site());
        Self {
            checked: local("checked"),
            value_rules: local("value_rules"),
        }
    }

    // A flattened field has no name to be described under, and a value that isn't in the input is not what a schema
    // of the input describes.
    fn field_rules(&self, field: &ValidatedField, has_its_struct_a_default: bool) -> Option<TokenStream> {
        let CheckedValue::FromTheInput { has_default } = field.checked_value else {
            return None;
        };
        if field.rules.is_empty() || field.is_flattened {
            return None;
        }
        let name = &field.name;
        let field_type = field.field_type;
        let rules: Vec<_> = field.rules.iter().collect();
        let value_rules = self.value_rules(
            &rules,
            field.is_optional,
            &quote!(::annotated::__private::Checked::<#field_type>::new()),
        );
        if has_default || has_its_struct_a_default {
            return Some(quote!(::annotated::FieldRules::new(#name, #value_rules).with_default()));
        }
        Some(quote!(::annotated::FieldRules::new(#name, #value_rules)))
    }

    fn value_rules(&self, rules: &[&FieldRule], is_value_optional: bool, checked_type: &TokenStream) -> TokenStream {
        let Self { checked, value_rules } = self;
        let (no_rules, checked_inside) = if is_value_optional {
            (quote!(::annotated::ValueRules::optional()), quote!(#checked.inside()))
        } else {
            (quote!(::annotated::ValueRules::new()), quote!(#checked))
        };
        let problems = rules.iter().filter_map(|rule| match rule {
            FieldRule::Check(required) if required.is_required => Some(self.problem_of(required, &quote!(#checked))),
            FieldRule::Check(check) => Some(self.problem_of(check, &checked_inside)),
            FieldRule::Nested { .. } | FieldRule::Each { .. } => None,
        });
        let elements = ElementRules::of_every_each_among(rules)
            .map(|element_rules| self.elements(&element_rules, &checked_inside));
        quote! {
            {
                let #checked = #checked_type;
                let #value_rules = #no_rules;
                #(#problems)*
                #elements
                #value_rules
            }
        }
    }

    fn problem_of(&self, check: &RuleCheck, checked_by_the_rule: &TokenStream) -> TokenStream {
        let value_rules = &self.value_rules;
        let RuleCheck { rule, span, .. } = check;
        quote_spanned! {*span=>
            let #value_rules = (&::annotated::__private::Said(&#rule, #checked_by_the_rule)).added_to(#value_rules);
        }
    }

    fn elements(&self, element_rules: &ElementRules, collection: &TokenStream) -> TokenStream {
        let value_rules = &self.value_rules;
        let rules_of_an_element = self.value_rules(
            &element_rules.rules,
            element_rules.are_elements_optional,
            &quote!((&#collection).element()),
        );
        quote!(let #value_rules = #value_rules.with_elements(#rules_of_an_element);)
    }
}

impl<'a> ElementRules<'a> {
    fn of_every_each_among(rules: &[&'a FieldRule]) -> Option<Self> {
        let each_rules: Vec<_> = rules
            .iter()
            .filter_map(|rule| match rule {
                FieldRule::Each {
                    element_rules,
                    are_elements_optional,
                    ..
                } => Some((element_rules, *are_elements_optional)),
                FieldRule::Check(_) | FieldRule::Nested { .. } => None,
            })
            .collect();
        let (_, are_elements_optional) = *each_rules.first()?;
        Some(Self {
            rules: each_rules
                .into_iter()
                .flat_map(|(element_rules, _)| element_rules)
                .collect(),
            are_elements_optional,
        })
    }
}
