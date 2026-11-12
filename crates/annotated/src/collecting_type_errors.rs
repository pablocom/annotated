use std::borrow::Cow;
use std::marker::PhantomData;

use serde::Deserializer;
use serde::de::DeserializeSeed;

use crate::content::{Content, ValueKind};
use crate::content_deserializer::{ContentDeserializer, WrongType};
use crate::{Deserialize, FoundViolations, Location, Problem};

/// The `DeserializeSeed` that a struct collecting type errors reads a field with.
///
/// It reads a `T` at a location in two steps: first into a buffer that holds any well-formed value, then from the
/// buffer as a `T`. A value of the wrong type is then pushed into a report as a [`Problem::InvalidType`], instead of
/// ending deserialization.
pub struct CollectingTypeErrorsAt<'a, 'l, T> {
    type_name: &'static str,
    at: &'a Location<'l>,
    found: &'a mut FoundViolations,
    read: PhantomData<fn() -> T>,
}

impl<'a, 'l, T> CollectingTypeErrorsAt<'a, 'l, T> {
    /// A seed that reads a `T` at `at` and pushes a value of the wrong type into `found`. `type_name` says what was
    /// expected when serde's error doesn't.
    pub fn new<'de>(type_name: &'static str, at: &'a Location<'l>, found: &'a mut FoundViolations) -> Self
    where
        T: serde::Deserialize<'de>,
    {
        Self {
            type_name,
            at,
            found,
            read: PhantomData,
        }
    }
}

impl<'de, T: serde::Deserialize<'de>> DeserializeSeed<'de> for CollectingTypeErrorsAt<'_, '_, T> {
    type Value = Option<T>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Option<T>, D::Error> {
        let (buffered, kind_found) = buffered(deserializer)?;
        match T::deserialize(buffered) {
            Ok(value) => Ok(Some(value)),
            Err(wrong_type) => {
                self.found
                    .push(self.at, invalid_type(wrong_type, self.type_name, kind_found));
                Ok(None)
            }
        }
    }
}

/// Like [`CollectingTypeErrorsAt`], but it reads the `T` through [`Deserialize`], so the violations inside the `T` are
/// pushed into the report too. A value of the wrong type replaces the violations found in the `T` before it.
pub struct NestedCollectingTypeErrorsAt<'a, 'l, T> {
    type_name: &'static str,
    at: &'a Location<'l>,
    found: &'a mut FoundViolations,
    read: PhantomData<fn() -> T>,
}

impl<'a, 'l, T> NestedCollectingTypeErrorsAt<'a, 'l, T> {
    /// A seed that reads a `T` at `at` and pushes its violations into `found`, or a single violation when it has the
    /// wrong type. `type_name` says what was expected when serde's error doesn't.
    pub fn new<'de>(type_name: &'static str, at: &'a Location<'l>, found: &'a mut FoundViolations) -> Self
    where
        T: Deserialize<'de>,
    {
        Self {
            type_name,
            at,
            found,
            read: PhantomData,
        }
    }
}

impl<'de, T: Deserialize<'de>> DeserializeSeed<'de> for NestedCollectingTypeErrorsAt<'_, '_, T> {
    type Value = Option<T>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Option<T>, D::Error> {
        let (buffered, kind_found) = buffered(deserializer)?;
        match T::deserialize_at(buffered, self.at, self.found) {
            Ok(value) => Ok(value),
            Err(wrong_type) => {
                *self.found = self.found.new_alike();
                self.found
                    .push(self.at, invalid_type(wrong_type, self.type_name, kind_found));
                Ok(None)
            }
        }
    }
}

fn buffered<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<(ContentDeserializer<'de, WrongType>, ValueKind), D::Error> {
    let is_human_readable = deserializer.is_human_readable();
    let content = <Content as serde::Deserialize>::deserialize(deserializer)?;
    let kind = content.kind();
    Ok((ContentDeserializer::new(content, is_human_readable), kind))
}

