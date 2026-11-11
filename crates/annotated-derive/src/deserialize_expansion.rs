use proc_macro2::{Literal, Span, TokenStream, TokenTree};
use quote::{ToTokens, format_ident, quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{DeriveInput, GenericParam, Generics, Ident, Lifetime, LifetimeParam, parse_quote};

use crate::deserialized_struct::DeserializedStruct;
use crate::expansion::{Expansion, validity_of};
use crate::validated_struct::ValidatedField;

pub fn deserialize_impl(deserialized_struct: &DeserializedStruct) -> TokenStream {
    DeserializeExpansion::new(deserialized_struct).deserialize_impl()
}

struct DeserializeExpansion<'a> {
    deserialized_struct: &'a DeserializedStruct<'a>,
    rules: Expansion,
    serde: TokenStream,
    deserializer: Ident,
    formatter: Ident,
    map: Ident,
    seq: Ident,
    key: Ident,
    deserialized: Ident,
    has_finished: Ident,
    visitor_lifetime: Lifetime,
    field_checks_label: Lifetime,
    struct_checks_label: Lifetime,
}

impl<'a> DeserializeExpansion<'a> {
    fn new(deserialized_struct: &'a DeserializedStruct<'a>) -> Self {
        let local = |name| Ident::new(name, Span::mixed_site());
        let field_checks_label = Lifetime::new("'field_checks", Span::mixed_site());
        let finishing_the_field_checks = quote!(break #field_checks_label true);
        Self {
            deserialized_struct,
            rules: Expansion::new(
                deserialized_struct.validated.is_reporting_all,
                finishing_the_field_checks,
            ),
            serde: quote!(::annotated::__private::serde),
            deserializer: local("deserializer"),
            formatter: local("formatter"),
            map: local("map"),
            seq: local("seq"),
            key: local("key"),
            deserialized: local("deserialized"),
            has_finished: local("has_finished"),
            visitor_lifetime: Lifetime::new("'__visitor", Span::call_site()),
            field_checks_label,
            struct_checks_label: Lifetime::new("'struct_checks", Span::mixed_site()),
        }
    }

    fn input(&self) -> &'a DeriveInput {
        self.deserialized_struct.validated.input
    }

