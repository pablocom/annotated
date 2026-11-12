use proc_macro2::{Literal, Span, TokenStream, TokenTree};
use quote::{ToTokens, format_ident, quote, quote_spanned};
use syn::ext::IdentExt;
use syn::spanned::Spanned;
use syn::{DeriveInput, GenericParam, Generics, Ident, Lifetime, LifetimeParam, Type, WherePredicate, parse_quote};

use crate::deserialized_struct::{DeserializedStruct, Form};
use crate::expansion::{Expansion, validity_of};
use crate::serde_names::{DefaultValue, SerdeField};
use crate::validated_struct::{FieldRule, Place, ValidatedField, nested_rules_in, option_content};

pub fn deserialize_impl(deserialized_struct: &DeserializedStruct) -> TokenStream {
    DeserializeExpansion::new(deserialized_struct).deserialize_impl()
}

pub struct DeserializeExpansion<'a> {
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
    wrapper: Ident,
    leftovers: Ident,
    element_at: Ident,
    visitor_lifetime: Lifetime,
    field_checks_label: Lifetime,
    struct_checks_label: Lifetime,
}

impl<'a> DeserializeExpansion<'a> {
    pub fn new(deserialized_struct: &'a DeserializedStruct<'a>) -> Self {
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
            wrapper: local("wrapper"),
            leftovers: local("leftovers"),
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

    pub fn read_fields(&self) -> impl Iterator<Item = &'a ValidatedField<'a>> {
        self.fields().iter().filter(|field| !self.is_skipped(field))
    }

    fn read_serde_fields(&self) -> impl Iterator<Item = &'a SerdeField> {
        self.deserialized_struct
            .serde
            .fields
            .iter()
            .filter(|serde_field| !serde_field.is_skipped)
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
        let struct_name = &self.deserialized_struct.serde.name;
        let functions_reading_fields = self.functions_reading_fields();
        let visitor = self.visitor();
        let new_visitor = self.new_visitor();
        let (field_identifier, deserializing) = if self.has_flattened_fields() {
            (
                TokenStream::new(),
                quote!(#serde::Deserializer::deserialize_map(#deserializer, #new_visitor)),
            )
        } else {
            (
                self.field_identifier(),
                quote!(#serde::Deserializer::deserialize_struct(#deserializer, #struct_name, __FIELDS, #new_visitor)),
            )
        };
        quote! {
            impl #impl_generics ::annotated::Deserialize<'de> for #name #type_generics #where_clause {
                fn deserialize_at<__D: #serde::Deserializer<'de>>(
                    #deserializer: __D,
                    #at: &::annotated::Location<'_>,
                    #found: &mut ::annotated::Report,
                ) -> ::core::result::Result<::core::option::Option<Self>, __D::Error> {
                    #field_identifier
                    #(#functions_reading_fields)*
                    #visitor
                    #deserializing
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

    pub fn new_visitor(&self) -> TokenStream {
        let Expansion { at, found, .. } = &self.rules;
        let without_lifetimes = without_lifetime_parameters(&self.input().generics);
        let (_, type_generics_without_lifetimes, _) = without_lifetimes.split_for_impl();
        let turbofish = type_generics_without_lifetimes.as_turbofish();
        quote! {
            __Visitor #turbofish {
                #at,
                #found,
                marker: ::core::marker::PhantomData,
            }
        }
    }

    // As in serde's derive, a function given to `deserialize_with` reads its field through a type whose `Deserialize`
    // calls it, since the function is generic over the deserializer.
    pub fn functions_reading_fields(&self) -> Vec<TokenStream> {
        let Self {
            serde, deserializer, ..
        } = self;
        let input = self.input();
        let name = &input.ident;
        let (_, type_generics, _) = input.generics.split_for_impl();
        let wrapper_generics = self.with_the_de_lifetime(&input.generics);
        let (wrapper_impl_generics, wrapper_type_generics, where_clause) = wrapper_generics.split_for_impl();
        self.read_fields()
            .filter_map(|field| {
                let function = self.serde_field_of(field).deserialize_with.as_ref()?;
                let wrapper = self.wrapper_of(field);
                let field_type = field.field_type;
                let read_by_the_function = quote_spanned!(function.span()=> #function(#deserializer)?);
                Some(quote! {
                    struct #wrapper #wrapper_impl_generics #where_clause {
                        value: #field_type,
                        marker: ::core::marker::PhantomData<fn() -> (#name #type_generics, &'de ())>,
                    }

                    impl #wrapper_impl_generics #serde::Deserialize<'de> for #wrapper #wrapper_type_generics
                    #where_clause
                    {
                        fn deserialize<__D: #serde::Deserializer<'de>>(
                            #deserializer: __D,
                        ) -> ::core::result::Result<Self, __D::Error> {
                            ::core::result::Result::Ok(Self {
                                value: #read_by_the_function,
                                marker: ::core::marker::PhantomData,
                            })
                        }
                    }
                })
            })
            .collect()
    }

    fn wrapper_of(&self, field: &ValidatedField) -> Ident {
        format_ident!("__DeserializeWith{}", self.position_of(field))
    }

    fn wrapper_type_of(&self, field: &ValidatedField) -> TokenStream {
        let wrapper = self.wrapper_of(field);
        let wrapper_generics = self.with_the_de_lifetime(&self.input().generics);
        let (_, wrapper_type_generics, _) = wrapper_generics.split_for_impl();
        quote!(#wrapper #wrapper_type_generics)
    }

    pub fn with_the_de_lifetime(&self, generics: &Generics) -> Generics {
        let borrowed_lifetimes = &self.deserialized_struct.serde.borrowed_lifetimes;
        let mut generics = generics.clone();
        generics.params.insert(0, parse_quote!('de: #(#borrowed_lifetimes)+*));
        generics
    }

    // A variant read by a function of its own reads all of its fields, so each of them is read by a function too.
    fn is_read_by_a_function(&self, field: &ValidatedField) -> bool {
        self.serde_field_of(field).deserialize_with.is_some() || self.deserialized_struct.read_whole_by.is_some()
    }

    fn generics_reading_fields(&self) -> Generics {
        let mut generics = self.with_the_de_lifetime(&self.generics_checking_fields());
        generics.make_where_clause().predicates.extend(self.reading_bounds());
        generics
    }

    pub fn reading_bounds(&self) -> Vec<WherePredicate> {
        let parameters = type_and_const_parameters(&self.input().generics);
        self.fields()
            .iter()
            .filter_map(|field| self.reading_bound_of(field, &parameters))
            .collect()
    }

    fn generics_checking_fields(&self) -> Generics {
        let checking_bounds = self.checking_bounds();
        let mut generics = self.input().generics.clone();
        if !checking_bounds.is_empty() {
            generics.make_where_clause().predicates.extend(checking_bounds);
        }
        generics
    }

    pub fn checking_bounds(&self) -> Vec<WherePredicate> {
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
        checking_bounds
    }

    fn reading_bound_of(&self, field: &ValidatedField, parameters: &[&Ident]) -> Option<WherePredicate> {
        let serde = &self.serde;
        let field_type = field.field_type;
        let is_read_by_serde = !self.is_skipped(field) && !self.is_read_by_a_function(field);
        if !is_read_by_serde || !mentions_any(field_type.to_token_stream(), parameters) {
            return None;
        }
        if field.has_nested_values() {
            return Some(parse_quote!(#field_type: ::annotated::Deserialize<'de>));
        }
        Some(parse_quote!(#field_type: #serde::Deserialize<'de>))
    }

    fn checking_bounds_of(&self, field: &ValidatedField, parameters: &[&Ident]) -> Vec<WherePredicate> {
        let (takes_a_default, is_its_type_s_default) = match self.when_absent(field) {
            WhenAbsent::TakesDefault(default) => (true, matches!(default, DefaultValue::OfItsType)),
            WhenAbsent::TakesStructDefault => (true, false),
            WhenAbsent::IsMissing | WhenAbsent::IsNone | WhenAbsent::IsNeverAbsent => (false, false),
        };
        if !takes_a_default && self.reads_nested_values(field) {
            return Vec::new();
        }
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

    pub fn field_identifier(&self) -> TokenStream {
        let Self { serde, formatter, .. } = self;
        let value = &self.rules.value;
        let serde_fields: Vec<_> = self.read_serde_fields().collect();
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

    pub fn visitor(&self) -> TokenStream {
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
        let has_flattened_fields = self.has_flattened_fields();
        let form = self.deserialized_struct.form;
        let is_read_whole_by_a_function = self.deserialized_struct.read_whole_by.is_some();
        let (visit_seq, visit_map) = match form {
            _ if is_read_whole_by_a_function => (TokenStream::new(), TokenStream::new()),
            Form::Struct if has_flattened_fields => (TokenStream::new(), self.visit_map_keeping_leftovers()),
            Form::Struct => (self.visit_seq(), self.visit_map()),
            Form::Tuple => (self.visit_seq(), TokenStream::new()),
            Form::Newtype => (TokenStream::new(), TokenStream::new()),
        };
        let checked_value = self.checked_value();
        let names_read = self.read_serde_fields().flat_map(|field| &field.names_read);
        let has_field_names = form == Form::Struct && !has_flattened_fields && !is_read_whole_by_a_function;
        let field_names = has_field_names.then(|| quote!(const __FIELDS: &[&str] = &[#(#names_read),*];));
        let reading = match form {
            _ if is_read_whole_by_a_function => TokenStream::new(),
            Form::Newtype => self.newtype_seed(),
            Form::Struct if has_flattened_fields && self.deserialized_struct.variant.is_some() => {
                self.seed_reading_a_map()
            }
            Form::Struct | Form::Tuple => TokenStream::new(),
        };
        quote! {
            #field_names

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

            #reading

            impl #checking_impl_generics __Visitor #visitor_type_generics #checking_where_clause {
                #checked_value
            }
        }
    }

    fn visitor_impl_generics(&self) -> Generics {
        with_lifetime(&self.generics_reading_fields(), &self.visitor_lifetime)
    }

    // As in serde's derive, a struct variant with a flattened field is read as a map, through a seed.
    fn seed_reading_a_map(&self) -> TokenStream {
        let Self {
            serde, deserializer, ..
        } = self;
        let input = self.input();
        let name = &input.ident;
        let (_, type_generics, _) = input.generics.split_for_impl();
        let visitor_generics = with_lifetime(&input.generics, &self.visitor_lifetime);
        let (_, visitor_type_generics, _) = visitor_generics.split_for_impl();
        let generics_reading_fields = self.visitor_impl_generics();
        let (impl_generics, _, where_clause) = generics_reading_fields.split_for_impl();
        quote! {
            impl #impl_generics #serde::de::DeserializeSeed<'de> for __Visitor #visitor_type_generics #where_clause {
                type Value = ::core::option::Option<#name #type_generics>;

                fn deserialize<__D: #serde::Deserializer<'de>>(
                    self,
                    #deserializer: __D,
                ) -> ::core::result::Result<Self::Value, __D::Error> {
                    #serde::Deserializer::deserialize_map(#deserializer, self)
                }
            }
        }
    }

    // A newtype variant's value is read whole, through a seed, at the variant's location.
    fn newtype_seed(&self) -> TokenStream {
        let Self {
            serde, deserializer, ..
        } = self;
        let Expansion { found, .. } = &self.rules;
        let input = self.input();
        let name = &input.ident;
        let (_, type_generics, _) = input.generics.split_for_impl();
        let visitor_generics = with_lifetime(&input.generics, &self.visitor_lifetime);
        let (_, visitor_type_generics, _) = visitor_generics.split_for_impl();
        let generics_reading_fields = self.visitor_impl_generics();
        let (impl_generics, _, where_clause) = generics_reading_fields.split_for_impl();
        let Some(field) = self.read_fields().next() else {
            return TokenStream::new();
        };
        let slot = slot_of(field);
        let slot_type = self.slot_type_of(field);
        let read_value = self.value_read_whole(field);
        let read_issues: Vec<_> = self.fields_read_with_issues().map(read_issues_of).collect();
        quote! {
            impl #impl_generics #serde::de::DeserializeSeed<'de> for __Visitor #visitor_type_generics #where_clause {
                type Value = ::core::option::Option<#name #type_generics>;

                fn deserialize<__D: #serde::Deserializer<'de>>(
                    self,
                    #deserializer: __D,
                ) -> ::core::result::Result<Self::Value, __D::Error> {
                    #(let mut #read_issues = self.#found.new_alike();)*
                    let #slot: #slot_type = #read_value;
                    ::core::result::Result::Ok(self.checked_value((#slot,), (#(#read_issues,)*)))
                }
            }
        }
    }

    fn value_read_whole(&self, field: &ValidatedField) -> TokenStream {
        let Self {
            serde,
            deserializer,
            wrapper,
            ..
        } = self;
        let field_type = field.field_type;
        let is_read_by_a_function = self.is_read_by_a_function(field);
        if !self.is_read_with_issues(field) && is_read_by_a_function {
            let wrapper_type = self.wrapper_type_of(field);
            return quote!(<#wrapper_type as #serde::Deserialize<'de>>::deserialize(#deserializer)?.value);
        }
        if !self.is_read_with_issues(field) {
            return quote_spanned! {field_type.span()=>
                <#field_type as #serde::Deserialize<'de>>::deserialize(#deserializer)?
            };
        }
        let reading_with_its_issues = self.reading_with_its_issues(field);
        let value = quote_spanned! {field_type.span()=>
            ::annotated::__private::serde::de::DeserializeSeed::deserialize(#reading_with_its_issues, #deserializer)?
        };
        if !is_read_by_a_function {
            return value;
        }
        quote!(::core::option::Option::map(#value, |#wrapper| #wrapper.value))
    }

    fn visit_seq(&self) -> TokenStream {
        let Self { serde, seq, .. } = self;
        let Expansion { found, .. } = &self.rules;
        let elements = self.read_fields().map(|field| {
            let slot = slot_of(field);
            let slot_type = self.slot_type_of(field);
            let field_type = field.field_type;
            let wrapper = &self.wrapper;
            if !self.is_read_with_issues(field) && self.is_read_by_a_function(field) {
                let wrapper_type = self.wrapper_type_of(field);
                return quote! {
                    let #slot: #slot_type = ::core::option::Option::map(
                        #serde::de::SeqAccess::next_element::<#wrapper_type>(&mut #seq)?,
                        |#wrapper| #wrapper.value,
                    );
                };
            }
            if !self.is_read_with_issues(field) {
                return quote! {
                    let #slot: #slot_type = #serde::de::SeqAccess::next_element::<#field_type>(&mut #seq)?;
                };
            }
            let read_issues = read_issues_of(field);
            let reading_with_its_issues = self.reading_with_its_issues(field);
            let mut element = quote_spanned! {field_type.span()=>
                ::annotated::__private::serde::de::SeqAccess::next_element_seed(
                    &mut #seq,
                    #reading_with_its_issues,
                )?
            };
            if self.is_read_by_a_function(field) {
                let read = &self.read;
                element = quote! {
                    ::core::option::Option::map(#element, |#read| ::core::option::Option::map(#read, |#wrapper| #wrapper.value))
                };
            }
            quote! {
                let mut #read_issues = self.#found.new_alike();
                let #slot: #slot_type = #element;
            }
        });
        let slots = self.read_fields().map(slot_of);
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
        let read_issues = read_issues_of(field);
        let at_the_field = match &field.place {
            Place::Named(name) => quote!(&self.#at.field(#name)),
            Place::Indexed(index) => quote!(&self.#at.index(#index)),
            Place::OfItsParent => quote!(self.#at),
        };
        if !self.deserialized_struct.collects_type_errors {
            return quote_spanned! {field_type.span()=>
                ::annotated::__private::DeserializeAt::<#field_type>::new(#at_the_field, &mut #read_issues)
            };
        }
        let type_name = type_name_of(field_type);
        if self.is_read_by_a_function(field) {
            let wrapper_type = self.wrapper_type_of(field);
            return quote_spanned! {field_type.span()=>
                ::annotated::__private::CollectingTypeErrorsOfAFunctionAt::<#wrapper_type>::new(
                    #type_name,
                    #at_the_field,
                    &mut #read_issues,
                )
            };
        }
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
        let is_read_alone = !self.is_skipped(field) && self.deserialized_struct.read_whole_by.is_none();
        is_read_alone && (self.reads_nested_values(field) || self.deserialized_struct.collects_type_errors)
    }

    fn reads_nested_values(&self, field: &ValidatedField) -> bool {
        field.has_nested_values() && !self.is_skipped(field) && !self.is_read_by_a_function(field)
    }

    fn is_skipped(&self, field: &ValidatedField) -> bool {
        self.serde_field_of(field).is_skipped
    }

    pub fn slots(&self) -> Vec<Ident> {
        self.read_fields().map(slot_of).collect()
    }

    pub fn has_flattened_fields(&self) -> bool {
        self.read_fields().any(|field| field.is_flattened)
    }

    fn keyed_fields(&self) -> impl Iterator<Item = &'a ValidatedField<'a>> {
        self.read_fields().filter(|field| !field.is_flattened)
    }

    fn fields_read_with_issues(&self) -> impl Iterator<Item = &'a ValidatedField<'a>> {
        self.fields().iter().filter(|field| self.is_read_with_issues(field))
    }

    fn visit_map(&self) -> TokenStream {
        let Self { serde, map, key, .. } = self;
        let Expansion { found, .. } = &self.rules;
        let slots = self.keyed_fields().map(slot_of);
        let slot_types = self.keyed_fields().map(|field| self.slot_type_of(field));
        let field_arms = self.keyed_fields().enumerate().map(|(index, field)| {
            let variant = format_ident!("Field{index}");
            let slot = slot_of(field);
            let name = &field.name;
            let value = self.value_read_from_the_map(field);
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
        let read_slots = self.read_fields().map(slot_of);
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

    // As in serde's derive, the keys that aren't the struct's fields are kept, and each flattened field is read from
    // them after the last key.
    fn visit_map_keeping_leftovers(&self) -> TokenStream {
        let Self {
            serde,
            map,
            key,
            leftovers,
            ..
        } = self;
        let Expansion { found, .. } = &self.rules;
        let slots = self.keyed_fields().map(slot_of);
        let slot_types = self.keyed_fields().map(|field| self.slot_type_of(field));
        let field_arms = self.keyed_fields().map(|field| {
            let names_read = &self.serde_field_of(field).names_read;
            let slot = slot_of(field);
            let name = &field.name;
            let value = self.value_read_from_the_map(field);
            quote! {
                ::core::option::Option::Some(#(#names_read)|*) => {
                    if ::core::option::Option::is_some(&#slot) {
                        return ::core::result::Result::Err(
                            <__A::Error as #serde::de::Error>::duplicate_field(#name),
                        );
                    }
                    #slot = ::core::option::Option::Some(#value);
                }
            }
        });
        let flattened_values = self
            .read_fields()
            .filter(|field| field.is_flattened)
            .map(|field| self.flattened_value(field));
        let read_slots = self.read_fields().map(slot_of);
        let read_issues: Vec<_> = self.fields_read_with_issues().map(read_issues_of).collect();
        quote! {
            #[inline]
            fn visit_map<__A: #serde::de::MapAccess<'de>>(
                self,
                mut #map: __A,
            ) -> ::core::result::Result<Self::Value, __A::Error> {
                #(let mut #slots: #slot_types = ::core::option::Option::None;)*
                #(let mut #read_issues = self.#found.new_alike();)*
                let mut #leftovers = ::annotated::__private::Leftovers::new();
                while let ::core::option::Option::Some(#key) =
                    #serde::de::MapAccess::next_key::<::annotated::__private::Key<'de>>(&mut #map)?
                {
                    match ::annotated::__private::Key::as_field_name(&#key) {
                        #(#field_arms)*
                        _ => ::annotated::__private::Leftovers::keep(&mut #leftovers, #key, &mut #map)?,
                    }
                }
                #(#flattened_values)*
                ::core::result::Result::Ok(self.checked_value((#(#read_slots,)*), (#(#read_issues,)*)))
            }
        }
    }

    fn flattened_value(&self, field: &ValidatedField) -> TokenStream {
        let Self {
            serde,
            leftovers,
            wrapper,
            ..
        } = self;
        let Expansion { at, .. } = &self.rules;
        let slot = slot_of(field);
        let slot_type = self.slot_type_of(field);
        let field_type = field.field_type;
        let read_issues = read_issues_of(field);
        let flattened = quote!(::annotated::__private::Leftovers::flattened::<__A::Error>(&mut #leftovers));
        let is_read_by_a_function = self.is_read_by_a_function(field);
        let content_type = option_content(field_type).filter(|_| !is_read_by_a_function);
        let read_type = if is_read_by_a_function {
            self.wrapper_type_of(field)
        } else {
            field_type.to_token_stream()
        };
        let read_value = match (self.reads_nested_values(field), content_type) {
            (true, Some(content_type)) => quote_spanned! {field_type.span()=>
                ::core::option::Option::Some(::core::option::Option::flatten(::core::result::Result::ok(
                    <#content_type as ::annotated::Deserialize<'de>>::deserialize_at(#flattened, self.#at, &mut #read_issues),
                )))
            },
            (true, None) => quote_spanned! {field_type.span()=>
                <#field_type as ::annotated::Deserialize<'de>>::deserialize_at(#flattened, self.#at, &mut #read_issues)?
            },
            (false, Some(content_type)) => {
                let value_or_none = quote_spanned! {field_type.span()=>
                    ::core::result::Result::ok(<#content_type as #serde::Deserialize<'de>>::deserialize(#flattened))
                };
                self.as_its_slot_holds_it(field, &value_or_none)
            }
            (false, None) if self.deserialized_struct.collects_type_errors => {
                let type_name = type_name_of(field_type);
                let read = quote_spanned! {field_type.span()=>
                    ::annotated::__private::Leftovers::flattened_collecting_type_errors::<#read_type>(
                        &mut #leftovers,
                        #type_name,
                        self.#at,
                        &mut #read_issues,
                    )
                };
                if is_read_by_a_function {
                    quote!(::core::option::Option::map(#read, |#wrapper| #wrapper.value))
                } else {
                    read
                }
            }
            (false, None) => {
                let read = quote_spanned! {field_type.span()=>
                    <#read_type as #serde::Deserialize<'de>>::deserialize(#flattened)?
                };
                if is_read_by_a_function {
                    quote!(#read.value)
                } else {
                    read
                }
            }
        };
        quote!(let #slot: #slot_type = #read_value;)
    }

    fn value_read_from_the_map(&self, field: &ValidatedField) -> TokenStream {
        let Self {
            serde, map, wrapper, ..
        } = self;
        let field_type = field.field_type;
        let is_read_by_a_function = self.is_read_by_a_function(field);
        if !self.is_read_with_issues(field) && is_read_by_a_function {
            let wrapper_type = self.wrapper_type_of(field);
            return quote!(#serde::de::MapAccess::next_value::<#wrapper_type>(&mut #map)?.value);
        }
        if !self.is_read_with_issues(field) {
            return quote!(#serde::de::MapAccess::next_value::<#field_type>(&mut #map)?);
        }
        let reading_with_its_issues = self.reading_with_its_issues(field);
        let value = quote_spanned! {field_type.span()=>
            ::annotated::__private::serde::de::MapAccess::next_value_seed(
                &mut #map,
                #reading_with_its_issues,
            )?
        };
        if !is_read_by_a_function {
            return value;
        }
        quote!(::core::option::Option::map(#value, |#wrapper| #wrapper.value))
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
        let slots = self.read_fields().map(slot_of);
        let slot_types = self.read_fields().map(|field| self.slot_type_of(field));
        let read_issues = self.fields_read_with_issues().map(read_issues_of);
        let read_issues_types = self.fields_read_with_issues().map(|_| quote!(::annotated::Report));
        let buffers_of_nested_values_not_read = self
            .fields()
            .iter()
            .filter(|field| field.has_nested_values() && !self.is_read_with_issues(field))
            .map(read_issues_of);
        let nested_values_not_read_validated = self.nested_values_not_read_validated();
        let values_of_slots = self.fields().iter().map(|field| self.value_of_slot(field));
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
                #(let mut #buffers_of_nested_values_not_read = #found.new_alike();)*
                #(#values_of_slots)*
                #values_from_the_struct_default
                #(#nested_values_not_read_validated)*
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
                if self.holds_its_value(field) {
                    let slot = slot_of(field);
                    let value_of_field = quote!(&#slot);
                    let waiting_for_the_elements = waiting_for_the_elements(field);
                    let checks = if keeps_validity {
                        self.validity_of_a_value_held(field, &value_of_field)
                    } else {
                        self.rules_of(field).field_checks(field, &value_of_field)
                    };
                    return quote!(#waiting_for_the_elements #checks);
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

    fn holds_its_value(&self, field: &ValidatedField) -> bool {
        !self.can_be_missing(field) && !self.is_read_with_issues(field)
    }

    fn validity_of_a_value_held(&self, field: &ValidatedField, value_of_field: &TokenStream) -> TokenStream {
        let is_valid = validity_of(field);
        let value = &self.rules.value;
        if field.rules.is_empty() {
            return quote!(#is_valid = true;);
        }
        let passes_every_rule = self.rules_of(field).field_passes_every_rule(field);
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
            let member = &field.member;
            let slot = slot_of(field);
            match (self.is_read_with_issues(field), self.can_be_missing(field)) {
                (false, false) => quote!(#member: #slot),
                (false, true) | (true, false) => quote!(#member: #slot?),
                (true, true) => quote!(#member: ::core::option::Option::flatten(#slot)?),
            }
        });
        let name = &self.input().ident;
        let built = self
            .deserialized_struct
            .variant
            .map_or_else(|| quote!(#name), |variant| quote!(#name::#variant));
        quote! {
            let #deserialized = #built { #(#fields,)* };
        }
    }

    fn slot_type_of(&self, field: &ValidatedField) -> TokenStream {
        let field_type = field.field_type;
        let is_never_absent = matches!(self.when_absent(field), WhenAbsent::IsNeverAbsent);
        if is_never_absent && self.is_read_with_issues(field) {
            return quote!(::core::option::Option<#field_type>);
        }
        if is_never_absent {
            return quote!(#field_type);
        }
        if self.is_read_with_issues(field) {
            return quote!(::core::option::Option<::core::option::Option<#field_type>>);
        }
        quote!(::core::option::Option<#field_type>)
    }

    fn value_of_slot(&self, field: &ValidatedField) -> TokenStream {
        let slot = slot_of(field);
        let is_read_with_issues = self.is_read_with_issues(field);
        match self.when_absent(field) {
            WhenAbsent::IsMissing | WhenAbsent::TakesStructDefault | WhenAbsent::IsNeverAbsent => TokenStream::new(),
            WhenAbsent::IsNone if is_read_with_issues => quote! {
                let #slot = ::core::option::Option::unwrap_or(
                    #slot,
                    ::core::option::Option::Some(::core::option::Option::None),
                );
            },
            WhenAbsent::IsNone => quote!(let #slot = ::core::option::Option::flatten(#slot);),
            WhenAbsent::TakesDefault(default) if self.is_skipped(field) => {
                let value_when_absent = self.value_when_absent(field, &default_value(field, default));
                quote!(let #slot = #value_when_absent;)
            }
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

    // The struct's default is made once, and only when a field that takes its value from it is absent. A skipped
    // field is always absent.
    fn values_from_the_struct_default(&self) -> TokenStream {
        let Some(struct_default) = &self.deserialized_struct.serde.default else {
            return TokenStream::new();
        };
        let (skipped_fields, read_fields): (Vec<_>, Vec<_>) = self
            .fields()
            .iter()
            .filter(|field| matches!(self.when_absent(field), WhenAbsent::TakesStructDefault))
            .partition(|field| self.is_skipped(field));
        if skipped_fields.is_empty() && read_fields.is_empty() {
            return TokenStream::new();
        }
        let read = &self.read;
        let read_slots: Vec<_> = read_fields.iter().map(|field| slot_of(field)).collect();
        let skipped_slots = skipped_fields.iter().map(|field| slot_of(field));
        let members = read_fields.iter().chain(&skipped_fields).map(|field| &field.member);
        let defaults = read_fields
            .iter()
            .chain(&skipped_fields)
            .map(|field| struct_default_of(field));
        let read_values = read_fields.iter().map(|field| {
            let slot = slot_of(field);
            let struct_default = struct_default_of(field);
            let value_when_absent = self.value_when_absent(field, &quote!(#struct_default));
            quote! {
                match #slot {
                    ::core::option::Option::Some(#read) => #read,
                    ::core::option::Option::None => #value_when_absent,
                }
            }
        });
        let skipped_values = skipped_fields.iter().map(|field| {
            let struct_default = struct_default_of(field);
            self.value_when_absent(field, &quote!(#struct_default))
        });
        let input = self.input();
        let name = &input.ident;
        let (_, type_generics, _) = input.generics.split_for_impl();
        let struct_default_value = match struct_default {
            DefaultValue::OfItsType => quote!(<#name #type_generics as ::core::default::Default>::default()),
            DefaultValue::ReturnedBy(function) => quote_spanned!(function.span()=> #function()),
        };
        let values = quote! {
            let #name { #(#members: #defaults,)* .. } = #struct_default_value;
            (#(#read_values,)* #(#skipped_values,)*)
        };
        if !skipped_fields.is_empty() {
            return quote! {
                let (#(#read_slots,)* #(#skipped_slots,)*) = { #values };
            };
        }
        quote! {
            let (#(#read_slots,)*) = match (#(#read_slots,)*) {
                (#(::core::option::Option::Some(#read_slots),)*) => (#(#read_slots,)*),
                (#(#read_slots,)*) => { #values }
            };
        }
    }

    fn when_absent(&self, field: &ValidatedField) -> WhenAbsent<'a> {
        let is_read_with_its_variant = self.deserialized_struct.read_whole_by.is_some();
        if is_read_with_its_variant || (is_its_parent_s_value(field) && !self.is_skipped(field)) {
            return WhenAbsent::IsNeverAbsent;
        }
        if let Some(default) = &self.serde_field_of(field).default {
            return WhenAbsent::TakesDefault(default);
        }
        if self.deserialized_struct.serde.default.is_some() {
            return WhenAbsent::TakesStructDefault;
        }
        if field.is_optional && !self.is_read_by_a_function(field) {
            return WhenAbsent::IsNone;
        }
        WhenAbsent::IsMissing
    }

    fn can_be_missing(&self, field: &ValidatedField) -> bool {
        matches!(self.when_absent(field), WhenAbsent::IsMissing)
    }

    fn serde_field_of(&self, field: &ValidatedField) -> &'a SerdeField {
        &self.deserialized_struct.serde.fields[self.position_of(field)]
    }

    fn position_of(&self, field: &ValidatedField) -> usize {
        self.fields()
            .iter()
            .position(|candidate| candidate.member == field.member)
            .expect("the field is one of the struct's")
    }

    fn value_when_absent(&self, field: &ValidatedField, default_value: &TokenStream) -> TokenStream {
        if !self.reads_nested_values(field) {
            return self.as_its_slot_holds_it(field, default_value);
        }
        let value = &self.rules.value;
        let field_type = field.field_type;
        let validating_nested_values = self.validating_nested_values(field, &quote!(&#value));
        let held_value = self.as_its_slot_holds_it(field, &quote!(#value));
        quote! {
            {
                let #value: #field_type = #default_value;
                #validating_nested_values
                #held_value
            }
        }
    }

    fn as_its_slot_holds_it(&self, field: &ValidatedField, value: &TokenStream) -> TokenStream {
        if self.is_read_with_issues(field) {
            return quote!(::core::option::Option::Some(#value));
        }
        value.clone()
    }

    fn nested_values_not_read_validated(&self) -> Vec<TokenStream> {
        let value = &self.rules.value;
        self.fields()
            .iter()
            .filter(|field| field.has_nested_values() && !self.reads_nested_values(field))
            .map(|field| {
                let slot = slot_of(field);
                if self.holds_its_value(field) {
                    return self.validating_nested_values(field, &quote!(&#slot));
                }
                let built = if self.is_read_with_issues(field) && self.can_be_missing(field) {
                    quote!(::core::option::Option::Some(::core::option::Option::Some(#value)))
                } else {
                    quote!(::core::option::Option::Some(#value))
                };
                let validating_nested_values = self.validating_nested_values(field, &quote!(#value));
                quote! {
                    if let #built = &#slot {
                        #validating_nested_values
                    }
                }
            })
            .collect()
    }

    // Nothing waits in the field's buffer for a value annotated didn't read. Validating its nested values into the
    // buffer lets the field's rules find them there, as for a value read, so the rules are expanded once for both.
    fn validating_nested_values(&self, field: &ValidatedField, value: &TokenStream) -> TokenStream {
        self.validating_nested_values_in(
            &field.rules,
            field.is_optional,
            value,
            &self.rules.field_location(field),
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
    IsNeverAbsent,
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

// A flattened field or a newtype variant's value is read whole from its parent, so it is never absent.
fn is_its_parent_s_value(field: &ValidatedField) -> bool {
    matches!(field.place, Place::OfItsParent)
}

fn struct_default_of(field: &ValidatedField) -> Ident {
    field.local("struct_default")
}

fn slot_of(field: &ValidatedField) -> Ident {
    field.local("slot")
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
    field.local("read_issues")
}

// A turbofish takes all of a type's lifetime arguments or none, and the visitor has one more than the struct.
pub fn without_lifetime_parameters(generics: &Generics) -> Generics {
    let mut generics = generics.clone();
    generics.params = generics
        .params
        .into_iter()
        .filter(|parameter| !matches!(parameter, GenericParam::Lifetime(_)))
        .collect();
    generics
}

pub fn with_lifetime(generics: &Generics, lifetime: &Lifetime) -> Generics {
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