#[cold]
fn invalid_type(wrong_type: WrongType, type_name: &'static str, kind_found: ValueKind) -> Problem {
    Problem::InvalidType {
        expected: wrong_type.expected.map_or(Cow::Borrowed(type_name), Cow::Owned),
        found: wrong_type.found.unwrap_or(kind_found).name(),
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize as _;
    use serde::de::Error as _;
    use serde::de::IgnoredAny;
    use serde_test::{Configure, Token, assert_de_tokens};

    use crate::{Path, Violation, Violations};

    use super::*;

    fn reading<'de, T: serde::Deserialize<'de>>(
        json: &'de str,
        type_name: &'static str,
        at: &Location<'_>,
    ) -> (Result<Option<T>, String>, Result<(), Violations>) {
        let mut found = FoundViolations::new();
        let read = CollectingTypeErrorsAt::<T>::new(type_name, at, &mut found)
            .deserialize(&mut serde_json::Deserializer::from_str(json))
            .map_err(|format_error| format_error.to_string());
        (read, found.into_result())
    }

    fn reading_nested<'de, T: Deserialize<'de>>(
        json: &'de str,
        type_name: &'static str,
        at: &Location<'_>,
    ) -> (Result<Option<T>, String>, Result<(), Violations>) {
        let mut found = FoundViolations::new();
        let read = NestedCollectingTypeErrorsAt::<T>::new(type_name, at, &mut found)
            .deserialize(&mut serde_json::Deserializer::from_str(json))
            .map_err(|format_error| format_error.to_string());
        (read, found.into_result())
    }

    fn invalid_type(expected: &'static str, found: &'static str) -> Problem {
        Problem::InvalidType {
            expected: Cow::Borrowed(expected),
            found,
        }
    }

    fn violations(violations: Vec<Violation>) -> Result<(), Violations> {
        Err(Violations::of(violations).expect("at least one violation"))
    }

    #[derive(Debug, PartialEq)]
    struct Handle(String);

    impl<'de> serde::Deserialize<'de> for Handle {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let handle = String::deserialize(deserializer)?;
            if !handle.starts_with('@') {
                return Err(D::Error::custom(format!("{handle} doesn't start with @")));
            }
            Ok(Self(handle))
        }
    }

    impl<'de> Deserialize<'de> for Handle {
        fn deserialize_at<D: Deserializer<'de>>(
            deserializer: D,
            at: &Location<'_>,
            found: &mut FoundViolations,
        ) -> Result<Option<Self>, D::Error> {
            let handle = String::deserialize(deserializer)?;
            if !handle.starts_with('@') {
                found.push(at, Problem::custom("handle", "must start with @"));
            }
            Ok(Some(Self(handle)))
        }
    }

    #[derive(Debug, PartialEq)]
    struct Refused;

    impl<'de> Deserialize<'de> for Refused {
        fn deserialize_at<D: Deserializer<'de>>(
            deserializer: D,
            _: &Location<'_>,
            _: &mut FoundViolations,
        ) -> Result<Option<Self>, D::Error> {
            IgnoredAny::deserialize(deserializer)?;
            Err(D::Error::custom("refused"))
        }
    }

    #[derive(Debug, PartialEq)]
    struct HumanReadability(bool);

    impl<'de> serde::Deserialize<'de> for HumanReadability {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let is_human_readable = deserializer.is_human_readable();
            IgnoredAny::deserialize(deserializer)?;
            Ok(Self(is_human_readable))
        }
    }

    #[derive(Debug, PartialEq)]
    struct ReadCollectingTypeErrors<T>(Option<T>);

    impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for ReadCollectingTypeErrors<T> {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let mut found = FoundViolations::new();
            let read =
                CollectingTypeErrorsAt::<T>::new("T", &Location::root(), &mut found).deserialize(deserializer)?;
            Ok(Self(read))
        }
    }

    #[test]
    fn a_value_of_its_type_is_read() {
        let root = Location::root();

        assert_eq!(reading::<u8>("30", "u8", &root.field("age")), (Ok(Some(30)), Ok(())));
    }

    #[test]
    fn a_value_of_the_wrong_type_is_reported_at_its_location_by_what_serde_expects_and_the_kind_found() {
        let root = Location::root();

        assert_eq!(
            reading::<u8>(r#""twelve""#, "u8", &root.field("age")),
            (
                Ok(None),
                violations(vec![Violation::new(
                    Path::root().field("age"),
                    invalid_type("u8", "string")
                )])
            )
        );
    }

    #[test]
    fn a_value_refused_without_a_reason_serde_structures_is_reported_by_its_type_s_name_and_its_kind() {
        let root = Location::root();

        assert_eq!(
            reading::<Handle>(r#""lucia""#, "Handle", &root.field("handle")),
            (
                Ok(None),
                violations(vec![Violation::new(
                    Path::root().field("handle"),
                    invalid_type("Handle", "string")
                )])
            )
        );
    }

    #[test]
    fn a_value_that_is_not_well_formed_is_the_format_s_error() {
        let root = Location::root();

        assert_eq!(
            reading::<u8>(r#""twel"#, "u8", &root.field("age")),
            (Err("EOF while parsing a string at line 1 column 5".to_owned()), Ok(()))
        );
    }

    #[test]
    fn the_value_is_read_as_human_readable_or_not_as_the_format_reads_it() {
        assert_de_tokens(
            &ReadCollectingTypeErrors(Some(HumanReadability(false))).compact(),
            &[Token::Unit],
        );
    }

    #[test]
    fn a_nested_value_of_its_type_is_read_with_its_violations_at_their_paths() {
        let root = Location::root();

        assert_eq!(
            reading_nested::<Vec<Handle>>(r#"["@lucia", "marcos"]"#, "Vec", &root.field("handles")),
            (
                Ok(Some(vec![Handle("@lucia".to_owned()), Handle("marcos".to_owned())])),
                violations(vec![Violation::new(
                    Path::root().field("handles").index(1),
                    Problem::custom("handle", "must start with @")
                )])
            )
        );
    }

    #[test]
    fn a_nested_value_of_the_wrong_type_is_reported_alone_at_its_location() {
        let root = Location::root();

        assert_eq!(
            reading_nested::<Vec<Handle>>(r#"["marcos", 5]"#, "Vec", &root.field("handles")),
            (
                Ok(None),
                violations(vec![Violation::new(
                    Path::root().field("handles"),
                    invalid_type("a string", "integer")
                )])
            )
        );
    }

    #[test]
    fn a_nested_value_refused_without_a_reason_serde_structures_is_reported_by_its_type_s_name_and_its_kind() {
        let root = Location::root();

        assert_eq!(
            reading_nested::<Refused>("[1, 2]", "Refused", &root.field("refused")),
            (
                Ok(None),
                violations(vec![Violation::new(
                    Path::root().field("refused"),
                    invalid_type("Refused", "sequence")
                )])
            )
        );
    }

    #[test]
    fn a_nested_value_that_is_not_well_formed_is_the_format_s_error() {
        let root = Location::root();

        assert_eq!(
            reading_nested::<Vec<Handle>>(r#"["@lucia""#, "Vec", &root.field("handles")),
            (Err("EOF while parsing a list at line 1 column 9".to_owned()), Ok(()))
        );
    }
}
