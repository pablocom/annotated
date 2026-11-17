//! Both derives compile on shapes meant to break a macro, with every warning denied: raw identifiers, names the
//! generated code uses for its own, user types shadowing std's, a module without the prelude, and every struct and
//! enum the derives accept in every serde tagging. A shape the derives rightly refuse is a compile-fail case under
//! `tests/ui/` instead.

#![cfg(feature = "serde")]
#![deny(warnings)]

use annotated::{Deserialize, Validate};

fn derives_both<T: Validate + for<'de> Deserialize<'de>>() {}

#[derive(Debug, Default, Validate, Deserialize)]
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

// Type, const and lifetime parameters, with bounds, defaults and where clauses, borrowed fields, recursion, and bounds
// given by `#[serde(bound = "...")]`.
mod generics {
    use std::borrow::Cow;
    use std::collections::HashMap;
    use std::fmt::Display;
    use std::hash::Hash;
    use std::marker::PhantomData;

    use annotated::{Deserialize, Validate};

    use super::Address;

    #[derive(Debug, Validate, Deserialize)]
    pub struct Shelf<T: Clone, U = u32, const N: usize = 3>
    where
        U: Copy + Default,
    {
        #[validate(length(max = N), each(nested))]
        pub items: Vec<T>,
        #[serde(default)]
        #[validate(range(max = 3))]
        pub count: u8,
        #[serde(default)]
        #[validate(required)]
        pub unit: Option<U>,
    }

    #[derive(Debug, Validate, Deserialize)]
    pub struct Catalog<K: Eq + Hash + Display + Clone, V> {
        #[validate(each(nested))]
        pub entries: HashMap<K, V>,
    }

    #[derive(Debug, Validate, Deserialize)]
    pub struct Borrowed<'a, 'b: 'a, T> {
        #[validate(length(max = 5))]
        pub name: &'a str,
        #[serde(borrow)]
        #[validate(required)]
        pub note: Cow<'b, str>,
        #[validate(required)]
        pub value: Option<T>,
    }

    #[derive(Debug, Validate, Deserialize)]
    pub struct Tree<T> {
        #[validate(required)]
        pub value: Option<T>,
        #[validate(each(nested))]
        pub children: Vec<Tree<T>>,
    }

    #[derive(Debug, Validate, Deserialize)]
    pub enum Either<L, R> {
        Left(#[validate(nested)] L),
        Right {
            #[validate(each(nested))]
            values: Vec<R>,
        },
    }

    #[derive(Debug, Validate, Deserialize)]
    pub struct Defaulted<T> {
        #[serde(default)]
        #[validate(nested)]
        pub item: T,
        #[validate(length(max = 1))]
        pub kinds: Vec<PhantomData<T>>,
    }

    #[derive(Debug, PartialEq, Eq)]
    pub struct Label<Kind> {
        pub text: String,
        pub kind: PhantomData<Kind>,
    }

    impl<'de, Kind> serde::Deserialize<'de> for Label<Kind> {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let text = <String as serde::Deserialize>::deserialize(deserializer)?;
            Ok(Self {
                text,
                kind: PhantomData,
            })
        }
    }

    #[derive(Debug, Validate, Deserialize)]
    #[serde(bound(deserialize = "T: Validate"))]
    pub struct Given<T> {
        #[validate(length(max = 3))]
        pub label: Label<T>,
        #[serde(skip)]
        #[validate(each(nested))]
        pub items: Vec<T>,
    }

    #[derive(Debug, Validate, Deserialize)]
    pub struct GivenOnAField<T> {
        #[serde(bound(deserialize = ""))]
        #[validate(length(max = 3))]
        pub label: Label<T>,
    }

    #[derive(Debug, Validate, Deserialize)]
    #[serde(bound = "")]
    pub enum GivenOnAnEnum<T> {
        Labelled(#[validate(length(max = 3))] Label<T>),
        #[serde(bound(deserialize = "T: Default"))]
        Defaulted {
            #[serde(default)]
            #[validate(length(max = 3))]
            labels: Vec<Label<T>>,
            #[serde(skip)]
            #[validate(required)]
            value: Option<T>,
        },
    }

    impl<Kind> annotated::rules::HasLength for Label<Kind> {
        const UNIT: annotated::LengthUnit = annotated::LengthUnit::Chars;

        fn length(&self) -> usize {
            self.text.chars().count()
        }
    }

    #[derive(Debug, Clone, Default, Validate, Deserialize)]
    pub struct Unit {
        #[validate(range(max = 3))]
        pub value: u8,
    }

    pub struct Unreadable;

    #[test]
    fn generic_types_derive_both_with_the_bounds_their_fields_need() {
        super::derives_both::<Shelf<Unit>>();
        super::derives_both::<Shelf<Unit, u8, 4>>();
        super::derives_both::<Catalog<String, Address>>();
        super::derives_both::<Tree<u8>>();
        super::derives_both::<Either<Address, Unit>>();
        super::derives_both::<Defaulted<Unit>>();
        super::derives_both::<Given<Unit>>();
        super::derives_both::<GivenOnAField<Unreadable>>();
        super::derives_both::<GivenOnAnEnum<u8>>();
    }

    #[test]
    fn a_struct_borrowing_from_the_input_derives_both() {
        fn borrows<'de, T: Validate + Deserialize<'de>>() {}

        borrows::<Borrowed<'_, '_, u8>>();
    }
}

// `Self` in a field's type, in a bound and in a rule's arguments, in a struct, a generic one and an enum's variants.
mod self_in_the_input {
    use annotated::{Deserialize, Problem, Validate};

