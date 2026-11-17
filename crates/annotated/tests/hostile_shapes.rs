//! Both derives compile on shapes meant to break a macro, with every warning denied: raw identifiers, names the
//! generated code uses for its own, user types shadowing std's, a module without the prelude, and every struct and
//! enum the derives accept in every serde tagging. A shape the derives rightly refuse is a compile-fail case under
//! `tests/ui/` instead.

#![cfg(feature = "serde")]
#![deny(warnings)]

use annotated::{Deserialize, Validate};

fn derives_both<T: Validate + for<'de> Deserialize<'de>>() {}

#[derive(Debug, Validate, Deserialize)]
struct Address {
    #[validate(required)]
    street: String,
}

mod raw_identifiers {
    use annotated::{Deserialize, Problem, Validate};

    #[derive(Debug, Validate, Deserialize)]
    #[validate(custom(Keywords::is_not_a_loop, field = r#type))]
    pub struct Keywords {
        #[validate(required)]
        pub r#type: String,
        #[validate(length(max = 3))]
        pub r#match: Option<String>,
        #[validate(each(required))]
        pub r#loop: Vec<String>,
    }

    impl Keywords {
        fn is_not_a_loop(&self) -> Result<(), Problem> {
            if self.r#type == "loop" {
                return Err(Problem::custom("a_loop", "is a loop"));
            }
            Ok(())
        }
    }

    #[derive(Debug, Validate, Deserialize)]
    #[serde(tag = "kind")]
    pub enum Keyword {
        r#Struct {
            #[validate(required)]
            r#type: String,
        },
        r#Unit,
    }
}

#[test]
fn raw_identifiers_are_fields_and_variants_like_any_other() {
    derives_both::<raw_identifiers::Keywords>();
    derives_both::<raw_identifiers::Keyword>();
}

// Every local and item the expansions name, as a field, and type parameters named like the derives' own.
mod names_the_derives_use {
    use annotated::{Deserialize, Problem, Validate};

    #[derive(Debug, Validate, Deserialize)]
    #[validate(custom(Locals::has_a_value))]
    pub struct Locals<T, E, D, A> {
        #[validate(required)]
        pub found: String,
        #[validate(required)]
        pub at: String,
        #[validate(required)]
        pub value: String,
        #[validate(each(required))]
        pub map: Vec<String>,
        #[validate(required)]
        pub key: String,
        #[validate(required)]
        pub seq: String,
        #[validate(required)]
        pub deserializer: String,
        #[validate(required)]
        pub formatter: String,
        #[validate(required)]
        pub deserialized: String,
        #[validate(required)]
        pub read: String,
        #[validate(required)]
        pub wrapper: String,
        #[validate(required)]
        pub leftovers: String,
        #[validate(required)]
        pub element_at: String,
        #[validate(required)]
        pub problem: String,
        #[validate(required)]
        pub found_before: String,
        #[validate(required)]
        pub content: String,
        #[validate(required)]
        pub tried: String,
        #[validate(required)]
        pub marker: String,
        #[validate(required)]
        pub checked: String,
        #[validate(required)]
        pub value_rules: String,
        #[validate(required)]
        pub has_finished: String,
        #[validate(required)]
        pub data: String,
        #[validate(required)]
        pub variant: String,
        #[validate(required)]
        pub tag_read: String,
        #[validate(required)]
        pub variant_read: String,
        #[validate(required)]
        pub content_buffered: String,
        #[validate(required)]
        pub buffered: String,
        #[validate(nested)]
        pub read_issues: super::Address,
        #[validate(each(nested))]
        pub is_valid: Vec<super::Address>,
        #[validate(length(max = 3))]
        pub t: Vec<T>,
        #[validate(length(max = 3))]
        pub e: Vec<E>,
        #[validate(length(max = 3))]
        pub d: Vec<D>,
        #[validate(length(max = 3))]
        pub a: Vec<A>,
    }

    impl<T, E, D, A> Locals<T, E, D, A> {
        fn has_a_value(&self) -> Result<(), Problem> {
            if self.value.is_empty() {
                return Err(Problem::custom("no_value", "has no value"));
            }
            Ok(())
        }
    }

