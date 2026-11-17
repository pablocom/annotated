use std::any::type_name;
use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;

use serde::Deserializer;
use serde::de::{self, DeserializeSeed, EnumAccess, Error as _, VariantAccess, Visitor};

use crate::content::{Content, ValueKind};
use crate::content_deserializer::{
    ContentDeserializer, FunctionError, READ_IN_PLACE, ReadAs, TypeError, WrongType, is_annotated_s_flattened_entries,
};
use crate::deserialize::DeserializeAt;
use crate::{Deserialize, Location, Problem, Report};

/// The `DeserializeSeed` that a struct collecting type errors reads a field with.
///
/// It reads a `T` at a location in two steps: first into a buffer that holds any well-formed value, then from the
/// buffer as a `T`. A value of the wrong type is then pushed into a report as a [`Problem::InvalidType`], instead of
/// ending deserialization. A key a struct in the `T` refuses, given twice or one it denies, isn't a value of the wrong
/// type, and still ends deserialization with the format's error.
pub struct CollectingTypeErrorsAt<'a, 'l, T> {
    type_name: &'static str,
    at: &'a Location<'l>,
    found: &'a mut Report,
    read: PhantomData<fn() -> T>,
}

impl<'a, 'l, T> CollectingTypeErrorsAt<'a, 'l, T> {
    /// A seed that reads a `T` at `at` and pushes a value of the wrong type into `found`. `type_name` says what was
    /// expected when serde's error doesn't.
    pub fn new<'de>(type_name: &'static str, at: &'a Location<'l>, found: &'a mut Report) -> Self
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
        match read_collecting_type_errors(deserializer, PhantomData::<T>, self.type_name)? {
            Ok(value) => Ok(Some(value)),
            Err(problem) => {
                self.found.push(self.at, problem);
                Ok(None)
            }
        }
    }
}

/// Like [`CollectingTypeErrorsAt`], but it reads the `T` through [`Deserialize`], so the issues inside the `T` are
/// pushed into the report too. A value of the wrong type replaces the issues found in the `T` before it.
pub struct NestedCollectingTypeErrorsAt<'a, 'l, T> {
    type_name: &'static str,
    at: &'a Location<'l>,
    found: &'a mut Report,
    read: PhantomData<fn() -> T>,
}

impl<'a, 'l, T> NestedCollectingTypeErrorsAt<'a, 'l, T> {
    /// A seed that reads a `T` at `at` and pushes its issues into `found`, or a single issue when it has the
    /// wrong type. `type_name` says what was expected when serde's error doesn't.
    pub fn new<'de>(type_name: &'static str, at: &'a Location<'l>, found: &'a mut Report) -> Self
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
        let reading_its_issues = DeserializeAt::<T>::new(self.at, &mut *self.found);
        match read_collecting_type_errors(deserializer, reading_its_issues, self.type_name)? {
            Ok(value) => Ok(value),
            Err(problem) => {
                *self.found = self.found.new_alike();
                self.found.push(self.at, problem);
                Ok(None)
            }
        }
    }
}

/// Like [`CollectingTypeErrorsAt`], for a `T` whose `Deserialize` calls a function given to serde's
/// `deserialize_with`. A value of the wrong type for the function is pushed into the report. Any other error the
/// function returns ends deserialization with the function's message, which may repeat the input and so never
/// becomes an issue.
pub struct CollectingTypeErrorsOfAFunctionAt<'a, 'l, T> {
    type_name: &'static str,
    at: &'a Location<'l>,
    found: &'a mut Report,
    read: PhantomData<fn() -> T>,
}

