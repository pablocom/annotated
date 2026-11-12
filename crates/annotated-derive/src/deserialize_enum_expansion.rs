use proc_macro2::{Literal, Span, TokenStream};
use quote::{ToTokens, format_ident, quote};
use syn::{Generics, Ident, Lifetime, parse_quote};

use crate::deserialize_expansion::{DeserializeExpansion, with_lifetime, without_lifetime_parameters};
use crate::deserialized_enum::{DeserializedEnum, DeserializedVariant};
use crate::deserialized_struct::Form;
use crate::expansion::Expansion;
use crate::serde_names::Tag;
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
        let variant_reader = match &self.deserialized_enum.tag {
            Tag::External => self.enum_visitor(),
            Tag::Adjacent { tag, content } => {
                let variant_seed = self.variant_seed();
                let adjacent_visitor = self.adjacent_visitor(tag, content);
                quote!(#variant_seed #adjacent_visitor)
            }
            Tag::Internal { .. } | Tag::Untagged => self.variant_seed(),
        };
        let reading = self.reading();
        quote! {
            impl #impl_generics ::annotated::Deserialize<'de> for #name #type_generics #where_clause {
                fn deserialize_at<__D: #serde::Deserializer<'de>>(
                    #deserializer: __D,
                    #at: &::annotated::Location<'_>,
                    #found: &mut ::annotated::Report,
                ) -> ::core::result::Result<::core::option::Option<Self>, __D::Error> {
                    #variant_identifier
                    #variant_reader
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
            #[derive(Clone, Copy)]
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
        let expecting = self.expecting();
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

    fn expecting(&self) -> String {
        let name = &self.deserialized_enum.input.ident;
        let representation = match &self.deserialized_enum.tag {
            Tag::External => "",
            Tag::Internal { .. } => "internally tagged ",
            Tag::Adjacent { .. } => "adjacently tagged ",
            Tag::Untagged => "untagged ",
        };
        self.deserialized_enum
            .expecting
            .clone()
            .unwrap_or_else(|| format!("{representation}enum {name}"))
    }

    // An externally tagged variant is read through the `VariantAccess` serde's `EnumAccess` gives for it.
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
        let new_visitor = content.new_visitor();
        let reading = match deserialized_content.form {
            Form::Struct if content.has_flattened_fields() => {
                quote!(#serde::de::VariantAccess::newtype_variant_seed(#variant_access, #new_visitor))
            }
            Form::Struct => {
                quote!(#serde::de::VariantAccess::struct_variant(#variant_access, __FIELDS, #new_visitor))
            }
            Form::Tuple => {
                let length = content.read_fields().count();
                quote!(#serde::de::VariantAccess::tuple_variant(#variant_access, #length, #new_visitor))
            }
            Form::Newtype if content.read_fields().next().is_none() => quote! {
                #serde::de::VariantAccess::unit_variant(#variant_access)?;
                ::core::result::Result::Ok(#new_visitor.checked_value((), ()))
            },
            Form::Newtype => {
                quote!(#serde::de::VariantAccess::newtype_variant_seed(#variant_access, #new_visitor))
            }
        };
        let content_reading = self.content_reading(variant, content, deserialized_content.form, &reading);
        quote! {
            (__Variant::#tag, #variant_access) => {
                #content_reading
            }
        }
    }

    // The items that read a variant's content, the location and report they're given, and the reading itself.
    fn content_reading(
        &self,
        variant: &DeserializedVariant,
        content: &DeserializeExpansion,
        form: Form,
        reading: &TokenStream,
    ) -> TokenStream {
        let Expansion { at, found, .. } = &self.rules;
        let content_at = match &variant.content_place {
            Place::Named(name) => quote!(&self.#at.field(#name)),
            Place::Indexed(index) => quote!(&self.#at.index(#index)),
            Place::OfItsParent => quote!(self.#at),
        };
        let reads_fields_by_name = form == Form::Struct && !content.has_flattened_fields();
        let field_identifier = reads_fields_by_name.then(|| content.field_identifier());
        let functions_reading_fields = content.functions_reading_fields();
        let visitor = content.visitor();
        quote! {
            #field_identifier
            #(#functions_reading_fields)*
            #visitor
            let #at = #content_at;
            let #found = self.#found;
            #reading
        }
    }

    // A variant named by a tag is read from a deserializer of its content: one buffered for an internally tagged enum.
    fn variant_seed(&self) -> TokenStream {
        let Self {
            serde,
            deserializer,
            variant,
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
        let variant_arms = self
            .readable_variants()
            .map(|(index, variant)| self.variant_read_from_its_content(index, variant));
        quote! {
            struct __VariantSeed #visitor_definition_generics #visitor_where_clause {
                #variant: __Variant,
                #at: &#visitor_lifetime ::annotated::Location<#visitor_lifetime>,
                #found: &#visitor_lifetime mut ::annotated::Report,
                marker: ::core::marker::PhantomData<fn() -> #name #type_generics>,
            }

            impl #impl_generics #serde::de::DeserializeSeed<'de> for __VariantSeed #visitor_type_generics #where_clause {
                type Value = ::core::option::Option<#name #type_generics>;

                fn deserialize<__D: #serde::Deserializer<'de>>(
                    self,
                    #deserializer: __D,
                ) -> ::core::result::Result<Self::Value, __D::Error> {
                    match self.#variant {
                        #(#variant_arms)*
                    }
                }
            }
        }
    }

    fn variant_read_from_its_content(&self, index: usize, variant: &DeserializedVariant) -> TokenStream {
        let Self {
            serde, deserializer, ..
        } = self;
        let tag = format_ident!("Variant{index}");
        let name = &self.deserialized_enum.input.ident;
        let ident = variant.ident;
        let skipping_the_content = if let Tag::Internal { .. } = &self.deserialized_enum.tag {
            quote!(#serde::Deserializer::deserialize_ignored_any(#deserializer, #serde::de::IgnoredAny)?;)
        } else {
            let enum_name = name.to_string();
            let variant_name = ident.to_string();
            quote! {
                #serde::Deserializer::deserialize_any(
                    #deserializer,
                    ::annotated::__private::UnitVariantVisitor::new(#enum_name, #variant_name),
                )?;
            }
        };
        let (Some(content), Some(deserialized_content)) = (&self.contents[index], &variant.content) else {
            return quote! {
                __Variant::#tag => {
                    #skipping_the_content
                    ::core::result::Result::Ok(::core::option::Option::Some(#name::#ident))
                }
            };
        };
        let new_visitor = content.new_visitor();
        let reading = match deserialized_content.form {
            Form::Struct => quote!(#serde::Deserializer::deserialize_any(#deserializer, #new_visitor)),
            Form::Tuple => {
                let length = content.read_fields().count();
                quote!(#serde::Deserializer::deserialize_tuple(#deserializer, #length, #new_visitor))
            }
            Form::Newtype if content.read_fields().next().is_none() => quote! {
                #skipping_the_content
                ::core::result::Result::Ok(#new_visitor.checked_value((), ()))
            },
            Form::Newtype => quote!(#serde::de::DeserializeSeed::deserialize(#new_visitor, #deserializer)),
        };
        let content_reading = self.content_reading(variant, content, deserialized_content.form, &reading);
        quote! {
            __Variant::#tag => {
                #content_reading
            }
        }
    }

    fn reading(&self) -> TokenStream {
        let Self {
            read,
            deserialized,
            enum_checks_label,
            ..
        } = self;
        let found = &self.rules.found;
        let enum_checks = self.rules.whole_value_checks(
            &self.deserialized_enum.enum_checks,
            &quote!(#deserialized),
            &quote!(Self),
        );
        let collects_type_errors = self.deserialized_enum.collects_type_errors;
        let read_value = match &self.deserialized_enum.tag {
            Tag::Internal { tag } => self.internally_tagged_value_read(tag),
            _ if collects_type_errors => self.value_read_collecting_type_errors(),
            _ if enum_checks.is_empty() => return self.enum_read(&quote!(#found)),
            _ => {
                let enum_read = self.enum_read(&quote!(&mut *#found));
                quote!(let #read = #enum_read?;)
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

    fn turbofish(&self) -> TokenStream {
        let without_lifetimes = without_lifetime_parameters(&self.deserialized_enum.input.generics);
        let (_, type_generics_without_lifetimes, _) = without_lifetimes.split_for_impl();
        type_generics_without_lifetimes.as_turbofish().to_token_stream()
    }

    fn enum_visitor_reporting_into(&self, found_by_the_visitor: &TokenStream) -> TokenStream {
        let Expansion { at, found, .. } = &self.rules;
        let turbofish = self.turbofish();
        let visitor = if let Tag::Adjacent { .. } = &self.deserialized_enum.tag {
            quote!(__AdjacentVisitor)
        } else {
            quote!(__EnumVisitor)
        };
        quote! {
            #visitor #turbofish {
                #at,
                #found: #found_by_the_visitor,
                marker: ::core::marker::PhantomData,
            }
        }
    }

    fn enum_read(&self, found_by_the_visitor: &TokenStream) -> TokenStream {
        let Self {
            serde, deserializer, ..
        } = self;
        let enum_name = &self.deserialized_enum.name;
        let enum_visitor = self.enum_visitor_reporting_into(found_by_the_visitor);
        if let Tag::Adjacent { .. } = &self.deserialized_enum.tag {
            return quote! {
                #serde::Deserializer::deserialize_struct(#deserializer, #enum_name, __ADJACENT_FIELDS, #enum_visitor)
            };
        }
        quote!(#serde::Deserializer::deserialize_enum(#deserializer, #enum_name, __VARIANTS, #enum_visitor))
    }

    // The enum is buffered whole first, since a streaming parser can't skip a variant it doesn't know.
    fn value_read_collecting_type_errors(&self) -> TokenStream {
        let Self {
            deserializer,
            read,
            read_issues,
            ..
        } = self;
        let Expansion { at, found, .. } = &self.rules;
        let enum_name = &self.deserialized_enum.name;
        let type_name = self.deserialized_enum.input.ident.to_string();
        let enum_visitor = self.enum_visitor_reporting_into(&quote!(&mut #read_issues));
        let (reading_collecting_type_errors, names) = if let Tag::Adjacent { .. } = &self.deserialized_enum.tag {
            (
                quote!(deserialize_adjacently_tagged_collecting_type_errors),
                quote!(__ADJACENT_FIELDS),
            )
        } else {
            (quote!(deserialize_enum_collecting_type_errors), quote!(__VARIANTS))
        };
        quote! {
            let mut #read_issues = #found.new_alike();
            let #read = match ::annotated::__private::#reading_collecting_type_errors(
                #deserializer,
                #enum_name,
                #names,
                #enum_visitor,
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
    }

    fn internally_tagged_value_read(&self, tag: &str) -> TokenStream {
        let Self {
            serde,
            deserializer,
            read,
            variant,
            ..
        } = self;
        let Expansion { at, found, .. } = &self.rules;
        let expecting = self.expecting();
        let collects_type_errors = self.deserialized_enum.collects_type_errors;
        let turbofish = self.turbofish();
        quote! {
            let #read = match ::annotated::__private::read_internally_tagged::<_, __Variant>(
                #deserializer,
                #tag,
                __VARIANTS,
                #expecting,
                #collects_type_errors,
            )? {
                ::annotated::__private::Tagged::Variant(#variant, content) => #serde::de::DeserializeSeed::deserialize(
                    __VariantSeed #turbofish {
                        #variant,
                        #at,
                        #found: &mut *#found,
                        marker: ::core::marker::PhantomData,
                    },
                    ::annotated::__private::Buffered::deserializer::<__D::Error>(content),
                )?,
                ::annotated::__private::Tagged::IssueAtTheTag(problem) => {
                    ::annotated::__private::record_issue(#found, &#at.field(#tag), problem, ::core::option::Option::None);
                    ::core::option::Option::None
                }
                ::annotated::__private::Tagged::IssueAtTheEnum(problem) => {
                    ::annotated::__private::record_issue(#found, #at, problem, ::core::option::Option::None);
                    ::core::option::Option::None
                }
            };
        }
    }
}

// As with serde, an adjacently tagged enum is read as a struct of its tag and its content. Content after the tag is
// read straight from the input, and content before it is buffered until the tag says which variant it is.
impl EnumExpansion<'_> {
    fn adjacent_visitor(&self, tag: &str, content: &str) -> TokenStream {
        let Self {
            serde,
            formatter,
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
        let expecting = self.expecting();
        let visit_map = self.adjacent_visit_map(tag, content);
        let visit_seq = self.adjacent_visit_seq();
        let finished = self.adjacent_finished(tag);
        let buffered_content_read = self.buffered_content_read(content);
        let missing_content = self.missing_content(content);
        quote! {
            const __ADJACENT_FIELDS: &[&str] = &[#tag, #content];

            struct __AdjacentVisitor #visitor_definition_generics #visitor_where_clause {
                #at: &#visitor_lifetime ::annotated::Location<#visitor_lifetime>,
                #found: &#visitor_lifetime mut ::annotated::Report,
                marker: ::core::marker::PhantomData<fn() -> #name #type_generics>,
            }

            impl #impl_generics #serde::de::Visitor<'de> for __AdjacentVisitor #visitor_type_generics #where_clause {
                type Value = ::core::option::Option<#name #type_generics>;

                fn expecting(&self, #formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    ::core::fmt::Formatter::write_str(#formatter, #expecting)
                }

                #visit_map

                #visit_seq
            }

            impl #impl_generics __AdjacentVisitor #visitor_type_generics #where_clause {
                #finished

                #buffered_content_read

                #missing_content
            }
        }
    }

    fn variant_seed_for(&self, variant: &TokenStream, found_by_the_seed: &TokenStream) -> TokenStream {
        let Expansion { at, found, .. } = &self.rules;
        let variant_field = &self.variant;
        let turbofish = self.turbofish();
        quote! {
            __VariantSeed #turbofish {
                #variant_field: #variant,
                #at: self.#at,
                #found: #found_by_the_seed,
                marker: ::core::marker::PhantomData,
            }
        }
    }

    fn adjacent_visit_map(&self, tag: &str, content: &str) -> TokenStream {
        let Self { serde, variant, .. } = self;
        let found = &self.rules.found;
        let local = |name| Ident::new(name, Span::mixed_site());
        let (map, key, tag_read, content_buffered, variant_read) = (
            local("map"),
            local("key"),
            local("tag_read"),
            local("content_buffered"),
            local("variant_read"),
        );
        let is_denying_unknown_fields = self.deserialized_enum.is_denying_unknown_fields;
        let content_read_after_its_tag = (!self.deserialized_enum.collects_type_errors).then(|| {
            let variant_seed = self.variant_seed_for(&quote!(#variant), &quote!(&mut *self.#found));
            quote! {
                ::core::option::Option::Some(::core::option::Option::Some(#variant)) => {
                    #variant_read = ::core::option::Option::Some(
                        #serde::de::MapAccess::next_value_seed(&mut #map, #variant_seed)?,
                    );
                }
            }
        });
        quote! {
            fn visit_map<__A: #serde::de::MapAccess<'de>>(
                self,
                mut #map: __A,
            ) -> ::core::result::Result<Self::Value, __A::Error> {
                let mut #tag_read = ::core::option::Option::None;
                let mut #content_buffered = ::core::option::Option::None;
                let mut #variant_read = ::core::option::Option::None;
                while let ::core::option::Option::Some(#key) = #serde::de::MapAccess::next_key_seed(
                    &mut #map,
                    ::annotated::__private::TagOrContentKey::new(
                        #tag,
                        #content,
                        __ADJACENT_FIELDS,
                        #is_denying_unknown_fields,
                    ),
                )? {
                    match #key {
                        ::annotated::__private::TagOrContent::Tag => {
                            if ::core::option::Option::is_some(&#tag_read) {
                                return ::core::result::Result::Err(
                                    <__A::Error as #serde::de::Error>::duplicate_field(#tag),
                                );
                            }
                            #tag_read = ::core::option::Option::Some(#serde::de::MapAccess::next_value_seed(
                                &mut #map,
                                ::annotated::__private::TagValue::<__Variant>::new(),
                            )?);
                        }
                        ::annotated::__private::TagOrContent::Content => {
                            if ::core::option::Option::is_some(&#content_buffered)
                                || ::core::option::Option::is_some(&#variant_read)
                            {
                                return ::core::result::Result::Err(
                                    <__A::Error as #serde::de::Error>::duplicate_field(#content),
                                );
                            }
                            match #tag_read {
                                #content_read_after_its_tag
                                ::core::option::Option::Some(::core::option::Option::None) => {
                                    #serde::de::MapAccess::next_value::<#serde::de::IgnoredAny>(&mut #map)?;
                                    #variant_read = ::core::option::Option::Some(::core::option::Option::None);
                                }
                                _ => {
                                    #content_buffered = ::core::option::Option::Some(
                                        #serde::de::MapAccess::next_value::<::annotated::__private::Buffered<'de>>(
                                            &mut #map,
                                        )?,
                                    );
                                }
                            }
                        }
                        ::annotated::__private::TagOrContent::Other => {
                            #serde::de::MapAccess::next_value::<#serde::de::IgnoredAny>(&mut #map)?;
                        }
                    }
                }
                self.finished(#tag_read, #content_buffered, #variant_read)
            }
        }
    }

    fn adjacent_visit_seq(&self) -> TokenStream {
        let Self { serde, variant, .. } = self;
        let found = &self.rules.found;
        let seq = Ident::new("seq", Span::mixed_site());
        let content_read = if self.deserialized_enum.collects_type_errors {
            quote! {
                match #serde::de::SeqAccess::next_element::<::annotated::__private::Buffered<'de>>(&mut #seq)? {
                    ::core::option::Option::Some(content) => self.buffered_content_read(#variant, content),
                    ::core::option::Option::None => self.missing_content(#variant),
                }
            }
        } else {
            let variant_seed = self.variant_seed_for(&quote!(#variant), &quote!(&mut *self.#found));
            quote! {
                match #serde::de::SeqAccess::next_element_seed(&mut #seq, #variant_seed)? {
                    ::core::option::Option::Some(read) => ::core::result::Result::Ok(read),
                    ::core::option::Option::None => self.missing_content(#variant),
                }
            }
        };
        quote! {
            fn visit_seq<__A: #serde::de::SeqAccess<'de>>(
                self,
                mut #seq: __A,
            ) -> ::core::result::Result<Self::Value, __A::Error> {
                match #serde::de::SeqAccess::next_element_seed(
                    &mut #seq,
                    ::annotated::__private::TagValue::<__Variant>::new(),
                )? {
                    ::core::option::Option::Some(::core::option::Option::Some(#variant)) => #content_read,
                    ::core::option::Option::Some(::core::option::Option::None) => {
                        #serde::de::SeqAccess::next_element::<#serde::de::IgnoredAny>(&mut #seq)?;
                        self.finished(
                            ::core::option::Option::Some(::core::option::Option::None),
                            ::core::option::Option::None,
                            ::core::option::Option::None,
                        )
                    }
                    ::core::option::Option::None => self.finished(
                        ::core::option::Option::None,
                        ::core::option::Option::None,
                        ::core::option::Option::None,
                    ),
                }
            }
        }
    }

    fn adjacent_finished(&self, tag: &str) -> TokenStream {
        let Self { variant, .. } = self;
        let Expansion { at, found, .. } = &self.rules;
        let input = self.deserialized_enum.input;
        let name = &input.ident;
        let (_, type_generics, _) = input.generics.split_for_impl();
        let reported_at_the_tag = |problem: TokenStream| {
            quote! {
                ::annotated::__private::record_issue(
                    self.#found,
                    &self.#at.field(#tag),
                    #problem,
                    ::core::option::Option::None,
                );
                return ::core::result::Result::Ok(::core::option::Option::None);
            }
        };
        let missing_tag = reported_at_the_tag(quote!(::annotated::Problem::Missing));
        let unknown_tag = reported_at_the_tag(quote!(::annotated::__private::unknown_variant(__VARIANTS)));
        quote! {
            fn finished<__E: ::annotated::__private::serde::de::Error>(
                self,
                tag_read: ::core::option::Option<::core::option::Option<__Variant>>,
                content_buffered: ::core::option::Option<::annotated::__private::Buffered<'de>>,
                variant_read: ::core::option::Option<::core::option::Option<#name #type_generics>>,
            ) -> ::core::result::Result<::core::option::Option<#name #type_generics>, __E> {
                let ::core::option::Option::Some(tag_read) = tag_read else {
                    #missing_tag
                };
                let ::core::option::Option::Some(#variant) = tag_read else {
                    #unknown_tag
                };
                if let ::core::option::Option::Some(variant_read) = variant_read {
                    return ::core::result::Result::Ok(variant_read);
                }
                match content_buffered {
                    ::core::option::Option::Some(content) => self.buffered_content_read(#variant, content),
                    ::core::option::Option::None => self.missing_content(#variant),
                }
            }
        }
    }

    fn buffered_content_read(&self, content: &str) -> TokenStream {
        let Self {
            serde,
            variant,
            read_issues,
            ..
        } = self;
        let Expansion { at, found, .. } = &self.rules;
        let input = self.deserialized_enum.input;
        let name = &input.ident;
        let (_, type_generics, _) = input.generics.split_for_impl();
        let reading = if self.deserialized_enum.collects_type_errors {
            let type_name = name.to_string();
            let variant_seed = self.variant_seed_for(&quote!(#variant), &quote!(&mut #read_issues));
            quote! {
                let mut #read_issues = self.#found.new_alike();
                match ::annotated::__private::Buffered::read_collecting_type_errors(content, #variant_seed, #type_name) {
                    ::core::result::Result::Ok(read) => {
                        self.#found.append(&mut #read_issues);
                        ::core::result::Result::Ok(read)
                    }
                    ::core::result::Result::Err(problem) => {
                        ::annotated::__private::record_issue(
                            self.#found,
                            &self.#at.field(#content),
                            problem,
                            ::core::option::Option::None,
                        );
                        ::core::result::Result::Ok(::core::option::Option::None)
                    }
                }
            }
        } else {
            let variant_seed = self.variant_seed_for(&quote!(#variant), &quote!(self.#found));
            quote! {
                #serde::de::DeserializeSeed::deserialize(
                    #variant_seed,
                    ::annotated::__private::Buffered::deserializer::<__E>(content),
                )
            }
        };
        quote! {
            fn buffered_content_read<__E: ::annotated::__private::serde::de::Error>(
                self,
                #variant: __Variant,
                content: ::annotated::__private::Buffered<'de>,
            ) -> ::core::result::Result<::core::option::Option<#name #type_generics>, __E> {
                #reading
            }
        }
    }

    // As with serde, a unit variant needs no content, and neither does a newtype variant whose value is an `Option`,
    // which is then `None`.
    fn missing_content(&self, content: &str) -> TokenStream {
        let Self { serde, variant, .. } = self;
        let Expansion { at, found, .. } = &self.rules;
        let input = self.deserialized_enum.input;
        let name = &input.ident;
        let (_, type_generics, _) = input.generics.split_for_impl();
        let (variants_needing_none, variants_needing_content): (Vec<_>, Vec<_>) = self
            .readable_variants()
            .map(|(index, deserialized_variant)| (index, deserialized_variant.needs_content()))
            .partition(|(_, needs_content)| !needs_content);
        let tags_needing_none = variants_needing_none
            .iter()
            .map(|(index, _)| format_ident!("Variant{index}"));
        let variant_seed = self.variant_seed_for(&quote!(#variant), &quote!(self.#found));
        let read_without_content = (!variants_needing_none.is_empty()).then(|| {
            quote! {
                #(__Variant::#tags_needing_none)|* => #serde::de::DeserializeSeed::deserialize(
                    #variant_seed,
                    #serde::de::value::UnitDeserializer::<__E>::new(),
                ),
            }
        });
        let missing = (!variants_needing_content.is_empty()).then(|| {
            quote! {
                _ => {
                    ::annotated::__private::record_issue(
                        self.#found,
                        &self.#at.field(#content),
                        ::annotated::Problem::Missing,
                        ::core::option::Option::None,
                    );
                    ::core::result::Result::Ok(::core::option::Option::None)
                }
            }
        });
        quote! {
            fn missing_content<__E: ::annotated::__private::serde::de::Error>(
                self,
                #variant: __Variant,
            ) -> ::core::result::Result<::core::option::Option<#name #type_generics>, __E> {
                match #variant {
                    #read_without_content
                    #missing
                }
            }
        }
    }
}
