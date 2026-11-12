use proc_macro2::{Literal, Span, TokenStream, TokenTree};
use quote::{ToTokens, format_ident, quote, quote_spanned};
use syn::ext::IdentExt;
use syn::spanned::Spanned;
use syn::{DeriveInput, GenericParam, Generics, Ident, Lifetime, LifetimeParam, Type, WherePredicate, parse_quote};

use crate::deserialized_struct::DeserializedStruct;
use crate::expansion::{Expansion, validity_of};
use crate::serde_names::{DefaultValue, SerdeField};
use crate::validated_struct::{FieldRule, ValidatedField, nested_rules_in, option_content};

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
    read: Ident,
    element_at: Ident,
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
            read: local("read"),
            element_at: local("element_at"),
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
                    #found: &mut ::annotated::Report,
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
        let parameters = type_and_const_parameters(&self.input().generics);
        let mut generics = self.generics_checking_fields();
        generics.params.insert(0, parse_quote!('de));
        let where_clause = generics.make_where_clause();
        for field in self.fields() {
            where_clause
                .predicates
                .extend(self.reading_bound_of(field, &parameters));
        }
        generics
    }

    fn generics_checking_fields(&self) -> Generics {
        let input = self.input();
        let parameters = type_and_const_parameters(&input.generics);
        let mut checking_bounds: Vec<_> = self
            .fields()
            .iter()
            .flat_map(|field| self.checking_bounds_of(field, &parameters))
            .collect();
        let is_default_of_its_type = matches!(self.deserialized_struct.serde.default, Some(DefaultValue::OfItsType));
        if is_default_of_its_type && !parameters.is_empty() {
            let name = &input.ident;
            let (_, type_generics, _) = input.generics.split_for_impl();
            checking_bounds.push(parse_quote!(#name #type_generics: ::core::default::Default));
        }
        let mut generics = input.generics.clone();
        if !checking_bounds.is_empty() {
            generics.make_where_clause().predicates.extend(checking_bounds);
        }
        generics
    }

    fn reading_bound_of(&self, field: &ValidatedField, parameters: &[&Ident]) -> Option<WherePredicate> {
        let serde = &self.serde;
        let field_type = field.field_type;
        if !mentions_any(field_type.to_token_stream(), parameters) {
            return None;
        }
        if field.has_nested_values() {
            return Some(parse_quote!(#field_type: ::annotated::Deserialize<'de>));
        }
        Some(parse_quote!(#field_type: #serde::Deserialize<'de>))
    }

    fn checking_bounds_of(&self, field: &ValidatedField, parameters: &[&Ident]) -> Vec<WherePredicate> {
        let is_its_type_s_default = match self.when_absent(field) {
            WhenAbsent::TakesDefault(default) => matches!(default, DefaultValue::OfItsType),
            WhenAbsent::TakesStructDefault => false,
            WhenAbsent::IsMissing | WhenAbsent::IsNone => return Vec::new(),
        };
        let mut bounds = Vec::new();
        let field_type = field.field_type;
        if is_its_type_s_default && mentions_any(field_type.to_token_stream(), parameters) {
            bounds.push(parse_quote!(#field_type: ::core::default::Default));
        }
        let validated_type = field
            .nested_value_type()
            .filter(|nested_type| mentions_any(nested_type.to_token_stream(), parameters));
        if let Some(validated_type) = validated_type {
            bounds.push(parse_quote!(#validated_type: ::annotated::Validate));
        }
        bounds
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
        let generics_checking_fields = with_lifetime(&self.generics_checking_fields(), visitor_lifetime);
        let (checking_impl_generics, _, checking_where_clause) = generics_checking_fields.split_for_impl();
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
                #found: &#visitor_lifetime mut ::annotated::Report,
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

            impl #checking_impl_generics __Visitor #visitor_type_generics #checking_where_clause {
                #checked_value
            }
        }
    }

    fn visit_seq(&self) -> TokenStream {
        let Self { serde, seq, .. } = self;
        let Expansion { found, .. } = &self.rules;
        let elements = self.fields().iter().map(|field| {
            let slot = slot_of(field);
            let slot_type = self.slot_type_of(field);
            let field_type = field.field_type;
            if !self.is_read_with_issues(field) {
                return quote! {
                    let #slot: #slot_type = #serde::de::SeqAccess::next_element::<#field_type>(&mut #seq)?;
                };
            }
            let read_issues = read_issues_of(field);
            let reading_with_its_issues = self.reading_with_its_issues(field);
            let element = quote_spanned! {field_type.span()=>
                ::annotated::__private::serde::de::SeqAccess::next_element_seed(
                    &mut #seq,
                    #reading_with_its_issues,
                )?
            };
            quote! {
                let mut #read_issues = self.#found.new_alike();
                let #slot: #slot_type = #element;
            }
        });
        let slots = self.fields().iter().map(slot_of);
        let read_issues = self.fields_read_with_issues().map(read_issues_of);
        quote! {
            #[inline]
            fn visit_seq<__A: #serde::de::SeqAccess<'de>>(
                self,
                mut #seq: __A,
            ) -> ::core::result::Result<Self::Value, __A::Error> {
                #(#elements)*
                ::core::result::Result::Ok(self.checked_value((#(#slots,)*), (#(#read_issues,)*)))
            }
        }
    }

    fn reading_with_its_issues(&self, field: &ValidatedField) -> TokenStream {
        let Expansion { at, .. } = &self.rules;
        let field_type = field.field_type;
        let name = &field.name;
        let read_issues = read_issues_of(field);
        let at_the_field = quote!(&self.#at.field(#name));
        if !self.deserialized_struct.collects_type_errors {
            return quote_spanned! {field_type.span()=>
                ::annotated::__private::DeserializeAt::<#field_type>::new(#at_the_field, &mut #read_issues)
            };
        }
        let type_name = type_name_of(field_type);
        let seed = if field.has_nested_values() {
            quote!(NestedCollectingTypeErrorsAt)
        } else {
            quote!(CollectingTypeErrorsAt)
        };
        quote_spanned! {field_type.span()=>
            ::annotated::__private::#seed::<#field_type>::new(#type_name, #at_the_field, &mut #read_issues)
        }
    }

    fn is_read_with_issues(&self, field: &ValidatedField) -> bool {
        field.has_nested_values() || self.deserialized_struct.collects_type_errors
    }

    fn fields_read_with_issues(&self) -> impl Iterator<Item = &'a ValidatedField<'a>> {
        self.fields().iter().filter(|field| self.is_read_with_issues(field))
    }

    fn visit_map(&self) -> TokenStream {
        let Self { serde, map, key, .. } = self;
        let Expansion { found, .. } = &self.rules;
        let fields = self.fields();
        let slots = fields.iter().map(slot_of);
        let slot_types = fields.iter().map(|field| self.slot_type_of(field));
        let field_arms = fields.iter().enumerate().map(|(index, field)| {
            let variant = format_ident!("Field{index}");
            let slot = slot_of(field);
            let field_type = field.field_type;
            let name = &field.name;
            let value = if self.is_read_with_issues(field) {
                let reading_with_its_issues = self.reading_with_its_issues(field);
                quote_spanned! {field_type.span()=>
                    ::annotated::__private::serde::de::MapAccess::next_value_seed(
                        &mut #map,
                        #reading_with_its_issues,
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
        let read_issues: Vec<_> = self.fields_read_with_issues().map(read_issues_of).collect();
        quote! {
            #[inline]
            fn visit_map<__A: #serde::de::MapAccess<'de>>(
                self,
                mut #map: __A,
            ) -> ::core::result::Result<Self::Value, __A::Error> {
                #(let mut #slots: #slot_types = ::core::option::Option::None;)*
                #(let mut #read_issues = self.#found.new_alike();)*
                while let ::core::option::Option::Some(#key) =
                    #serde::de::MapAccess::next_key::<__Field>(&mut #map)?
                {
                    match #key {
                        #(#field_arms)*
                        #unknown_field_arm
                    }
                }
                ::core::result::Result::Ok(self.checked_value((#(#read_slots,)*), (#(#read_issues,)*)))
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
        let slot_types = fields.iter().map(|field| self.slot_type_of(field));
        let read_issues = self.fields_read_with_issues().map(read_issues_of);
        let read_issues_types = self.fields_read_with_issues().map(|_| quote!(::annotated::Report));
        let values_of_slots = fields.iter().map(|field| self.value_of_slot(field));
        let values_from_the_struct_default = self.values_from_the_struct_default();
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
                (#(mut #read_issues,)*): (#(#read_issues_types,)*),
            ) -> ::core::option::Option<#name #type_generics> {
                let #at = self.#at;
                let #found = self.#found;
                #(#values_of_slots)*
                #values_from_the_struct_default
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
                if !self.can_be_missing(field) && !self.is_read_with_issues(field) {
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
        if field.rules.is_empty() && !self.is_read_with_issues(field) {
            return quote! {
                if ::core::option::Option::is_none(&#slot) {
                    #missing
                }
            };
        }
        let checks = if field.rules.is_empty() {
            TokenStream::new()
        } else {
            self.rules_of(field).field_rule_checks(field)
        };
        let waiting_for_the_elements = waiting_for_the_elements(field);
        let not_built_arm = not_built.map(|(pattern, not_built)| quote!(#pattern => { #not_built }));
        let missing_arm = self
            .can_be_missing(field)
            .then(|| quote!(::core::option::Option::None => { #missing }));
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
        let passes_every_rule = if field.rules.is_empty() {
            quote!(true)
        } else {
            self.rules_of(field).field_passes_every_rule(field)
        };
        let waiting_for_the_elements = waiting_for_the_elements(field);
        let not_built_arm = not_built.map(|(pattern, not_built)| quote!(#pattern => { #not_built false }));
        let missing_arm = self
            .can_be_missing(field)
            .then(|| quote!(::core::option::Option::None => { #missing false }));
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
        self.rules.reading_nested_issues_from(&read_issues_of(field))
    }

    fn slot_states(&self, field: &ValidatedField) -> SlotStates {
        let value = if field.rules.is_empty() {
            quote!(_)
        } else {
            self.rules.value.to_token_stream()
        };
        let found = &self.rules.found;
        let missing = self.missing(field);
        if !self.is_read_with_issues(field) {
            return SlotStates {
                built: quote!(::core::option::Option::Some(#value)),
                not_built: None,
                missing,
            };
        }
        let read_issues = read_issues_of(field);
        let stop_if_finished = self.rules.stop_if_finished();
        let not_built = quote! {
            #found.append(&mut #read_issues);
            #stop_if_finished
        };
        let (built, not_built_pattern) = if self.can_be_missing(field) {
            (
                quote!(::core::option::Option::Some(::core::option::Option::Some(#value))),
                quote!(::core::option::Option::Some(::core::option::Option::None)),
            )
        } else {
            (
                quote!(::core::option::Option::Some(#value)),
                quote!(::core::option::Option::None),
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
            ::annotated::__private::record_issue(
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
            match (self.is_read_with_issues(field), self.can_be_missing(field)) {
                (false, false) => quote!(#member: #slot),
                (false, true) | (true, false) => quote!(#member: #slot?),
                (true, true) => quote!(#member: ::core::option::Option::flatten(#slot)?),
            }
        });
        let name = &self.input().ident;
        quote! {
            let #deserialized = #name { #(#fields,)* };
        }
    }

    fn slot_type_of(&self, field: &ValidatedField) -> TokenStream {
        let field_type = field.field_type;
        if self.is_read_with_issues(field) {
            return quote!(::core::option::Option<::core::option::Option<#field_type>>);
        }
        quote!(::core::option::Option<#field_type>)
    }

    fn value_of_slot(&self, field: &ValidatedField) -> TokenStream {
        let slot = slot_of(field);
        let is_read_with_issues = self.is_read_with_issues(field);
        match self.when_absent(field) {
            WhenAbsent::IsMissing | WhenAbsent::TakesStructDefault => TokenStream::new(),
            WhenAbsent::IsNone if is_read_with_issues => quote! {
                let #slot = ::core::option::Option::unwrap_or(
                    #slot,
                    ::core::option::Option::Some(::core::option::Option::None),
                );
            },
            WhenAbsent::IsNone => quote!(let #slot = ::core::option::Option::flatten(#slot);),
            WhenAbsent::TakesDefault(default) => {
                let read = &self.read;
                let value_when_absent = self.value_when_absent(field, &default_value(field, default));
                quote! {
                    let #slot = match #slot {
                        ::core::option::Option::Some(#read) => #read,
                        ::core::option::Option::None => #value_when_absent,
                    };
                }
            }
        }
    }

    // The struct's default is made once, and only when a field that takes its value from it is absent.
    fn values_from_the_struct_default(&self) -> TokenStream {
        let Some(struct_default) = &self.deserialized_struct.serde.default else {
            return TokenStream::new();
        };
        let fields: Vec<_> = self
            .fields()
            .iter()
            .filter(|field| matches!(self.when_absent(field), WhenAbsent::TakesStructDefault))
            .collect();
        if fields.is_empty() {
            return TokenStream::new();
        }
        let read = &self.read;
        let slots: Vec<_> = fields.iter().map(|field| slot_of(field)).collect();
        let members = fields.iter().map(|field| field.member);
        let defaults: Vec<_> = fields.iter().map(|field| struct_default_of(field)).collect();
        let values_when_absent = fields
            .iter()
            .zip(&defaults)
            .map(|(field, default)| self.value_when_absent(field, &quote!(#default)));
        let input = self.input();
        let name = &input.ident;
        let (_, type_generics, _) = input.generics.split_for_impl();
        let struct_default_value = match struct_default {
            DefaultValue::OfItsType => quote!(<#name #type_generics as ::core::default::Default>::default()),
            DefaultValue::ReturnedBy(function) => quote_spanned!(function.span()=> #function()),
        };
        quote! {
            let (#(#slots,)*) = match (#(#slots,)*) {
                (#(::core::option::Option::Some(#slots),)*) => (#(#slots,)*),
                (#(#slots,)*) => {
                    let #name { #(#members: #defaults,)* .. } = #struct_default_value;
                    (#(
                        match #slots {
                            ::core::option::Option::Some(#read) => #read,
                            ::core::option::Option::None => #values_when_absent,
                        },
                    )*)
                }
            };
        }
    }

    fn when_absent(&self, field: &ValidatedField) -> WhenAbsent<'a> {
        if let Some(default) = &self.serde_field_of(field).default {
            return WhenAbsent::TakesDefault(default);
        }
        if self.deserialized_struct.serde.default.is_some() {
            return WhenAbsent::TakesStructDefault;
        }
        if field.is_optional {
            return WhenAbsent::IsNone;
        }
        WhenAbsent::IsMissing
    }

    fn can_be_missing(&self, field: &ValidatedField) -> bool {
        matches!(self.when_absent(field), WhenAbsent::IsMissing)
    }

    fn serde_field_of(&self, field: &ValidatedField) -> &'a SerdeField {
        let position = self
            .fields()
            .iter()
            .position(|candidate| candidate.member == field.member)
            .expect("the field is one of the struct's");
        &self.deserialized_struct.serde.fields[position]
    }

    fn value_when_absent(&self, field: &ValidatedField, default_value: &TokenStream) -> TokenStream {
        if !self.is_read_with_issues(field) {
            return default_value.clone();
        }
        if !field.has_nested_values() {
            return quote!(::core::option::Option::Some(#default_value));
        }
        let value = &self.rules.value;
        let field_type = field.field_type;
        let validating_nested_values = self.validating_nested_values(field, &quote!(&#value));
        quote! {
            {
                let #value: #field_type = #default_value;
                #validating_nested_values
                ::core::option::Option::Some(#value)
            }
        }
    }

    // Nothing waits in the field's buffer for a value annotated didn't read. Validating its nested values into the
    // buffer lets the field's rules find them there, as for a value read, so the rules are expanded once for both.
    fn validating_nested_values(&self, field: &ValidatedField, value: &TokenStream) -> TokenStream {
        let Expansion { at, .. } = &self.rules;
        let name = &field.name;
        self.validating_nested_values_in(
            &field.rules,
            field.is_optional,
            value,
            &quote!(&#at.field(#name)),
            &FieldBuffer {
                read_issues: read_issues_of(field),
                field_type_span: field.field_type.span(),
            },
        )
    }

    fn validating_nested_values_in(
        &self,
        rules: &[FieldRule],
        is_value_optional: bool,
        value: &TokenStream,
        value_at: &TokenStream,
        buffer: &FieldBuffer,
    ) -> TokenStream {
        let Self { element_at, .. } = self;
        let FieldBuffer {
            read_issues,
            field_type_span,
        } = buffer;
        let present = &self.rules.value;
        let present_value = if is_value_optional {
            quote!(#present)
        } else {
            value.clone()
        };
        let validating = rules
            .iter()
            .find_map(|rule| match rule {
                FieldRule::Nested { .. } => Some(quote_spanned! {*field_type_span=>
                    ::annotated::Validate::validate_into(#present_value, #value_at, &mut #read_issues);
                }),
                FieldRule::Each {
                    element_rules,
                    are_elements_optional,
                    ..
                } if !nested_rules_in(element_rules).is_empty() => {
                    let validating_the_element = self.validating_nested_values_in(
                        element_rules,
                        *are_elements_optional,
                        &quote!(#present),
                        &quote!(#element_at),
                        buffer,
                    );
                    Some(quote! {
                        ::annotated::rules::HasElements::for_each_element(
                            #present_value,
                            #value_at,
                            |#present, #element_at| {
                                #validating_the_element
                                true
                            },
                        );
                    })
                }
                _ => None,
            })
            .unwrap_or_default();
        if !is_value_optional {
            return validating;
        }
        quote! {
            if let ::core::option::Option::Some(#present) = #value {
                #validating
            }
        }
    }
}

enum WhenAbsent<'a> {
    IsMissing,
    IsNone,
    TakesDefault(&'a DefaultValue),
    TakesStructDefault,
}

struct FieldBuffer {
    read_issues: Ident,
    field_type_span: Span,
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

fn default_value(field: &ValidatedField, default: &DefaultValue) -> TokenStream {
    let field_type = field.field_type;
    match default {
        DefaultValue::OfItsType => {
            quote_spanned!(field_type.span()=> <#field_type as ::core::default::Default>::default())
        }
        DefaultValue::ReturnedBy(function) => quote_spanned!(function.span()=> #function()),
    }
}

fn struct_default_of(field: &ValidatedField) -> Ident {
    format_ident!("{}_struct_default", field.member, span = Span::mixed_site())
}

fn slot_of(field: &ValidatedField) -> Ident {
    format_ident!("{}_slot", field.member, span = Span::mixed_site())
}

fn waiting_for_the_elements(field: &ValidatedField) -> TokenStream {
    if !field.joins_nested_issues_element_by_element() {
        return TokenStream::new();
    }
    let read_issues = read_issues_of(field);
    quote! {
        let mut #read_issues = ::annotated::__private::ElementIssues::new(#read_issues);
    }
}

fn type_name_of(field_type: &Type) -> String {
    if let Some(content) = option_content(field_type) {
        return type_name_of(content);
    }
    match field_type {
        Type::Path(type_path) => type_path
            .path
            .segments
            .last()
            .map_or_else(String::new, |last_segment| last_segment.ident.unraw().to_string()),
        Type::Group(group) => type_name_of(&group.elem),
        Type::Paren(parenthesized) => type_name_of(&parenthesized.elem),
        Type::Reference(reference) => type_name_of(&reference.elem),
        other => other.to_token_stream().to_string(),
    }
}

fn read_issues_of(field: &ValidatedField) -> Ident {
    format_ident!("{}_read_issues", field.member, span = Span::mixed_site())
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