impl<'a, 'l, T> CollectingTypeErrorsOfAFunctionAt<'a, 'l, T> {
    /// A seed that reads a `T` at `at` and pushes a value of the wrong type into `found`. `type_name` says what was
    /// expected when serde's error doesn't.
    pub fn new<'de>(type_name: &'static str, at: &'a Location<'l>, found: &'a mut Report) -> Self
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

impl<'de, T: serde::Deserialize<'de>> DeserializeSeed<'de> for CollectingTypeErrorsOfAFunctionAt<'_, '_, T> {
    type Value = Option<T>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Option<T>, D::Error> {
        let (buffered, kind_found) = buffered(deserializer)?;
        match T::deserialize(buffered) {
            Ok(value) => Ok(Some(value)),
            Err(FunctionError::WrongType(wrong_type)) => {
                let problem = problem_or_error(wrong_type, self.type_name, kind_found)?;
                self.found.push(self.at, problem);
                Ok(None)
            }
            Err(FunctionError::Message(message)) => Err(D::Error::custom(message)),
        }
    }
}

/// Reads an enum that collects type errors. The whole value is buffered first, so a name that is no variant of the
/// enum, or a value that isn't the enum at all, becomes the [`Problem`] returned instead of ending deserialization.
/// `type_name` says what was expected when serde's error doesn't.
///
/// A flattened enum is the first entry left that names one of its variants, as with serde, and only that entry is
/// buffered, so the others are left to the flattened fields after it.
///
/// # Errors
///
/// The format's own error, when the value isn't well-formed, when a struct in it refuses a key, given twice or one it
/// denies, or when no entry left names a variant of a flattened enum.
pub fn deserialize_enum_collecting_type_errors<'de, D, V, T>(
    deserializer: D,
    name: &'static str,
    variants: &'static [&'static str],
    visitor: V,
    type_name: &'static str,
) -> Result<Result<Option<T>, Problem>, D::Error>
where
    D: Deserializer<'de>,
    V: Visitor<'de, Value = Option<T>>,
{
    let reading_the_enum = EnumNamed {
        name,
        variants,
        visitor,
    };
    if is_annotated_s_flattened_entries::<D>() {
        return read_flattened(deserializer, reading_the_enum, type_name);
    }
    read_collecting_type_errors(deserializer, reading_the_enum, type_name)
}

fn read_flattened<'de, D, V>(
    deserializer: D,
    reading_the_enum: EnumNamed<V>,
    type_name: &'static str,
) -> Result<Result<V::Value, Problem>, D::Error>
where
    D: Deserializer<'de>,
    V: Visitor<'de>,
{
    let read_as = ReadAs::of_a_value_from(&deserializer);
    let entry = deserializer.deserialize_enum(reading_the_enum.name, reading_the_enum.variants, EntryNamingAVariant)?;
    match reading_the_enum.deserialize(ContentDeserializer::<TypeError>::new(entry, read_as)) {
        Ok(value) => Ok(Ok(value)),
        Err(error) => problem_or_error(error.into_wrong_type(), type_name, ValueKind::Map).map(Err),
    }
}

/// Takes the entry a flattened enum is read from, as a map of that entry alone.
struct EntryNamingAVariant;

impl<'de> Visitor<'de> for EntryNamingAVariant {
    type Value = Content<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an entry naming a variant")
    }

    fn visit_enum<A: EnumAccess<'de>>(self, entry: A) -> Result<Content<'de>, A::Error> {
        let (variant, content) = entry.variant::<Content<'de>>()?;
        let content = content.newtype_variant::<Content<'de>>()?;
        Ok(Content::Map(vec![(variant, content)]))
    }
}

/// Like [`deserialize_enum_collecting_type_errors`], for an adjacently tagged enum, which is read as a struct with the
/// `fields` of its tag and its content.
///
/// # Errors
///
/// The format's own error, when the value isn't well-formed, or when the enum or a struct in it refuses a key, given
/// twice or one it denies.
pub fn deserialize_adjacently_tagged_collecting_type_errors<'de, D, V, T>(
    deserializer: D,
    name: &'static str,
    fields: &'static [&'static str],
    visitor: V,
    type_name: &'static str,
) -> Result<Result<Option<T>, Problem>, D::Error>
where
    D: Deserializer<'de>,
    V: Visitor<'de, Value = Option<T>>,
{
    let reading_the_struct = StructNamed { name, fields, visitor };
    read_collecting_type_errors(deserializer, reading_the_struct, type_name)
}