    #[derive(Debug, Validate, Deserialize)]
    #[validate(custom(Self::is_not_their_own_friend))]
    pub struct Member {
        #[validate(range(max = Self::MAX_AGE))]
        pub age: u8,
        #[validate(length(max = Self::MAX_FRIENDS), each(nested))]
        pub friends: Vec<Self>,
        #[validate(required)]
        pub mentor: Option<Box<Self>>,
        #[validate(custom(|name: &String| Self::is_a_name(name)))]
        pub name: String,
    }

    impl Member {
        const MAX_AGE: u8 = 120;
        const MAX_FRIENDS: usize = 10;

        fn is_not_their_own_friend(&self) -> Result<(), Problem> {
            if self.friends.iter().any(|friend| friend.name == self.name) {
                return Err(Problem::custom("own_friend", "is their own friend"));
            }
            Ok(())
        }

        fn is_a_name(name: &str) -> Result<(), Problem> {
            if name.is_empty() {
                return Err(Problem::custom("no_name", "is no name"));
            }
            Ok(())
        }
    }

    #[derive(Debug, Validate, Deserialize)]
    pub struct Ranked<T>
    where
        T: Clone,
        Option<Self>: Default,
    {
        #[validate(length(max = Self::MOST))]
        pub ranks: Vec<T>,
    }

    impl<T: Clone> Ranked<T> {
        const MOST: usize = 3;
    }

    #[derive(Debug, Validate, Deserialize)]
    pub enum Node {
        Leaf(#[validate(range(max = Self::DEEPEST))] u8),
        Branch {
            #[validate(each(nested))]
            children: Vec<Self>,
        },
    }

    impl Node {
        const DEEPEST: u8 = 4;
    }
}

#[test]
fn self_in_the_input_is_the_type_deriving() {
    derives_both::<self_in_the_input::Member>();
    derives_both::<self_in_the_input::Ranked<u8>>();
    derives_both::<self_in_the_input::Node>();
}

// Tokens a `macro_rules!` passes on keep its hygiene, and a type it passes as `$t:ty` arrives in an invisible group.
mod through_macros {
    use annotated::{Deserialize, Validate};
    use serde::Deserializer;

    use super::Address;

    fn trimmed<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Address, D::Error> {
        let street: String = serde::Deserialize::deserialize(deserializer)?;
        Ok(Address {
            street: street.trim().to_owned(),
        })
    }

    macro_rules! derived {
        ($item:item) => {
            #[derive(Debug, Validate, Deserialize)]
            $item
        };
    }

    derived! {
        pub struct Order {
            #[validate(length(max = 3))]
            pub reference: String,
            #[validate(each(nested))]
            pub stops: Vec<Address>,
            #[serde(default, deserialize_with = "trimmed")]
            #[validate(nested)]
            pub pickup: Address,
            #[serde(flatten)]
            #[validate(nested)]
            pub address: Address,
            #[serde(flatten)]
            #[validate(nested)]
            pub billing: Option<Address>,
        }
    }

    derived! {
        #[annotated(collect_type_errors)]
        pub struct Collected {
            #[validate(nested)]
            pub address: Address,
            #[serde(flatten)]
            #[validate(each(length(max = 3)))]
            pub rest: std::collections::BTreeMap<String, String>,
        }
    }

    derived! {
        #[serde(tag = "kind", content = "data")]
        pub enum Delivery {
            Courier(#[validate(nested)] Address),
            Pickup {
                #[validate(each(nested))]
                stops: Vec<Address>,
            },
        }
    }

    macro_rules! with_fields_of_types {
        ($name:ident, $text:ty, $address:ty) => {
            #[derive(Debug, Validate, Deserialize)]
            pub struct $name {
                #[validate(length(max = 3))]
                pub nickname: $text,
                #[validate(nested)]
                pub address: $address,
                #[validate(each(nested))]
                pub stops: Vec<$address>,
                #[serde(flatten)]
                #[validate(nested)]
                pub billing: Option<$address>,
            }
        };
    }

    with_fields_of_types!(Note, Option<String>, Address);
}

#[test]
fn items_and_types_passed_through_macros_derive_both() {
    derives_both::<through_macros::Order>();
    derives_both::<through_macros::Collected>();
    derives_both::<through_macros::Delivery>();
    derives_both::<through_macros::Note>();
}

// A crate that re-exports annotated, and serde, offers the derives under paths of its own.
mod through_a_re_export {
    mod facade {
        pub use annotated as checks;
        pub use serde as wire;
    }

    #[derive(Debug, facade::checks::Validate, facade::checks::Deserialize)]
    #[annotated(crate = "facade::checks", collect_type_errors)]
    #[serde(crate = "facade::wire")]
    pub struct Member {
        #[validate(required, length(max = 5))]
        pub name: String,
        #[validate(each(nested))]
        pub friends: Vec<Friend>,
        #[serde(flatten)]
        #[validate(nested)]
        pub friend: Friend,
    }

    #[derive(Debug, facade::checks::Validate, facade::checks::Deserialize)]
    #[annotated(crate = "self::facade::checks")]
    pub struct Friend {
        #[validate(required)]
        pub nickname: String,
    }

    #[derive(Debug, facade::checks::Validate, facade::checks::Deserialize)]
    #[annotated(crate = "facade::checks")]
    #[serde(tag = "kind", content = "data")]
    pub enum Payment {
        Card {
            #[validate(length(min = 12))]
            number: String,
        },
        Transfer(#[validate(nested)] Friend),
        Cash,
        #[serde(untagged)]
        Other(#[validate(required)] String),
    }
}

#[test]
fn a_crate_re_exporting_annotated_offers_the_derives_under_its_own_path() {
    derives_both::<through_a_re_export::Member>();
    derives_both::<through_a_re_export::Friend>();
    derives_both::<through_a_re_export::Payment>();
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
