use proc_macro2::{Literal, Span, TokenStream};
use quote::{format_ident, quote};
use syn::{Generics, Ident, Lifetime, parse_quote};

use crate::deserialize_expansion::{DeserializeExpansion, with_lifetime, without_lifetime_parameters};
use crate::deserialized_enum::{DeserializedEnum, DeserializedVariant};
use crate::deserialized_struct::Form;
use crate::expansion::Expansion;
use crate::validated_struct::Place;

pub fn deserialize_enum_impl(deserialized_enum: &DeserializedEnum) -> TokenStream {
    EnumExpansion::new(deserialized_enum).deserialize_impl()
}

struct EnumExpansion<'a> {
    deserialized_enum: &'a DeserializedEnum<'a>,
    contents: Vec<Option<DeserializeExpansion<'a>>>,
    rules: Expansion,
    serde: TokenStream,
    deserializer: Ident,
    formatter: Ident,
    data: Ident,
    variant: Ident,
    read: Ident,
    read_issues: Ident,
    deserialized: Ident,
    visitor_lifetime: Lifetime,
    enum_checks_label: Lifetime,
}

impl<'a> EnumExpansion<'a> {
    fn new(deserialized_enum: &'a DeserializedEnum<'a>) -> Self {
        let local = |name| Ident::new(name, Span::mixed_site());
        let contents = deserialized_enum
            .variants
            .iter()
            .map(|variant| variant.content.as_ref().map(DeserializeExpansion::new))
            .collect();
        let enum_checks_label = Lifetime::new("'enum_checks", Span::mixed_site());
        Self {
            deserialized_enum,
            contents,
            rules: Expansion::new(deserialized_enum.is_reporting_all, quote!(break #enum_checks_label)),
            serde: quote!(::annotated::__private::serde),
            deserializer: local("deserializer"),
            formatter: local("formatter"),
            data: local("data"),
            variant: local("variant"),
            read: local("read"),
            read_issues: local("read_issues"),
            deserialized: local("deserialized"),
            visitor_lifetime: Lifetime::new("'__visitor", Span::call_site()),
            enum_checks_label,
        }
    }

    fn deserialize_impl(&self) -> TokenStream {
        let Self {
            serde, deserializer, ..
        } = self;
        let Expansion { at, found, .. } = &self.rules;
        let input = self.deserialized_enum.input;
        let name = &input.ident;
        let generics = self.generics_reading_variants();
        let (impl_generics, _, where_clause) = generics.split_for_impl();
        let (_, type_generics, _) = input.generics.split_for_impl();
        let variant_identifier = self.variant_identifier();
        let enum_visitor = self.enum_visitor();
        let reading = self.reading();
        quote! {
            impl #impl_generics ::annotated::Deserialize<'de> for #name #type_generics #where_clause {
                fn deserialize_at<__D: #serde::Deserializer<'de>>(
                    #deserializer: __D,
                    #at: &::annotated::Location<'_>,
                    #found: &mut ::annotated::Report,
                ) -> ::core::result::Result<::core::option::Option<Self>, __D::Error> {
                    #variant_identifier
                    #enum_visitor
                    #reading
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

    // The bounds of every variant's fields, as each variant's own reading needs them.
    fn generics_reading_variants(&self) -> Generics {
        let mut checking_generics = self.deserialized_enum.input.generics.clone();
        let checking_bounds: Vec<_> = self
            .contents
            .iter()
            .flatten()
            .flat_map(DeserializeExpansion::checking_bounds)
            .collect();
        if !checking_bounds.is_empty() {
            checking_generics.make_where_clause().predicates.extend(checking_bounds);
        }
        let borrowed_lifetimes = &self.deserialized_enum.borrowed_lifetimes;
        let mut generics = checking_generics;
        generics.params.insert(0, parse_quote!('de: #(#borrowed_lifetimes)+*));
        let reading_bounds = self
            .contents
            .iter()
            .flatten()
            .flat_map(DeserializeExpansion::reading_bounds);
        generics.make_where_clause().predicates.extend(reading_bounds);
        generics
    }

    fn readable_variants(&self) -> impl Iterator<Item = (usize, &'a DeserializedVariant<'a>)> {
        self.deserialized_enum.variants.iter().enumerate()
    }

    fn variant_identifier(&self) -> TokenStream {
        let Self { serde, formatter, .. } = self;
        let value = &self.rules.value;
        let indices: Vec<_> = self.readable_variants().map(|(index, _)| index).collect();
        let tags = indices.iter().map(|index| format_ident!("Variant{index}"));
        let index_arms = tags.clone().enumerate().map(|(position, tag)| {
            let position = position as u64;
            quote!(#position => ::core::result::Result::Ok(__Variant::#tag),)
        });
        let names_read: Vec<_> = self
            .readable_variants()
            .map(|(_, variant)| &variant.names_read)
            .collect();
        let name_arms = tags
            .clone()
            .zip(&names_read)
            .map(|(tag, names)| quote!(#(#names)|* => ::core::result::Result::Ok(__Variant::#tag),));
        let byte_arms = tags.clone().zip(&names_read).map(|(tag, names)| {
            let names = names.iter().map(|name| Literal::byte_string(name.as_bytes()));
            quote!(#(#names)|* => ::core::result::Result::Ok(__Variant::#tag),)
        });
        let all_names_read = names_read.iter().copied().flatten();
        let indices_expected = format!("variant index 0 <= i < {}", indices.len());
        quote! {
            enum __Variant {
                #(#tags,)*
            }

            const __VARIANTS: &[&str] = &[#(#all_names_read),*];

            struct __VariantVisitor;

            impl<'de> #serde::de::Visitor<'de> for __VariantVisitor {
                type Value = __Variant;

                fn expecting(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::write_str(#formatter, "variant identifier")
                }

                fn visit_u64<__E: #serde::de::Error>(self, #value: u64) -> ::core::result::Result<__Variant, __E> {
                    match #value {
                        #(#index_arms)*
                        _ => ::core::result::Result::Err(<__E as #serde::de::Error>::invalid_value(
                            #serde::de::Unexpected::Unsigned(#value),
                            &#indices_expected,
                        )),
                    }
                }

                fn visit_str<__E: #serde::de::Error>(self, #value: &str) -> ::core::result::Result<__Variant, __E> {
                    match #value {
                        #(#name_arms)*
                        _ => ::core::result::Result::Err(<__E as #serde::de::Error>::unknown_variant(#value, __VARIANTS)),
                    }
                }

                fn visit_bytes<__E: #serde::de::Error>(self, #value: &[u8]) -> ::core::result::Result<__Variant, __E> {
                    match #value {
                        #(#byte_arms)*
                        _ => {
                            let #value = &::std::string::String::from_utf8_lossy(#value);
                            ::core::result::Result::Err(<__E as #serde::de::Error>::unknown_variant(#value, __VARIANTS))
                        }
                    }
                }
            }

            impl<'de> #serde::Deserialize<'de> for __Variant {
                #[inline]
                fn deserialize<__D: #serde::Deserializer<'de>>(
                    deserializer: __D,
                ) -> ::core::result::Result<Self, __D::Error> {
                    #serde::Deserializer::deserialize_identifier(deserializer, __VariantVisitor)
                }
            }
        }
    }

    fn enum_visitor(&self) -> TokenStream {
        let Self {
            serde,
            formatter,
            data,
            visitor_lifetime,
            ..
        } = self;
        let Expansion { at, found, .. } = &self.rules;
        let input = self.deserialized_enum.input;
        let name = &input.ident;
        let (_, type_generics, _) = input.generics.split_for_impl();
        let visitor_generics = with_lifetime(&input.generics, visitor_lifetime);
        let (visitor_definition_generics, visitor_type_generics, visitor_where_clause) =
            visitor_generics.split_for_impl();
        let generics_reading_variants = with_lifetime(&self.generics_reading_variants(), visitor_lifetime);
        let (impl_generics, _, where_clause) = generics_reading_variants.split_for_impl();
        let expecting = self
            .deserialized_enum
            .expecting
            .clone()
            .unwrap_or_else(|| format!("enum {name}"));
        let variant_arms = self
            .readable_variants()
            .map(|(index, variant)| self.variant_arm(index, variant));
        quote! {
            struct __EnumVisitor #visitor_definition_generics #visitor_where_clause {
                #at: &#visitor_lifetime ::annotated::Location<#visitor_lifetime>,
                #found: &#visitor_lifetime mut ::annotated::Report,
                marker: ::core::marker::PhantomData<fn() -> #name #type_generics>,
            }

            impl #impl_generics #serde::de::Visitor<'de> for __EnumVisitor #visitor_type_generics #where_clause {
                type Value = ::core::option::Option<#name #type_generics>;

                fn expecting(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::write_str(#formatter, #expecting)
                }

                fn visit_enum<__A: #serde::de::EnumAccess<'de>>(
                    self,
                    #data: __A,
                ) -> ::core::result::Result<Self::Value, __A::Error> {
                    match #serde::de::EnumAccess::variant::<__Variant>(#data)? {
                        #(#variant_arms)*
                    }
                }
            }
        }
    }

    fn variant_arm(&self, index: usize, variant: &DeserializedVariant) -> TokenStream {
        let Self { serde, .. } = self;
        let variant_access = &self.variant;
        let tag = format_ident!("Variant{index}");
        let name = &self.deserialized_enum.input.ident;
        let ident = variant.ident;
        let (Some(content), Some(deserialized_content)) = (&self.contents[index], &variant.content) else {
            return quote! {
                (__Variant::#tag, #variant_access) => {
                    #serde::de::VariantAccess::unit_variant(#variant_access)?;
                    ::core::result::Result::Ok(::core::option::Option::Some(#name::#ident))
                }
            };
        };
        let Expansion { at, found, .. } = &self.rules;
        let content_at = match &variant.content_place {
            Place::Named(name) => quote!(&self.#at.field(#name)),
            Place::Indexed(index) => quote!(&self.#at.index(#index)),
            Place::OfItsParent => quote!(self.#at),
        };
        let functions_reading_fields = content.functions_reading_fields();
        let visitor = content.visitor();
        let new_visitor = content.new_visitor();
        let (field_identifier, reading) = match deserialized_content.form {
            Form::Struct if content.has_flattened_fields() => (
                TokenStream::new(),
                quote!(#serde::de::VariantAccess::newtype_variant_seed(#variant_access, #new_visitor)),
            ),
            Form::Struct => (
                content.field_identifier(),
                quote!(#serde::de::VariantAccess::struct_variant(#variant_access, __FIELDS, #new_visitor)),
            ),
            Form::Tuple => {
                let length = content.read_fields().count();
                (
                    TokenStream::new(),
                    quote!(#serde::de::VariantAccess::tuple_variant(#variant_access, #length, #new_visitor)),
                )
            }
            Form::Newtype if content.read_fields().next().is_none() => (
                TokenStream::new(),
                quote! {
                    #serde::de::VariantAccess::unit_variant(#variant_access)?;
                    ::core::result::Result::Ok(#new_visitor.checked_value((), ()))
                },
            ),
            Form::Newtype => (
                TokenStream::new(),
                quote!(#serde::de::VariantAccess::newtype_variant_seed(#variant_access, #new_visitor)),
            ),
        };
        quote! {
            (__Variant::#tag, #variant_access) => {
                #field_identifier
                #(#functions_reading_fields)*
                #visitor
                let #at = #content_at;
                let #found = self.#found;
                #reading
            }
        }
    }

    fn reading(&self) -> TokenStream {
        let Self {
            serde,
            deserializer,
            read,
            read_issues,
            deserialized,
            enum_checks_label,
            ..
        } = self;
        let Expansion { at, found, .. } = &self.rules;
        let input = self.deserialized_enum.input;
        let enum_name = &self.deserialized_enum.name;
        let without_lifetimes = without_lifetime_parameters(&input.generics);
        let (_, type_generics_without_lifetimes, _) = without_lifetimes.split_for_impl();
        let turbofish = type_generics_without_lifetimes.as_turbofish();
        let enum_checks = self.rules.whole_value_checks(
            &self.deserialized_enum.enum_checks,
            &quote!(#deserialized),
            &quote!(Self),
        );
        let new_visitor = |found_by_the_visitor: TokenStream| {
            quote! {
                __EnumVisitor #turbofish {
                    #at,
                    #found: #found_by_the_visitor,
                    marker: ::core::marker::PhantomData,
                }
            }
        };
        let read_value = if self.deserialized_enum.collects_type_errors {
            let type_name = input.ident.to_string();
            let new_visitor = new_visitor(quote!(&mut #read_issues));
            quote! {
                let mut #read_issues = #found.new_alike();
                let #read = match ::annotated::__private::deserialize_enum_collecting_type_errors(
                    #deserializer,
                    #enum_name,
                    __VARIANTS,
                    #new_visitor,
                    #type_name,
                )? {
                    ::core::result::Result::Ok(#read) => {
                        #found.append(&mut #read_issues);
                        #read
                    }
                    ::core::result::Result::Err(problem) => {
                        ::annotated::__private::record_issue(#found, #at, problem, ::core::option::Option::None);
                        ::core::option::Option::None
                    }
                };
            }
        } else if enum_checks.is_empty() {
            let new_visitor = new_visitor(quote!(#found));
            return quote! {
                #serde::Deserializer::deserialize_enum(#deserializer, #enum_name, __VARIANTS, #new_visitor)
            };
        } else {
            let new_visitor = new_visitor(quote!(&mut *#found));
            quote! {
                let #read = #serde::Deserializer::deserialize_enum(#deserializer, #enum_name, __VARIANTS, #new_visitor)?;
            }
        };
        let enum_checks_unless_finished = (!enum_checks.is_empty()).then(|| {
            quote! {
                if let ::core::option::Option::Some(#deserialized) = &#read {
                    if !#found.is_finished() {
                        #enum_checks_label: {
                            #(#enum_checks)*
                        }
                    }
                }
            }
        });
        quote! {
            #read_value
            #enum_checks_unless_finished
            ::core::result::Result::Ok(#read)
        }
    }
}