struct EnumNamed<V> {
    name: &'static str,
    variants: &'static [&'static str],
    visitor: V,
}

impl<'de, V: Visitor<'de>> DeserializeSeed<'de> for EnumNamed<V> {
    type Value = V::Value;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<V::Value, D::Error> {
        deserializer.deserialize_enum(self.name, self.variants, self.visitor)
    }
}

struct StructNamed<V> {
    name: &'static str,
    fields: &'static [&'static str],
    visitor: V,
}

impl<'de, V: Visitor<'de>> DeserializeSeed<'de> for StructNamed<V> {
    type Value = V::Value;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<V::Value, D::Error> {
        deserializer.deserialize_struct(self.name, self.fields, self.visitor)
    }
}

/// Reads a value with `seed` from a buffer, so that a value of the wrong type is the [`Problem`] returned instead of
/// ending deserialization. `type_name` says what was expected when serde's error doesn't.
///
/// A value already in annotated's buffer, because a type around it collects type errors too, is read where it is:
/// buffering it again would copy it once for every type around it.
fn read_collecting_type_errors<'de, D, S>(
    deserializer: D,
    seed: S,
    type_name: &'static str,
) -> Result<Result<S::Value, Problem>, D::Error>
where
    D: Deserializer<'de>,
    S: DeserializeSeed<'de>,
{
    if may_be_annotated_s_buffer::<D>() {
        return deserializer.deserialize_newtype_struct(READ_IN_PLACE, ReadInPlace { seed, type_name });
    }
    read_buffered(deserializer, seed, type_name)
}

// Rust can't ask whether `D` is annotated's buffer: there's no specialization, and `TypeId` needs a type that borrows
// nothing. So the buffer itself answers a request for a newtype struct named `READ_IN_PLACE`. Any other deserializer
// would read a newtype struct from the input, and not every format reads one as JSON does: RON expects parentheses. So
// only a deserializer whose type bears the buffer's name is asked. Only a buffer bears it, of this version of annotated
// or of another, which answers as for any newtype struct.
fn may_be_annotated_s_buffer<'de, D: Deserializer<'de>>() -> bool {
    type_name::<D>() == type_name::<ContentDeserializer<'static, TypeError>>()
}

fn read_buffered<'de, D, S>(
    deserializer: D,
    seed: S,
    type_name: &'static str,
) -> Result<Result<S::Value, Problem>, D::Error>
where
    D: Deserializer<'de>,
    S: DeserializeSeed<'de>,
{
    let (buffered, kind_found) = buffered::<D, TypeError>(deserializer)?;
    match seed.deserialize(buffered) {
        Ok(value) => Ok(Ok(value)),
        Err(error) => problem_or_error(error.into_wrong_type(), type_name, kind_found).map(Err),
    }
}

/// Reads with `seed` the value annotated's buffer offers to read in place, as an enum whose variant is the kind of the
/// value and whose content is the value.
struct ReadInPlace<S> {
    seed: S,
    type_name: &'static str,
}

impl<'de, S: DeserializeSeed<'de>> Visitor<'de> for ReadInPlace<S> {
    type Value = Result<S::Value, Problem>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a value in annotated's buffer")
    }

    fn visit_enum<A: EnumAccess<'de>>(self, in_place: A) -> Result<Self::Value, A::Error> {
        let (kind_found, value): (&str, _) = in_place.variant()?;
        match value.newtype_variant_seed(self.seed) {
            Ok(value) => Ok(Ok(value)),
            Err(error) => match (TypeError::take_wrong_type(&error), ValueKind::named(kind_found)) {
                (Some(wrong_type), Some(kind_found)) => {
                    problem_or_error(wrong_type, self.type_name, kind_found).map(Err)
                }
                _ => Err(error),
            },
        }
    }

    // The buffer of another version of annotated answers as for any newtype struct.
    fn visit_newtype_struct<D: Deserializer<'de>>(self, buffer: D) -> Result<Self::Value, D::Error> {
        read_buffered(buffer, self.seed, self.type_name)
    }
}