    fn fields(&self) -> &'a [ValidatedField<'a>] {
        &self.deserialized_struct.validated.fields
    }

    fn deserialize_impl(&self) -> TokenStream {
        let Self {
            serde, deserializer, ..
        } = self;
        let Expansion { at, found, .. } = &self.rules;
        let input = self.input();
        let name = &input.ident;
        let generics = self.generics_reading_fields();
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, type_generics, _) = input.generics.split_for_impl();
        let turbofish = type_generics.as_turbofish();
        let struct_name = &self.deserialized_struct.serde.name;
        let field_identifier = self.field_identifier();
        let visitor = self.visitor();
        quote! {
            impl #impl_generics ::annotated::Deserialize<'de> for #name #type_generics #where_clause {
                fn deserialize_at<__D: #serde::Deserializer<'de>>(
                    #deserializer: __D,
                    #at: &::annotated::Location<'_>,
                    #found: &mut ::annotated::FoundViolations,
                ) -> ::core::result::Result<::core::option::Option<Self>, __D::Error> {
                    #field_identifier
                    #visitor
                    #serde::Deserializer::deserialize_struct(
                        #deserializer,
                        #struct_name,
                        __FIELDS,
                        __Visitor #turbofish {
                            #at,
                            #found,
                            marker: ::core::marker::PhantomData,
                        },
                    )
                }
            }

            impl #impl_generics #serde::Deserialize<'de> for #name #type_generics #where_clause {
                fn deserialize<__D: #serde::Deserializer<'de>>(
                    #deserializer: __D,
                ) -> ::core::result::Result<Self, __D::Error> {
                    ::annotated::__private::deserialize_for_serde(#deserializer)
                }
            }
        }
    }

    fn generics_reading_fields(&self) -> Generics {
        let serde = &self.serde;
        let input = self.input();
        let parameters = type_and_const_parameters(&input.generics);
        let mut generics = input.generics.clone();
        generics.params.insert(0, parse_quote!('de));
        let where_clause = generics.make_where_clause();
        for field in self.fields() {
            let field_type = field.field_type;
            if !mentions_any(field_type.to_token_stream(), &parameters) {
                continue;
            }
            if field.has_nested_values() {
                where_clause
                    .predicates
                    .push(parse_quote!(#field_type: ::annotated::Deserialize<'de>));
            } else {
                where_clause
                    .predicates
                    .push(parse_quote!(#field_type: #serde::Deserialize<'de>));
            }
        }
        generics
    }

    fn field_identifier(&self) -> TokenStream {
        let Self { serde, formatter, .. } = self;
        let value = &self.rules.value;
        let serde_fields = &self.deserialized_struct.serde.fields;
        let variants = (0..serde_fields.len()).map(|index| format_ident!("Field{index}"));
        let names_read = serde_fields.iter().map(|field| &field.names_read);
        let index_arms = variants.clone().enumerate().map(|(index, variant)| {
            let index = index as u64;
            quote!(#index => ::core::result::Result::Ok(__Field::#variant),)
        });
        let name_arms = variants
            .clone()
            .zip(names_read.clone())
            .map(|(variant, names)| quote!(#(#names)|* => ::core::result::Result::Ok(__Field::#variant),));
        let byte_arms = variants.clone().zip(names_read).map(|(variant, names)| {
            let names = names.iter().map(|name| Literal::byte_string(name.as_bytes()));
            quote!(#(#names)|* => ::core::result::Result::Ok(__Field::#variant),)
        });
        let unknown = self.unknown_field(serde_fields.len());
        let UnknownField {
            variant: unknown_variant,
            by_index,
            by_name,
            by_bytes,
        } = &unknown;
        quote! {
            enum __Field {
                #(#variants,)*
                #unknown_variant
            }

            struct __FieldVisitor;

            impl<'de> #serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;

                fn expecting(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::write_str(#formatter, "field identifier")
                }

                fn visit_u64<__E: #serde::de::Error>(self, #value: u64) -> ::core::result::Result<__Field, __E> {
                    match #value {
                        #(#index_arms)*
                        _ => #by_index,
                    }
                }

                fn visit_str<__E: #serde::de::Error>(self, #value: &str) -> ::core::result::Result<__Field, __E> {
                    match #value {
                        #(#name_arms)*
                        _ => #by_name,
                    }
                }

                fn visit_bytes<__E: #serde::de::Error>(self, #value: &[u8]) -> ::core::result::Result<__Field, __E> {
                    match #value {
                        #(#byte_arms)*
                        _ => #by_bytes,
                    }
                }
            }

            impl<'de> #serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D: #serde::Deserializer<'de>>(
                    deserializer: __D,
                ) -> ::core::result::Result<Self, __D::Error> {
                    #serde::Deserializer::deserialize_identifier(deserializer, __FieldVisitor)
                }
            }
        }
    }

    fn unknown_field(&self, field_count: usize) -> UnknownField {
        let serde = &self.serde;
        let value = &self.rules.value;
        if !self.deserialized_struct.serde.is_denying_unknown_fields {
            let ignored = quote!(::core::result::Result::Ok(__Field::Unknown));
            return UnknownField {
                variant: quote!(Unknown,),
                by_index: ignored.clone(),
                by_name: ignored.clone(),
                by_bytes: ignored,
            };
        }
        let indices_expected = format!("field index 0 <= i < {field_count}");
        UnknownField {
            variant: TokenStream::new(),
            by_index: quote! {
                ::core::result::Result::Err(<__E as #serde::de::Error>::invalid_value(
                    #serde::de::Unexpected::Unsigned(#value),
                    &#indices_expected,
                ))
            },
            by_name: quote!(::core::result::Result::Err(<__E as #serde::de::Error>::unknown_field(#value, __FIELDS))),
            by_bytes: quote! {
                {
                    let #value = &::std::string::String::from_utf8_lossy(#value);
                    ::core::result::Result::Err(<__E as #serde::de::Error>::unknown_field(#value, __FIELDS))
                }
            },
        }
    }

    fn visitor(&self) -> TokenStream {
        let Self {
            serde,
            formatter,
            visitor_lifetime,
            ..
        } = self;
        let Expansion { at, found, .. } = &self.rules;
        let input = self.input();
        let name = &input.ident;
        let (_, type_generics, _) = input.generics.split_for_impl();
        let visitor_generics = with_lifetime(&input.generics, visitor_lifetime);
        let (visitor_definition_generics, visitor_type_generics, visitor_where_clause) =
            visitor_generics.split_for_impl();
        let generics_reading_fields = with_lifetime(&self.generics_reading_fields(), visitor_lifetime);
        let (impl_generics, _, where_clause) = generics_reading_fields.split_for_impl();
        let expecting = self
            .deserialized_struct
            .serde
            .expecting
            .clone()
            .unwrap_or_else(|| format!("struct {name}"));
        let visit_seq = self.visit_seq();
        let visit_map = self.visit_map();
        let checked_value = self.checked_value();
        let names_read = self
            .deserialized_struct
            .serde
            .fields
            .iter()
            .flat_map(|field| &field.names_read);
        quote! {
            const __FIELDS: &[&str] = &[#(#names_read),*];

            struct __Visitor #visitor_definition_generics #visitor_where_clause {
                #at: &#visitor_lifetime ::annotated::Location<#visitor_lifetime>,
                #found: &#visitor_lifetime mut ::annotated::FoundViolations,
                marker: ::core::marker::PhantomData<fn() -> #name #type_generics>,
            }

            impl #impl_generics #serde::de::Visitor<'de> for __Visitor #visitor_type_generics #where_clause {
                type Value = ::core::option::Option<#name #type_generics>;

                fn expecting(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::write_str(#formatter, #expecting)
                }

                #visit_seq

                #visit_map
            }

            impl #visitor_definition_generics __Visitor #visitor_type_generics #visitor_where_clause {
                #checked_value
            }
        }
    }

    fn visit_seq(&self) -> TokenStream {
        let Self { serde, seq, .. } = self;
        let Expansion { found, .. } = &self.rules;
        let elements = self.fields().iter().map(|field| {
            let slot = slot_of(field);
            let slot_type = slot_type_of(field);
            let field_type = field.field_type;
            if !field.has_nested_values() {
                return quote! {
                    let #slot: #slot_type = #serde::de::SeqAccess::next_element::<#field_type>(&mut #seq)?;
                };
            }
            let read_violations = read_violations_of(field);
            let reading_with_its_violations = self.reading_with_its_violations(field);
            let element = quote_spanned! {field_type.span()=>
                ::annotated::__private::serde::de::SeqAccess::next_element_seed(
                    &mut #seq,
                    #reading_with_its_violations,
                )?
            };
            quote! {
                let mut #read_violations = self.#found.new_alike();
                let #slot: #slot_type = #element;
            }
        });
        let slots = self.fields().iter().map(slot_of);
        let read_violations = self.fields_with_nested_values().map(read_violations_of);
        quote! {
            #[inline]
            fn visit_seq<__A: #serde::de::SeqAccess<'de>>(
                self,
                mut #seq: __A,
            ) -> ::core::result::Result<Self::Value, __A::Error> {
                #(#elements)*
                ::core::result::Result::Ok(self.checked_value((#(#slots,)*), (#(#read_violations,)*)))
            }
        }
    }

    fn reading_with_its_violations(&self, field: &ValidatedField) -> TokenStream {
        let Expansion { at, .. } = &self.rules;
        let field_type = field.field_type;
        let name = &field.name;
        let read_violations = read_violations_of(field);
        quote_spanned! {field_type.span()=>
            ::annotated::__private::DeserializeAt::<#field_type>::new(&self.#at.field(#name), &mut #read_violations)
        }
    }

    fn fields_with_nested_values(&self) -> impl Iterator<Item = &'a ValidatedField<'a>> {
        self.fields().iter().filter(|field| field.has_nested_values())
    }

    fn visit_map(&self) -> TokenStream {
        let Self { serde, map, key, .. } = self;
        let Expansion { found, .. } = &self.rules;
        let fields = self.fields();
        let slots = fields.iter().map(slot_of);
        let slot_types = fields.iter().map(slot_type_of);
        let field_arms = fields.iter().enumerate().map(|(index, field)| {
            let variant = format_ident!("Field{index}");
            let slot = slot_of(field);
            let field_type = field.field_type;
            let name = &field.name;
            let value = if field.has_nested_values() {
                let reading_with_its_violations = self.reading_with_its_violations(field);
                quote_spanned! {field_type.span()=>
                    ::annotated::__private::serde::de::MapAccess::next_value_seed(
                        &mut #map,
                        #reading_with_its_violations,
                    )?
                }
            } else {
                quote!(#serde::de::MapAccess::next_value::<#field_type>(&mut #map)?)
            };
            quote! {
                __Field::#variant => {
                    if ::core::option::Option::is_some(&#slot) {
                        return ::core::result::Result::Err(
                            <__A::Error as #serde::de::Error>::duplicate_field(#name),
                        );
                    }
                    #slot = ::core::option::Option::Some(#value);
                }
            }
        });
        let unknown_field_arm = (!self.deserialized_struct.serde.is_denying_unknown_fields).then(|| {
            quote! {
                __Field::Unknown => {
                    #serde::de::MapAccess::next_value::<#serde::de::IgnoredAny>(&mut #map)?;
                }
            }
        });
        let read_slots = fields.iter().map(slot_of);
        let read_violations: Vec<_> = self.fields_with_nested_values().map(read_violations_of).collect();
        quote! {
            #[inline]
            fn visit_map<__A: #serde::de::MapAccess<'de>>(
                self,
                mut #map: __A,
            ) -> ::core::result::Result<Self::Value, __A::Error> {
                #(let mut #slots: #slot_types = ::core::option::Option::None;)*
                #(let mut #read_violations = self.#found.new_alike();)*
                while let ::core::option::Option::Some(#key) =
                    #serde::de::MapAccess::next_key::<__Field>(&mut #map)?
                {
                    match #key {
                        #(#field_arms)*
                        #unknown_field_arm
                    }
                }
                ::core::result::Result::Ok(self.checked_value((#(#read_slots,)*), (#(#read_violations,)*)))
            }
        }
    }

    // Always inlined into `visit_map` and `visit_seq`: left a call taking every slot, reading a valid 8-field body
    // without LTO ran 7987 instructions against 7918, and `serde_json::from_str` then `validate()` 7977. Emitting the
    // checks in each instead would compile every `pattern` twice.
    fn checked_value(&self) -> TokenStream {
        let Self {
            deserialized,
            has_finished,
            field_checks_label,
            struct_checks_label,
            ..
        } = self;
        let Expansion { at, found, .. } = &self.rules;
        let input = self.input();
        let name = &input.ident;
        let (_, type_generics, _) = input.generics.split_for_impl();
        let fields = self.fields();
        let slots = fields.iter().map(slot_of);
        let slot_types = fields.iter().map(slot_type_of);
        let read_violations = self.fields_with_nested_values().map(read_violations_of);
        let read_violations_types = self
            .fields_with_nested_values()
            .map(|_| quote!(::annotated::FoundViolations));
        let values_of_slots = fields.iter().map(value_of_slot);
        let validities = self.fields_keeping_their_validity().map(validity_of);
        let field_checks = self.field_checks();
        let build = self.build();
        let struct_checks = self
            .rules
            .stopping_with(quote!(break #struct_checks_label))
            .struct_checks(
                &self.deserialized_struct.validated,
                &quote!(&#deserialized),
                &quote!(#name #type_generics),
            );
        let (finishing, struct_checks_unless_finished) = if struct_checks.is_empty() {
            (quote!(let _ =), TokenStream::new())
        } else {
            (
                quote!(let #has_finished =),
                quote! {
                    if !#has_finished {
                        #struct_checks_label: {
                            #(#struct_checks)*
                        }
                    }
                },
            )
        };
        quote! {
            #[inline(always)]
            fn checked_value(
                self,
                (#(#slots,)*): (#(#slot_types,)*),
                (#(mut #read_violations,)*): (#(#read_violations_types,)*),
            ) -> ::core::option::Option<#name #type_generics> {
                let #at = self.#at;
                let #found = self.#found;
                #(#values_of_slots)*
                #(let mut #validities = false;)*
                #finishing #field_checks_label: {
                    #(#field_checks)*
                    false
                };
                #build
                #struct_checks_unless_finished
                ::core::option::Option::Some(#deserialized)
            }
        }
    }

    fn field_checks(&self) -> Vec<TokenStream> {
        let validated_struct = &self.deserialized_struct.validated;
        self.fields()
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let keeps_validity = self.rules.keeps_validity_of(validated_struct, index);
                if field.is_optional && !field.has_nested_values() {
                    let slot = slot_of(field);
                    let value_of_field = quote!(&#slot);
                    if keeps_validity {
                        return self.optional_field_validity(field, &value_of_field);
                    }
                    return self.rules.field_checks(field, &value_of_field);
                }
                if keeps_validity {
                    return self.read_field_validity(field);
                }
                self.read_field_checks(field)
            })
            .collect()
    }

    fn read_field_checks(&self, field: &ValidatedField) -> TokenStream {
        let slot = slot_of(field);
        let SlotStates {
            built,
            not_built,
            missing,
        } = self.slot_states(field);
        if field.rules.is_empty() {
            return quote! {
                if ::core::option::Option::is_none(&#slot) {
                    #missing
                }
            };
        }
        let checks = self.rules_of(field).field_rule_checks(field);
        let waiting_for_the_elements = waiting_for_the_elements(field);
        let not_built_arm = not_built.map(|(pattern, not_built)| quote!(#pattern => { #not_built }));
        let missing_arm = (!field.is_optional).then(|| quote!(::core::option::Option::None => { #missing }));
        quote! {
            match &#slot {
                #built => {
                    #waiting_for_the_elements
                    #checks
                }
                #not_built_arm
                #missing_arm
            }
        }
    }

    fn fields_keeping_their_validity(&self) -> impl Iterator<Item = &'a ValidatedField<'a>> {
        let validated_struct = &self.deserialized_struct.validated;
        let rules = self.rules.clone();
        self.fields()
            .iter()
            .enumerate()
            .filter(move |(index, _)| rules.keeps_validity_of(validated_struct, *index))
            .map(|(_, field)| field)
    }

    fn optional_field_validity(&self, field: &ValidatedField, value_of_field: &TokenStream) -> TokenStream {
        let is_valid = validity_of(field);
        let value = &self.rules.value;
        if field.rules.is_empty() {
            return quote!(#is_valid = true;);
        }
        let passes_every_rule = self.rules.field_passes_every_rule(field);
        quote! {
            #is_valid = {
                let #value = #value_of_field;
                #passes_every_rule
            };
        }
    }

    fn read_field_validity(&self, field: &ValidatedField) -> TokenStream {
        let slot = slot_of(field);
        let is_valid = validity_of(field);
        let SlotStates {
            built,
            not_built,
            missing,
        } = self.slot_states(field);
        let (built, passes_every_rule) = if field.rules.is_empty() {
            (quote!(::core::option::Option::Some(_)), quote!(true))
        } else {
            (built, self.rules_of(field).field_passes_every_rule(field))
        };
        let waiting_for_the_elements = waiting_for_the_elements(field);
        let not_built_arm = not_built.map(|(pattern, not_built)| quote!(#pattern => { #not_built false }));
        let missing_arm = (!field.is_optional).then(|| quote!(::core::option::Option::None => { #missing false }));
        quote! {
            #is_valid = match &#slot {
                #built => {
                    #waiting_for_the_elements
                    #passes_every_rule
                }
                #not_built_arm
                #missing_arm
            };
        }
    }

    fn rules_of(&self, field: &ValidatedField) -> Expansion {
        if !field.has_nested_values() {
            return self.rules.clone();
        }
        self.rules.reading_nested_violations_from(&read_violations_of(field))
    }

    fn slot_states(&self, field: &ValidatedField) -> SlotStates {
        let value = &self.rules.value;
        let found = &self.rules.found;
        let missing = self.missing(field);
        if !field.has_nested_values() {
            return SlotStates {
                built: quote!(::core::option::Option::Some(#value)),
                not_built: None,
                missing,
            };
        }
        let read_violations = read_violations_of(field);
        let stop_if_finished = self.rules.stop_if_finished();
        let not_built = quote! {
            #found.append(&mut #read_violations);
            #stop_if_finished
        };
        let (built, not_built_pattern) = if field.is_optional {
            (
                quote!(::core::option::Option::Some(#value)),
                quote!(::core::option::Option::None),
            )
        } else {
            (
                quote!(::core::option::Option::Some(::core::option::Option::Some(#value))),
                quote!(::core::option::Option::Some(::core::option::Option::None)),
            )
        };
        SlotStates {
            built,
            not_built: Some((not_built_pattern, not_built)),
            missing,
        }
    }

    fn missing(&self, field: &ValidatedField) -> TokenStream {
        let found = &self.rules.found;
        let missing_at = self.rules.field_location(field);
        let stop_if_finished = self.rules.stop_if_finished();
        quote! {
            ::annotated::__private::record_violation(
                #found,
                #missing_at,
                ::annotated::Problem::Missing,
                ::core::option::Option::None,
            );
            #stop_if_finished
        }
    }

    fn build(&self) -> TokenStream {
        let deserialized = &self.deserialized;
        let fields = self.fields().iter().map(|field| {
            let member = field.member;
            let slot = slot_of(field);
            match (field.has_nested_values(), field.is_optional) {
                (false, true) => quote!(#member: #slot),
                (false, false) | (true, true) => quote!(#member: #slot?),
                (true, false) => quote!(#member: ::core::option::Option::flatten(#slot)?),
            }
        });
        let name = &self.input().ident;
        quote! {
            let #deserialized = #name { #(#fields,)* };
        }
    }
}

struct SlotStates {
    built: TokenStream,
    not_built: Option<(TokenStream, TokenStream)>,
    missing: TokenStream,
}

struct UnknownField {
    variant: TokenStream,
    by_index: TokenStream,
    by_name: TokenStream,
    by_bytes: TokenStream,
}

fn slot_of(field: &ValidatedField) -> Ident {
    format_ident!("{}_slot", field.member, span = Span::mixed_site())
}

fn slot_type_of(field: &ValidatedField) -> TokenStream {
    let field_type = field.field_type;
    if field.has_nested_values() {
        return quote!(::core::option::Option<::core::option::Option<#field_type>>);
    }
    quote!(::core::option::Option<#field_type>)
}

fn value_of_slot(field: &ValidatedField) -> TokenStream {
    let slot = slot_of(field);
    match (field.has_nested_values(), field.is_optional) {
        (_, false) => TokenStream::new(),
        (false, true) => quote!(let #slot = ::core::option::Option::flatten(#slot);),
        (true, true) => quote! {
            let #slot = ::core::option::Option::unwrap_or(
                #slot,
                ::core::option::Option::Some(::core::option::Option::None),
            );
        },
    }
}

fn waiting_for_the_elements(field: &ValidatedField) -> TokenStream {
    if !field.joins_nested_violations_element_by_element() {
        return TokenStream::new();
    }
    let read_violations = read_violations_of(field);
    quote! {
        let mut #read_violations = ::annotated::__private::ElementViolations::new(#read_violations);
    }
}

fn read_violations_of(field: &ValidatedField) -> Ident {
    format_ident!("{}_read_violations", field.member, span = Span::mixed_site())
}

fn with_lifetime(generics: &Generics, lifetime: &Lifetime) -> Generics {
    let mut generics = generics.clone();
    let position = generics
        .params
        .iter()
        .take_while(|parameter| matches!(parameter, GenericParam::Lifetime(_)))
        .count();
    generics
        .params
        .insert(position, GenericParam::Lifetime(LifetimeParam::new(lifetime.clone())));
    generics
}

fn type_and_const_parameters(generics: &Generics) -> Vec<&Ident> {
    generics
        .params
        .iter()
        .filter_map(|parameter| match parameter {
            GenericParam::Type(type_parameter) => Some(&type_parameter.ident),
            GenericParam::Const(const_parameter) => Some(&const_parameter.ident),
            GenericParam::Lifetime(_) => None,
        })
        .collect()
}

fn mentions_any(tokens: TokenStream, parameters: &[&Ident]) -> bool {
    tokens.into_iter().any(|token| match token {
        TokenTree::Ident(ident) => parameters.iter().any(|parameter| ident == **parameter),
        TokenTree::Group(group) => mentions_any(group.stream(), parameters),
        TokenTree::Punct(_) | TokenTree::Literal(_) => false,
    })
}