    #[derive(Debug, Validate, Deserialize)]
    #[serde(tag = "found", content = "at")]
    pub enum Variants {
        Value {
            #[validate(required)]
            value: String,
            #[validate(required)]
            map: String,
        },
        Key(#[validate(required)] String, #[validate(required)] String),
        Read(#[validate(nested)] super::Address),
    }
}

#[test]
fn fields_and_type_parameters_take_the_names_the_derives_use() {
    derives_both::<names_the_derives_use::Locals<u8, u8, u8, u8>>();
    derives_both::<names_the_derives_use::Variants>();
}

// User items named as std's are in scope where the code is generated.
mod names_shadowing_std_s {
    #![allow(dead_code, clippy::partial_pub_fields)]

    pub struct Option;
    pub struct Result;
    pub struct Some;
    pub struct None;
    pub struct Vec;
    pub struct String;
    pub struct Box;
    pub struct Ok;
    pub struct Err;
    pub trait Default {}
    pub trait Clone {}
    pub trait Iterator {}
    pub trait Into {}
    pub trait From {}
    pub trait Fn {}
    pub trait Sized {}
    pub trait Deserialize {}
    pub trait Validate {}

    #[derive(Debug, annotated::Validate, annotated::Deserialize)]
    #[annotated(collect_type_errors)]
    pub struct Shadowed {
        #[validate(required, length(max = 3))]
        pub name: ::std::string::String,
        #[validate(length(max = 3))]
        pub nickname: ::core::option::Option<::std::string::String>,
        #[validate(each(length(max = 3)))]
        pub tags: ::std::vec::Vec<::std::string::String>,
        #[validate(nested)]
        pub address: super::Address,
        #[serde(default)]
        #[validate(range(max = 3))]
        pub count: u8,
    }

    #[derive(Debug, annotated::Validate, annotated::Deserialize)]
    #[serde(tag = "kind")]
    pub enum Internal {
        One {
            #[validate(required)]
            name: ::std::string::String,
        },
        Two(#[validate(nested)] super::Address),
    }

    #[derive(Debug, annotated::Validate, annotated::Deserialize)]
    #[serde(untagged)]
    pub enum Untagged {
        One {
            #[validate(required)]
            name: ::std::string::String,
        },
        Two(#[validate(nested)] super::Address),
    }

    #[derive(Debug, annotated::Validate, annotated::Deserialize)]
    #[serde(tag = "t", content = "c")]
    pub enum Adjacent {
        One {
            #[validate(required)]
            name: ::std::string::String,
        },
        Two(#[validate(nested)] super::Address),
        Three,
    }
}

#[test]
fn user_items_named_as_std_s_change_nothing() {
    derives_both::<names_shadowing_std_s::Shadowed>();
    derives_both::<names_shadowing_std_s::Internal>();
    derives_both::<names_shadowing_std_s::Untagged>();
    derives_both::<names_shadowing_std_s::Adjacent>();
}

mod without_the_prelude {
    #![no_implicit_prelude]

    #[derive(Debug, ::annotated::Validate, ::annotated::Deserialize)]
    pub struct Account {
        #[validate(required, length(max = 3))]
        pub name: ::std::string::String,
        #[validate(length(max = 3))]
        pub nickname: ::core::option::Option<::std::string::String>,
        #[validate(each(nested))]
        pub children: ::std::vec::Vec<Account>,
        #[serde(default)]
        #[validate(range(max = 3))]
        pub count: u8,
        #[serde(flatten)]
        #[validate(nested)]
        pub address: super::Address,
    }