fn buffered<'de, D: Deserializer<'de>, E>(
    deserializer: D,
) -> Result<(ContentDeserializer<'de, E>, ValueKind), D::Error> {
    let read_as = ReadAs::of_a_value_from(&deserializer);
    let content = <Content as serde::Deserialize>::deserialize(deserializer)?;
    let kind = content.kind();
    Ok((ContentDeserializer::new(content, read_as), kind))
}

/// The problem to report for a value that doesn't fit its type, or the format's error for a key its type refuses,
/// which isn't a value of the wrong type. `type_name` says what was expected when serde's error doesn't.
#[cold]
pub(crate) fn problem_or_error<E: de::Error>(
    wrong_type: WrongType,
    type_name: &'static str,
    kind_found: ValueKind,
) -> Result<Problem, E> {
    match wrong_type {
        WrongType::Mismatched { expected, found } => Ok(Problem::InvalidType {
            expected: expected.map_or(Cow::Borrowed(type_name), Cow::Owned),
            found: found.unwrap_or(kind_found).name(),
        }),
        WrongType::UnknownVariant { expected } => Ok(Problem::UnknownVariant { expected }),
        WrongType::RefusedKey(key) => Err((*key).into_error()),
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize as _;
    use serde::de::IgnoredAny;
    use serde_test::{Configure, Token, assert_de_tokens};

    use crate::{Issue, Issues, Path};

    use super::*;

    fn reading<'de, T: serde::Deserialize<'de>>(
        json: &'de str,
        type_name: &'static str,
        at: &Location<'_>,
    ) -> (Result<Option<T>, String>, Result<(), Issues>) {
        let mut found = Report::new();
        let read = CollectingTypeErrorsAt::<T>::new(type_name, at, &mut found)
            .deserialize(&mut serde_json::Deserializer::from_str(json))
            .map_err(|format_error| format_error.to_string());
        (read, found.into_result())
    }

    fn reading_nested<'de, T: Deserialize<'de>>(
        json: &'de str,
        type_name: &'static str,
        at: &Location<'_>,
    ) -> (Result<Option<T>, String>, Result<(), Issues>) {
        let mut found = Report::new();
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

    fn issues(issues: Vec<Issue>) -> Result<(), Issues> {
        Err(Issues::of(issues).expect("at least one issue"))
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
            found: &mut Report,
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
            _: &mut Report,
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
            let mut found = Report::new();
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
                issues(vec![Issue::new(
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
                issues(vec![Issue::new(
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
    fn a_nested_value_of_its_type_is_read_with_its_issues_at_their_paths() {
        let root = Location::root();

        assert_eq!(
            reading_nested::<Vec<Handle>>(r#"["@lucia", "marcos"]"#, "Vec", &root.field("handles")),
            (
                Ok(Some(vec![Handle("@lucia".to_owned()), Handle("marcos".to_owned())])),
                issues(vec![Issue::new(
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
                issues(vec![Issue::new(
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
                issues(vec![Issue::new(
                    Path::root().field("refused"),
                    invalid_type("Refused", "sequence")
                )])
            )
        );
    }

    #[test]
    fn a_value_not_offered_to_be_read_in_place_is_buffered_and_read() {
        let mut deserializer = serde_json::Deserializer::from_str(r#"[1, "x"]"#);

        let read = deserializer
            .deserialize_newtype_struct(
                READ_IN_PLACE,
                ReadInPlace {
                    seed: PhantomData::<Vec<u8>>,
                    type_name: "Vec",
                },
            )
            .map_err(|format_error| format_error.to_string());

        assert_eq!(read, Ok(Err(invalid_type("u8", "string"))));
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
