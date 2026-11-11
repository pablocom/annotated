use proc_macro2::{Literal, Span, TokenStream, TokenTree};
use quote::{ToTokens, format_ident, quote};
use syn::{DeriveInput, GenericParam, Generics, Ident, parse_quote};

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
}

impl<'a> DeserializeExpansion<'a> {
    fn new(deserialized_struct: &'a DeserializedStruct<'a>) -> Self {
        let local = |name| Ident::new(name, Span::mixed_site());
        let not_built = quote!(return ::core::result::Result::Ok(::core::option::Option::None));
        Self {
            deserialized_struct,
            rules: Expansion::new(deserialized_struct.validated.is_reporting_all, not_built),
            serde: quote!(::annotated::__private::serde),
            deserializer: local("deserializer"),
            formatter: local("formatter"),
            map: local("map"),
            seq: local("seq"),
            key: local("key"),
            deserialized: local("deserialized"),
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
            serde,
            deserializer,
            deserialized,
            ..
        } = self;
        let Expansion { at, found, .. } = &self.rules;
        let input = self.input();
        let name = &input.ident;
        let generics = self.generics_reading_fields();
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, type_generics, _) = input.generics.split_for_impl();
        let field_identifier = self.field_identifier();
        let visitor = self.visitor();
        let read_fields = self.read_fields();
        let field_checks = self.field_checks();
        let build = self.build();
        let struct_checks = self
            .rules
            .struct_checks(&self.deserialized_struct.validated, &quote!(&#deserialized));
        quote! {
            impl #impl_generics ::annotated::Deserialize<'de> for #name #type_generics #where_clause {
                fn deserialize_at<__D: #serde::Deserializer<'de>>(
                    #deserializer: __D,
                    #at: &::annotated::Location<'_>,
                    #found: &mut ::annotated::FoundViolations,
                ) -> ::core::result::Result<::core::option::Option<Self>, __D::Error> {
                    #field_identifier
                    #visitor
                    #read_fields
                    #(#field_checks)*
                    #build
                    #(#struct_checks)*
                    ::core::result::Result::Ok(::core::option::Option::Some(#deserialized))
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
            if mentions_any(field_type.to_token_stream(), &parameters) {
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
        let Self { serde, formatter, .. } = self;
        let input = self.input();
        let name = &input.ident;
        let (struct_generics, type_generics, struct_where_clause) = input.generics.split_for_impl();
        let generics = self.generics_reading_fields();
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let field_types = self.fields().iter().map(|field| field.field_type);
        let expecting = self
            .deserialized_struct
            .serde
            .expecting
            .clone()
            .unwrap_or_else(|| format!("struct {name}"));
        let visit_seq = self.visit_seq();
        let visit_map = self.visit_map();
        let names_read = self
            .deserialized_struct
            .serde
            .fields
            .iter()
            .flat_map(|field| &field.names_read);
        quote! {
            const __FIELDS: &[&str] = &[#(#names_read),*];

            struct __Visitor #struct_generics #struct_where_clause {
                marker: ::core::marker::PhantomData<fn() -> #name #type_generics>,
            }

            impl #impl_generics #serde::de::Visitor<'de> for __Visitor #type_generics #where_clause {
                type Value = (#(::core::option::Option<#field_types>,)*);

                fn expecting(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::write_str(#formatter, #expecting)
                }

                #visit_seq

                #visit_map
            }
        }
    }

    fn visit_seq(&self) -> TokenStream {
        let Self { serde, seq, .. } = self;
        let fields = self.fields();
        let elements = fields.iter().map(|field| {
            let slot = slot_of(field);
            let field_type = field.field_type;
            quote! {
                let #slot: ::core::option::Option<#field_type> =
                    #serde::de::SeqAccess::next_element::<#field_type>(&mut #seq)?;
            }
        });
        let slots = fields.iter().map(slot_of);
        quote! {
            fn visit_seq<__A: #serde::de::SeqAccess<'de>>(
                self,
                mut #seq: __A,
            ) -> ::core::result::Result<Self::Value, __A::Error> {
                #(#elements)*
                ::core::result::Result::Ok((#(#slots,)*))
            }
        }
    }

    fn visit_map(&self) -> TokenStream {
        let Self { serde, map, key, .. } = self;
        let fields = self.fields();
        let slots = fields.iter().map(slot_of);
        let field_types = fields.iter().map(|field| field.field_type);
        let field_arms = fields.iter().enumerate().map(|(index, field)| {
            let variant = format_ident!("Field{index}");
            let slot = slot_of(field);
            let field_type = field.field_type;
            let name = &field.name;
            quote! {
                __Field::#variant => {
                    if ::core::option::Option::is_some(&#slot) {
                        return ::core::result::Result::Err(
                            <__A::Error as #serde::de::Error>::duplicate_field(#name),
                        );
                    }
                    #slot = ::core::option::Option::Some(
                        #serde::de::MapAccess::next_value::<#field_type>(&mut #map)?,
                    );
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
        quote! {
            fn visit_map<__A: #serde::de::MapAccess<'de>>(
                self,
                mut #map: __A,
            ) -> ::core::result::Result<Self::Value, __A::Error> {
                #(let mut #slots: ::core::option::Option<#field_types> = ::core::option::Option::None;)*
                while let ::core::option::Option::Some(#key) =
                    #serde::de::MapAccess::next_key::<__Field>(&mut #map)?
                {
                    match #key {
                        #(#field_arms)*
                        #unknown_field_arm
                    }
                }
                ::core::result::Result::Ok((#(#read_slots,)*))
            }
        }
    }

    fn read_fields(&self) -> TokenStream {
        let Self {
            serde, deserializer, ..
        } = self;
        let (_, type_generics, _) = self.input().generics.split_for_impl();
        let turbofish = type_generics.as_turbofish();
        let struct_name = &self.deserialized_struct.serde.name;
        let fields = self.fields();
        let slots = fields.iter().map(slot_of);
        let optional_slots = fields.iter().filter(|field| field.is_optional).map(slot_of);
        quote! {
            let (#(#slots,)*) = #serde::Deserializer::deserialize_struct(
                #deserializer,
                #struct_name,
                __FIELDS,
                __Visitor #turbofish { marker: ::core::marker::PhantomData },
            )?;
            #(let #optional_slots = ::core::option::Option::flatten(#optional_slots);)*
        }
    }

    fn field_checks(&self) -> Vec<TokenStream> {
        let validated_struct = &self.deserialized_struct.validated;
        self.fields()
            .iter()
            .enumerate()
            .map(|(index, field)| {
                let keeps_validity = self.rules.keeps_validity_of(validated_struct, index);
                if field.is_optional {
                    let slot = slot_of(field);
                    let value_of_field = quote!(&#slot);
                    if keeps_validity {
                        return self.rules.field_validity(field, &value_of_field);
                    }
                    return self.rules.field_checks(field, &value_of_field);
                }
                if keeps_validity {
                    return self.present_field_validity(field);
                }
                self.present_field_checks(field)
            })
            .collect()
    }

    fn present_field_checks(&self, field: &ValidatedField) -> TokenStream {
        let slot = slot_of(field);
        let missing = self.missing(field);
        if field.rules.is_empty() {
            return quote! {
                if ::core::option::Option::is_none(&#slot) {
                    #missing
                }
            };
        }
        let value = &self.rules.value;
        let checks = self.rules.field_rule_checks(field);
        quote! {
            match &#slot {
                ::core::option::Option::Some(#value) => {
                    #checks
                }
                ::core::option::Option::None => {
                    #missing
                }
            }
        }
    }

    fn present_field_validity(&self, field: &ValidatedField) -> TokenStream {
        let slot = slot_of(field);
        let is_valid = validity_of(field);
        let missing = self.missing(field);
        let value = &self.rules.value;
        let (present, passes_every_rule) = if field.rules.is_empty() {
            (quote!(_), quote!(true))
        } else {
            (quote!(#value), self.rules.field_passes_every_rule(field))
        };
        quote! {
            let #is_valid = match &#slot {
                ::core::option::Option::Some(#present) => #passes_every_rule,
                ::core::option::Option::None => {
                    #missing
                    false
                }
            };
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
        let fields = self.fields();
        let present_slots: Vec<_> = fields.iter().filter(|field| !field.is_optional).map(slot_of).collect();
        let members = fields.iter().map(|field| field.member);
        let slots = fields.iter().map(slot_of);
        let present_values = (!present_slots.is_empty()).then(|| {
            quote! {
                let (#(::core::option::Option::Some(#present_slots),)*) = (#(#present_slots,)*) else {
                    return ::core::result::Result::Ok(::core::option::Option::None);
                };
            }
        });
        quote! {
            #present_values
            let #deserialized = Self { #(#members: #slots,)* };
        }
    }
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