    #[derive(Debug, ::annotated::Validate, ::annotated::Deserialize)]
    #[annotated(collect_type_errors)]
    #[serde(tag = "t", content = "c")]
    pub enum Adjacent {
        One {
            #[validate(required)]
            name: ::std::string::String,
        },
        Two(#[validate(nested)] super::Address),
        Three(#[validate(range(max = 3))] u8, #[validate(range(max = 3))] u8),
        Four,
        #[serde(untagged)]
        Other(#[validate(required)] ::std::string::String),
    }

    #[derive(Debug, ::annotated::Validate, ::annotated::Deserialize)]
    #[serde(tag = "t")]
    pub enum Internal {
        One {
            #[validate(required)]
            name: ::std::string::String,
        },
        Two(#[validate(nested)] super::Address),
        Four,
    }
}

#[test]
fn a_module_without_the_prelude_derives_both() {
    derives_both::<without_the_prelude::Account>();
    derives_both::<without_the_prelude::Adjacent>();
    derives_both::<without_the_prelude::Internal>();
}

// Every struct and enum shape the derives accept, in every serde tagging, with the serde attributes they read.
mod every_shape {
    use annotated::{Deserialize, Validate};
    use serde::Deserializer;

    use super::Address;

    fn upper_case<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
        let text: String = serde::Deserialize::deserialize(deserializer)?;
        Ok(text.to_uppercase())
    }

    fn pair<'de, D: Deserializer<'de>>(deserializer: D) -> Result<(u8, String), D::Error> {
        serde::Deserialize::deserialize(deserializer)
    }

    fn three() -> u8 {
        3
    }

    #[derive(Debug, Validate, Deserialize)]
    pub struct Empty {}

    #[derive(Debug, Default, Validate, Deserialize)]
    #[serde(default, rename_all = "camelCase", deny_unknown_fields, expecting = "a struct")]
    #[validate(report_all)]
    pub struct Defaulted {
        #[serde(rename = "fullName", alias = "name")]
        #[validate(required, length(max = 10))]
        pub full_name: String,
        #[serde(default = "three")]
        #[validate(range(max = 5))]
        pub count: u8,
        #[serde(skip)]
        #[validate(length(max = 0))]
        pub skipped: Vec<u8>,
        #[serde(deserialize_with = "upper_case")]
        #[validate(required)]
        pub shouted: String,
    }

    #[derive(Debug, Validate, Deserialize)]
    #[annotated(collect_type_errors)]
    pub struct Flattened {
        #[validate(required)]
        pub name: String,
        #[serde(flatten)]
        #[validate(nested)]
        pub address: Address,
        #[serde(flatten)]
        #[validate(each(length(max = 10)))]
        pub rest: std::collections::BTreeMap<String, String>,
    }

    #[derive(Debug, Validate, Deserialize)]
    #[validate(custom(External::is_known))]
    pub enum External {
        Unit,
        Newtype(#[validate(nested)] Address),
        Tuple(#[validate(range(max = 3))] u8, #[validate(required)] String),
        #[validate(custom(External::is_known, field = name))]
        Struct {
            #[validate(required)]
            name: String,
            #[serde(flatten)]
            #[validate(nested)]
            address: Address,
        },
        #[serde(skip)]
        #[allow(dead_code, reason = "serde never reads a skipped variant")]
        Skipped(#[validate(required)] String),
        #[serde(deserialize_with = "pair")]
        ReadByAFunction(#[validate(range(max = 3))] u8, #[validate(required)] String),
        #[serde(untagged)]
        Untagged(#[validate(required)] String),
    }

    impl External {
        fn is_known(&self) -> Result<(), annotated::Problem> {
            if let Self::Untagged(text) = self
                && text == "unknown"
            {
                return Err(annotated::Problem::custom("unknown", "is unknown"));
            }
            Ok(())
        }
    }

    #[derive(Debug, Validate, Deserialize)]
    #[annotated(collect_type_errors)]
    #[serde(tag = "kind", rename_all = "snake_case", rename_all_fields = "camelCase")]
    pub enum Internal {
        Unit,
        Newtype(#[validate(nested)] Address),
        Struct {
            #[validate(required)]
            full_name: String,
        },
        #[serde(other)]
        Other,
    }

    #[derive(Debug, Validate, Deserialize)]
    #[serde(tag = "kind", content = "data", deny_unknown_fields)]
    pub enum Adjacent {
        Unit,
        Newtype(#[validate(nested)] Address),
        Optional(#[validate(length(max = 3))] Option<String>),
        Tuple(#[validate(range(max = 3))] u8, #[validate(required)] String),
        Struct {
            #[validate(required)]
            name: String,
        },
    }

    #[derive(Debug, Validate, Deserialize)]
    #[serde(untagged, expecting = "something")]
    pub enum Untagged {
        Unit,
        Newtype(#[validate(nested)] Address),
        Tuple(#[validate(range(max = 3))] u8, #[validate(required)] String),
        Struct {
            #[validate(required)]
            name: String,
        },
    }
}

#[test]
fn every_shape_derives_both_in_every_tagging() {
    derives_both::<every_shape::Empty>();
    derives_both::<every_shape::Defaulted>();
    derives_both::<every_shape::Flattened>();
    derives_both::<every_shape::External>();
    derives_both::<every_shape::Internal>();
    derives_both::<every_shape::Adjacent>();
    derives_both::<every_shape::Untagged>();
}
