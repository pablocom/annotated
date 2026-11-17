//! A body with no issue is read as serde's own derive reads it: the same value, or an error where serde gives one.
//!
//! Each shape is declared once and derived four times by `read_by_both!`: with serde's derive, with annotated's, with
//! annotated's collecting type errors at the root of the shape, and with annotated's collecting type errors in every
//! type of it. Each derive reads the same bodies, written below as JSON, in JSON and in MessagePack, and annotated's
//! must agree with serde's on each: both read values that print alike, or both refuse the body. annotated may refuse
//! with issues where serde returns its error, which is the one difference the promise allows.

#![cfg(feature = "serde")]

use std::collections::BTreeMap;
use std::fmt::{self, Debug};

use serde::de::{DeserializeOwned, MapAccess, SeqAccess, Visitor};
use serde::ser::{SerializeMap, SerializeSeq};

/// Declares the items of a shape in four modules: `by_serde`, deriving serde's `Deserialize`, `by_annotated`, deriving
/// annotated's, `collecting_at_the_root`, where the first item collects type errors, and `collecting_throughout`,
/// where every item does. serde's side also derives `Validate`, which takes the `#[validate(...)]` and
/// `#[annotated(...)]` attributes and reads nothing.
macro_rules! read_by_both {
    (
        $(#[$root_attribute:meta])* $root_kind:ident $root:ident $root_body:tt
        $($(#[$attribute:meta])* $kind:ident $name:ident $body:tt)*
    ) => {
        #[allow(dead_code)]
        mod by_serde {
            #[allow(unused_imports)]
            use super::*;

            #[derive(serde::Deserialize, annotated::Validate, Debug)]
            $(#[$root_attribute])*
            pub $root_kind $root $root_body

            $(
                #[derive(serde::Deserialize, annotated::Validate, Debug)]
                $(#[$attribute])*
                pub $kind $name $body
            )*
        }

        #[allow(dead_code)]
        mod by_annotated {
            #[allow(unused_imports)]
            use super::*;

            #[derive(annotated::Deserialize, Debug)]
            $(#[$root_attribute])*
            pub $root_kind $root $root_body

            $(
                #[derive(annotated::Deserialize, Debug)]
                $(#[$attribute])*
                pub $kind $name $body
            )*
        }

        #[allow(dead_code)]
        mod collecting_at_the_root {
            #[allow(unused_imports)]
            use super::*;

            #[derive(annotated::Deserialize, Debug)]
            #[annotated(collect_type_errors)]
            $(#[$root_attribute])*
            pub $root_kind $root $root_body

            $(
                #[derive(annotated::Deserialize, Debug)]
                $(#[$attribute])*
                pub $kind $name $body
            )*
        }

        #[allow(dead_code)]
        mod collecting_throughout {
            #[allow(unused_imports)]
            use super::*;

            #[derive(annotated::Deserialize, Debug)]
            #[annotated(collect_type_errors)]
            $(#[$root_attribute])*
            pub $root_kind $root $root_body

            $(
                #[derive(annotated::Deserialize, Debug)]
                #[annotated(collect_type_errors)]
                $(#[$attribute])*
                pub $kind $name $body
            )*
        }
    };
}

/// A unit struct, which neither derive takes, read by serde's derive inside both.
#[derive(serde::Deserialize, Debug)]
struct Marker;

/// The format a body is read in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Format {
    Json,
    MessagePack,
}

/// A body that annotated is known to read otherwise than serde does, in some formats. Each list of them says why.
struct KnownDifference {
    formats: &'static [Format],
    body: &'static str,
}

const fn in_json(body: &'static str) -> KnownDifference {
    KnownDifference {
        formats: &[Format::Json],
        body,
    }
}

const fn in_message_pack(body: &'static str) -> KnownDifference {
    KnownDifference {
        formats: &[Format::MessagePack],
        body,
    }
}

const fn in_both_formats(body: &'static str) -> KnownDifference {
    KnownDifference {
        formats: &[Format::Json, Format::MessagePack],
        body,
    }
}

/// What reading a body came to: a value, printed, or a refusal and why.
enum Outcome {
    Read(String),
    Refused(String),
}

impl Outcome {
    fn agrees_with(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Read(value), Self::Read(other_value)) => value == other_value,
            (Self::Refused(_), Self::Refused(_)) => true,
            _ => false,
        }
    }
}

impl Debug for Outcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read(value) => write!(formatter, "reads {value}"),
            Self::Refused(reason) => write!(formatter, "refuses: {reason}"),
        }
    }
}

/// Reads every body in both formats with serde's derive, `S`, and with annotated's, `A`, and fails listing each body
/// they disagree on, but those `known` to differ. A known difference that no longer differs fails too, so the list
/// stays true.
fn assert_each_body_reads_as_serde_reads_it<S, A>(shape: &str, bodies: &[&str], known: &[&[KnownDifference]])
where
    S: DeserializeOwned + Debug,
    A: for<'de> annotated::Deserialize<'de> + Debug,
{
    let mut disagreements = Vec::new();
    for format in [Format::Json, Format::MessagePack] {
        for &body in bodies {
            let by_serde = read_by_serde::<S>(format, body);
            let by_annotated = read_by_annotated::<A>(format, body);
            let is_known = known
                .iter()
                .copied()
                .flatten()
                .any(|difference| difference.formats.contains(&format) && difference.body == body);
            match (by_serde.agrees_with(&by_annotated), is_known) {
                (false, false) => disagreements.push(format!(
                    "{format:?} {body}\n    serde's derive:     {by_serde:?}\n    annotated's derive: {by_annotated:?}"
                )),
                (true, true) => disagreements.push(format!(
                    "{format:?} {body} is listed as a known difference, but both derives agree: {by_serde:?}"
                )),
                _ => {}
            }
        }
    }
    assert!(
        disagreements.is_empty(),
        "{shape}: annotated's derive reads {} bodies otherwise than serde's\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );
}

fn read_by_serde<S: DeserializeOwned + Debug>(format: Format, body: &str) -> Outcome {
    let read = match format {
        Format::Json => serde_json::from_str::<S>(body).map_err(|error| error.to_string()),
        Format::MessagePack => rmp_serde::from_slice::<S>(&message_pack_of(body)).map_err(|error| error.to_string()),
    };
    outcome_of(read)
}

fn read_by_annotated<A>(format: Format, body: &str) -> Outcome
where
    A: for<'de> annotated::Deserialize<'de> + Debug,
{
    let read = match format {
        Format::Json => {
            let mut deserializer = serde_json::Deserializer::from_str(body);
            annotated::from_deserializer::<A, _>(&mut deserializer)
                .map_err(|error| error.to_string())
                .and_then(|value| deserializer.end().map(|()| value).map_err(|error| error.to_string()))
        }
        Format::MessagePack => {
            let bytes = message_pack_of(body);
            let mut deserializer = rmp_serde::Deserializer::from_read_ref(&bytes);
            annotated::from_deserializer::<A, _>(&mut deserializer).map_err(|error| error.to_string())
        }
    };
    outcome_of(read)
}

fn outcome_of<T: Debug>(read: Result<T, String>) -> Outcome {
    match read {
        Ok(value) => Outcome::Read(format!("{value:?}")),
        Err(reason) => Outcome::Refused(reason),
    }
}

/// The body, written in JSON, as MessagePack: with its keys in the same order, repeated keys included.
fn message_pack_of(body: &str) -> Vec<u8> {
    let json: Json = serde_json::from_str(body).expect("a body is well-formed JSON");
    rmp_serde::to_vec(&json).expect("any JSON value can be written as MessagePack")
}

/// A JSON value that keeps its keys in order, a key given twice included, so that it is written as it was read.
enum Json {
    Null,
    Boolean(bool),
    Unsigned(u64),
    Signed(i64),
    Float(f64),
    Text(String),
    List(Vec<Json>),
    Object(Vec<(String, Json)>),
}

impl<'de> serde::Deserialize<'de> for Json {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(JsonVisitor)
    }
}

struct JsonVisitor;

impl<'de> Visitor<'de> for JsonVisitor {
    type Value = Json;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value")
    }

    fn visit_unit<E>(self) -> Result<Json, E> {
        Ok(Json::Null)
    }

    fn visit_bool<E>(self, boolean: bool) -> Result<Json, E> {
        Ok(Json::Boolean(boolean))
    }

    fn visit_u64<E>(self, integer: u64) -> Result<Json, E> {
        Ok(Json::Unsigned(integer))
    }

    fn visit_i64<E>(self, integer: i64) -> Result<Json, E> {
        Ok(Json::Signed(integer))
    }

    fn visit_f64<E>(self, float: f64) -> Result<Json, E> {
        Ok(Json::Float(float))
    }

    fn visit_str<E>(self, text: &str) -> Result<Json, E> {
        Ok(Json::Text(text.to_owned()))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Json, A::Error> {
        let mut elements = Vec::new();
        while let Some(element) = seq.next_element()? {
            elements.push(element);
        }
        Ok(Json::List(elements))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Json, A::Error> {
        let mut entries = Vec::new();
        while let Some(entry) = map.next_entry()? {
            entries.push(entry);
        }
        Ok(Json::Object(entries))
    }
}

impl serde::Serialize for Json {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Boolean(boolean) => serializer.serialize_bool(*boolean),
            Self::Unsigned(integer) => serializer.serialize_u64(*integer),
            Self::Signed(integer) => serializer.serialize_i64(*integer),
            Self::Float(float) => serializer.serialize_f64(*float),
            Self::Text(text) => serializer.serialize_str(text),
            Self::List(elements) => {
                let mut seq = serializer.serialize_seq(Some(elements.len()))?;
                for element in elements {
                    seq.serialize_element(element)?;
                }
                seq.end()
            }
            Self::Object(entries) => {
                let mut map = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
        }
    }
}

mod a_struct {
    use super::*;

    read_by_both! {
        struct Member {
            id: u8,
            nickname: Option<String>,
            #[validate(nested)]
            address: Option<Address>,
        }

        #[serde(deny_unknown_fields)]
        struct Address {
            street: String,
            number: Option<u16>,
        }
    }

    const BODIES: &[&str] = &[
        r#"{"id": 1}"#,
        r#"{"id": 1, "nickname": "Al"}"#,
        r#"{"id": 1, "nickname": null}"#,
        r#"{"nickname": "Al", "id": 1}"#,
        "{}",
        r#"{"nickname": "Al"}"#,
        r#"{"id": 1, "id": 2}"#,
        r#"{"id": 1, "nickname": "Al", "nickname": "Bo"}"#,
        r#"{"id": 1, "unknown": true}"#,
        r#"{"id": "1"}"#,
        r#"{"id": -1}"#,
        r#"{"id": 1.5}"#,
        r#"{"id": 256}"#,
        r#"{"id": null}"#,
        r#"{"id": true}"#,
        r#"{"id": [1]}"#,
        r#"{"id": {}}"#,
        r#"{"id": 1, "nickname": 1}"#,
        r#"{"id": 1, "nickname": []}"#,
        r#"{"id": 1, "address": {"street": "Gran Vía"}}"#,
        r#"{"id": 1, "address": {"number": 1, "street": "Gran Vía"}}"#,
        r#"{"id": 1, "address": {"street": "Gran Vía", "street": "Alcalá"}}"#,
        r#"{"id": 1, "address": {"street": "Gran Vía", "zip": 1}}"#,
        r#"{"id": 1, "address": {}}"#,
        r#"{"id": 1, "address": null}"#,
        r#"{"id": 1, "address": ["Gran Vía"]}"#,
        r#"{"id": 1, "address": ["Gran Vía", 1]}"#,
        r#"{"id": 1, "address": ["Gran Vía", 1, 2]}"#,
        r#"{"id": 1, "address": "Gran Vía"}"#,
        r#"{"id": 1, "address": {"street": 1}}"#,
        "[1]",
        r#"[1, "Al"]"#,
        r#"[1, "Al", {"street": "Gran Vía"}]"#,
        r#"[1, "Al", null, 4]"#,
        "[]",
        "1",
        r#""Al""#,
        "null",
        "true",
    ];

    /// An `Option` field whose element is missing from the end of a sequence is `None` to annotated. serde takes it
    /// for a sequence of the wrong length, unless the field has a default.
    const OPTIONS_MISSING_FROM_A_SEQUENCE: &[KnownDifference] = &[
        in_both_formats(r#"{"id": 1, "address": ["Gran Vía"]}"#),
        in_both_formats("[1]"),
        in_both_formats(r#"[1, "Al"]"#),
    ];

    #[test]
    fn a_struct_reads_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Member, by_annotated::Member>(
            "a struct",
            BODIES,
            &[OPTIONS_MISSING_FROM_A_SEQUENCE],
        );
    }

    #[test]
    fn a_struct_collecting_type_errors_reads_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Member, collecting_at_the_root::Member>(
            "a struct collecting type errors",
            BODIES,
            &[OPTIONS_MISSING_FROM_A_SEQUENCE],
        );
    }

    #[test]
    fn a_struct_collecting_type_errors_throughout_reads_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Member, collecting_throughout::Member>(
            "a struct collecting type errors throughout",
            BODIES,
            &[OPTIONS_MISSING_FROM_A_SEQUENCE],
        );
    }
}

mod an_externally_tagged_enum {
    use super::*;

    read_by_both! {
        enum Payment {
            Cash,
            Card(#[validate(nested)] Card),
            Nothing(()),
            Marked(Marker),
            Labels(BTreeMap<String, u8>),
            Point(u8, u8),
            Transfer { iban: String, reference: Option<String> },
        }

        struct Card {
            number: String,
        }
    }

    const BODIES: &[&str] = &[
        r#""Cash""#,
        r#"{"Cash": null}"#,
        r#"{"Cash": []}"#,
        r#"{"Cash": {}}"#,
        r#"{"Card": {"number": "4111"}}"#,
        r#"{"Card": ["4111"]}"#,
        r#"{"Card": {}}"#,
        r#"{"Card": null}"#,
        r#"{"Card": {"number": 4111}}"#,
        r#"{"Nothing": null}"#,
        r#"{"Nothing": {}}"#,
        r#"{"Nothing": []}"#,
        r#""Nothing""#,
        r#"{"Marked": null}"#,
        r#"{"Marked": {}}"#,
        r#"{"Marked": []}"#,
        r#""Marked""#,
        r#"{"Labels": {"en": 1}}"#,
        r#"{"Labels": {}}"#,
        r#"{"Labels": []}"#,
        r#"{"Point": [1, 2]}"#,
        r#"{"Point": [1]}"#,
        r#"{"Point": [1, 2, 3]}"#,
        r#"{"Point": {"0": 1, "1": 2}}"#,
        r#"{"Transfer": {"iban": "ES12"}}"#,
        r#"{"Transfer": {"reference": "rent", "iban": "ES12"}}"#,
        r#"{"Transfer": {"iban": "ES12", "iban": "ES34"}}"#,
        r#"{"Transfer": {"iban": "ES12", "unknown": 1}}"#,
        r#"{"Transfer": {}}"#,
        r#"{"Transfer": ["ES12"]}"#,
        r#"{"Transfer": ["ES12", "rent"]}"#,
        r#"{"Transfer": ["ES12", "rent", 3]}"#,
        r#"{"Transfer": []}"#,
        r#"{"Transfer": null}"#,
        r#"{"Transfer": "ES12"}"#,
        r#""Transfer""#,
        r#""Unknown""#,
        r#"{"Unknown": null}"#,
        "0",
        "6",
        "7",
        r#"{"0": null}"#,
        "true",
        "null",
        "[]",
        r#"["Cash"]"#,
        "{}",
        r#"{"Cash": null, "Card": {"number": "4111"}}"#,
    ];

    /// An `Option` field whose element is missing from the end of a sequence is `None` to annotated. serde takes it
    /// for a sequence of the wrong length, unless the field has a default.
    const OPTIONS_MISSING_FROM_A_SEQUENCE: &[KnownDifference] = &[in_both_formats(r#"{"Transfer": ["ES12"]}"#)];

    /// annotated's buffer reads a variant's name only, where MessagePack also reads its index.
    const INDICES_AS_TAGS_IN_THE_BUFFER: &[KnownDifference] = &[in_message_pack("0")];

    /// MessagePack reads a unit struct from an empty sequence, which annotated's buffer doesn't.
    const UNIT_STRUCTS_AS_EMPTY_SEQUENCES_IN_THE_BUFFER: &[KnownDifference] = &[in_message_pack(r#"{"Marked": []}"#)];

    #[test]
    fn an_externally_tagged_enum_reads_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Payment, by_annotated::Payment>(
            "an externally tagged enum",
            BODIES,
            &[OPTIONS_MISSING_FROM_A_SEQUENCE],
        );
    }

    #[test]
    fn an_externally_tagged_enum_collecting_type_errors_reads_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Payment, collecting_at_the_root::Payment>(
            "an externally tagged enum collecting type errors",
            BODIES,
            &[
                OPTIONS_MISSING_FROM_A_SEQUENCE,
                INDICES_AS_TAGS_IN_THE_BUFFER,
                UNIT_STRUCTS_AS_EMPTY_SEQUENCES_IN_THE_BUFFER,
            ],
        );
    }

    #[test]
    fn an_externally_tagged_enum_collecting_type_errors_throughout_reads_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Payment, collecting_throughout::Payment>(
            "an externally tagged enum collecting type errors throughout",
            BODIES,
            &[
                OPTIONS_MISSING_FROM_A_SEQUENCE,
                INDICES_AS_TAGS_IN_THE_BUFFER,
                UNIT_STRUCTS_AS_EMPTY_SEQUENCES_IN_THE_BUFFER,
            ],
        );
    }
}

mod an_internally_tagged_enum {
    use super::*;

    read_by_both! {
        #[serde(tag = "type")]
        enum Payment {
            Cash,
            Card(#[validate(nested)] Card),
            Nothing(()),
            Marked(Marker),
            Labels(BTreeMap<String, u8>),
            Transfer { iban: String, reference: Option<String> },
        }

        struct Card {
            number: String,
        }
    }

    const BODIES: &[&str] = &[
        r#"{"type": "Cash"}"#,
        r#"{"type": "Cash", "unknown": 1}"#,
        r#"{"unknown": 1, "type": "Cash"}"#,
        r#"{"type": "Cash", "type": "Cash"}"#,
        r#"{"type": 0}"#,
        r#"{"type": 1, "number": "4111"}"#,
        r#"{"type": 6}"#,
        r#"{"type": -1}"#,
        r#"{"type": 0.5}"#,
        r#"{"type": true}"#,
        r#"{"type": null}"#,
        r#"{"type": {}}"#,
        r#"{"type": {"Cash": null}}"#,
        r#"{"type": []}"#,
        r#"{"type": "Unknown"}"#,
        r#"{"type": "cash"}"#,
        "{}",
        r#"{"number": "4111"}"#,
        r#"{"type": "Card", "number": "4111"}"#,
        r#"{"number": "4111", "type": "Card"}"#,
        r#"{"type": "Card"}"#,
        r#"{"type": "Card", "number": 4111}"#,
        r#"{"type": "Card", "number": "4111", "number": "5500"}"#,
        r#"{"type": "Nothing"}"#,
        r#"{"type": "Nothing", "unknown": 1}"#,
        r#"{"type": "Marked"}"#,
        r#"{"type": "Marked", "unknown": 1}"#,
        r#"{"type": "Labels", "en": 1}"#,
        r#"{"type": "Labels"}"#,
        r#"{"type": "Labels", "en": "one"}"#,
        r#"{"type": "Transfer", "iban": "ES12"}"#,
        r#"{"iban": "ES12", "reference": null, "type": "Transfer"}"#,
        r#"{"type": "Transfer", "iban": "ES12", "iban": "ES34"}"#,
        r#"{"type": "Transfer"}"#,
        r#"{"type": "Transfer", "iban": 12}"#,
        r#"["Cash"]"#,
        r#"["Cash", 1]"#,
        r#"["Card", "4111"]"#,
        r#"["Nothing"]"#,
        r#"["Marked"]"#,
        r#"["Transfer", "ES12"]"#,
        r#"["Transfer", "ES12", "rent", 3]"#,
        "[0]",
        "[]",
        r#""Cash""#,
        "null",
        "0",
    ];

    /// annotated reads a tag given as an integer as a variant's index, which `serde_json` refuses.
    const INDICES_AS_TAGS: &[KnownDifference] = &[
        in_json(r#"{"type": 0}"#),
        in_json(r#"{"type": 1, "number": "4111"}"#),
        in_json("[0]"),
    ];

    /// An `Option` field whose element is missing from the end of a sequence is `None` to annotated. serde takes it
    /// for a sequence of the wrong length, unless the field has a default.
    const OPTIONS_MISSING_FROM_A_SEQUENCE: &[KnownDifference] = &[in_both_formats(r#"["Transfer", "ES12"]"#)];

    #[test]
    fn an_internally_tagged_enum_reads_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Payment, by_annotated::Payment>(
            "an internally tagged enum",
            BODIES,
            &[INDICES_AS_TAGS, OPTIONS_MISSING_FROM_A_SEQUENCE],
        );
    }

    #[test]
    fn an_internally_tagged_enum_collecting_type_errors_reads_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Payment, collecting_at_the_root::Payment>(
            "an internally tagged enum collecting type errors",
            BODIES,
            &[INDICES_AS_TAGS, OPTIONS_MISSING_FROM_A_SEQUENCE],
        );
    }

    #[test]
    fn an_internally_tagged_enum_collecting_type_errors_throughout_reads_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Payment, collecting_throughout::Payment>(
            "an internally tagged enum collecting type errors throughout",
            BODIES,
            &[INDICES_AS_TAGS, OPTIONS_MISSING_FROM_A_SEQUENCE],
        );
    }
}

mod an_adjacently_tagged_enum {
    use super::*;

    read_by_both! {
        #[serde(tag = "type", content = "data")]
        enum Payment {
            Cash,
            Card(#[validate(nested)] Card),
            Nothing(()),
            Marked(Marker),
            Labels(BTreeMap<String, u8>),
            Point(u8, u8),
            Transfer { iban: String, reference: Option<String> },
        }

        struct Card {
            number: String,
        }
    }

    const BODIES: &[&str] = &[
        r#"{"type": "Cash"}"#,
        r#"{"type": "Cash", "data": null}"#,
        r#"{"data": null, "type": "Cash"}"#,
        r#"{"type": "Cash", "data": {}}"#,
        r#"{"type": "Cash", "unknown": 1}"#,
        r#"{"type": "Cash", "type": "Cash"}"#,
        r#"{"type": "Cash", "data": null, "data": null}"#,
        r#"{"type": 0}"#,
        r#"{"type": 0, "data": null}"#,
        r#"{"type": 1, "data": {"number": "4111"}}"#,
        r#"{"type": 7, "data": null}"#,
        r#"{"type": true}"#,
        r#"{"type": null}"#,
        r#"{"type": {"Cash": null}}"#,
        r#"{"type": {"Cash": 1}}"#,
        r#"{"type": {"Unknown": null}}"#,
        r#"{"type": []}"#,
        r#"{"type": "Unknown", "data": 1}"#,
        r#"{"data": {"number": "4111"}}"#,
        "{}",
        r#"{"type": "Card", "data": {"number": "4111"}}"#,
        r#"{"data": {"number": "4111"}, "type": "Card"}"#,
        r#"{"type": "Card"}"#,
        r#"{"type": "Card", "data": null}"#,
        r#"{"type": "Card", "data": {"number": 4111}}"#,
        r#"{"data": {"number": 4111}, "type": "Card"}"#,
        r#"{"type": "Card", "data": ["4111"]}"#,
        r#"{"type": "Nothing"}"#,
        r#"{"type": "Nothing", "data": null}"#,
        r#"{"type": "Nothing", "data": {}}"#,
        r#"{"type": "Marked"}"#,
        r#"{"type": "Marked", "data": null}"#,
        r#"{"type": "Marked", "data": []}"#,
        r#"{"type": "Labels", "data": {"en": 1}}"#,
        r#"{"type": "Point", "data": [1, 2]}"#,
        r#"{"type": "Point", "data": [1]}"#,
        r#"{"data": [1, 2], "type": "Point"}"#,
        r#"{"type": "Transfer", "data": {"iban": "ES12"}}"#,
        r#"{"data": {"iban": "ES12"}, "type": "Transfer"}"#,
        r#"{"type": "Transfer", "data": ["ES12"]}"#,
        r#"{"data": ["ES12"], "type": "Transfer"}"#,
        r#"{"type": "Transfer", "data": {"iban": "ES12", "iban": "ES34"}}"#,
        r#"{"type": "Transfer", "data": {}}"#,
        r#"{"type": "Transfer"}"#,
        r#"["Cash"]"#,
        r#"["Cash", null]"#,
        r#"["Card", {"number": "4111"}]"#,
        r#"["Transfer", {"iban": "ES12"}]"#,
        r#"["Transfer", ["ES12"]]"#,
        "[0, null]",
        r#"[{"Cash": null}, null]"#,
        "[]",
        r#""Cash""#,
        "null",
    ];

    /// annotated reads a tag given as an integer as a variant's index, which `serde_json` refuses, and refuses a tag given
    /// as a unit variant, which serde reads.
    const TAGS_READ_FROM_THE_BUFFER: &[KnownDifference] = &[
        in_json(r#"{"type": 0}"#),
        in_json(r#"{"type": 0, "data": null}"#),
        in_json(r#"{"type": 1, "data": {"number": "4111"}}"#),
        in_json("[0, null]"),
        in_both_formats(r#"{"type": {"Cash": null}}"#),
    ];

    /// annotated reads a unit variant from a sequence of its tag alone, which serde takes for a sequence of the wrong
    /// length.
    const CONTENT_MISSING_FROM_A_SEQUENCE: &[KnownDifference] = &[in_both_formats(r#"["Cash"]"#)];

    /// MessagePack reads a unit struct from an empty sequence, which annotated's buffer doesn't.
    const UNIT_STRUCTS_AS_EMPTY_SEQUENCES_IN_THE_BUFFER: &[KnownDifference] =
        &[in_message_pack(r#"{"type": "Marked", "data": []}"#)];

    #[test]
    fn an_adjacently_tagged_enum_reads_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Payment, by_annotated::Payment>(
            "an adjacently tagged enum",
            BODIES,
            &[TAGS_READ_FROM_THE_BUFFER, CONTENT_MISSING_FROM_A_SEQUENCE],
        );
    }

    #[test]
    fn an_adjacently_tagged_enum_collecting_type_errors_reads_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Payment, collecting_at_the_root::Payment>(
            "an adjacently tagged enum collecting type errors",
            BODIES,
            &[
                TAGS_READ_FROM_THE_BUFFER,
                CONTENT_MISSING_FROM_A_SEQUENCE,
                UNIT_STRUCTS_AS_EMPTY_SEQUENCES_IN_THE_BUFFER,
            ],
        );
    }

    #[test]
    fn an_adjacently_tagged_enum_collecting_type_errors_throughout_reads_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Payment, collecting_throughout::Payment>(
            "an adjacently tagged enum collecting type errors throughout",
            BODIES,
            &[
                TAGS_READ_FROM_THE_BUFFER,
                CONTENT_MISSING_FROM_A_SEQUENCE,
                UNIT_STRUCTS_AS_EMPTY_SEQUENCES_IN_THE_BUFFER,
            ],
        );
    }
}

mod an_untagged_enum {
    use super::*;

    read_by_both! {
        #[serde(untagged)]
        enum Payment {
            Transfer { iban: String, reference: Option<String> },
            Card(#[validate(nested)] Card),
            Point(u8, u8),
            Cash,
            Nothing(()),
            Marked(Marker),
            Labels(BTreeMap<String, u8>),
        }

        struct Card {
            number: String,
        }
    }

    const BODIES: &[&str] = &[
        r#"{"iban": "ES12"}"#,
        r#"{"reference": "rent", "iban": "ES12"}"#,
        r#"{"iban": "ES12", "iban": "ES34"}"#,
        r#"{"iban": 12}"#,
        r#"{"number": "4111"}"#,
        r#"{"number": 4111}"#,
        r#"{"en": 1}"#,
        r#"{"en": "one"}"#,
        "{}",
        r#"["ES12"]"#,
        r#"["ES12", "rent"]"#,
        r#"["4111"]"#,
        "[1, 2]",
        "[1]",
        "[1, 2, 3]",
        "[]",
        "null",
        r#""Cash""#,
        "1",
        "true",
    ];

    #[test]
    fn an_untagged_enum_reads_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Payment, by_annotated::Payment>(
            "an untagged enum",
            BODIES,
            &[],
        );
    }

    #[test]
    fn an_untagged_enum_collecting_type_errors_reads_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Payment, collecting_at_the_root::Payment>(
            "an untagged enum collecting type errors",
            BODIES,
            &[],
        );
    }

    #[test]
    fn an_untagged_enum_collecting_type_errors_throughout_reads_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Payment, collecting_throughout::Payment>(
            "an untagged enum collecting type errors throughout",
            BODIES,
            &[],
        );
    }
}

mod tagged_enums_in_a_struct {
    use super::*;

    read_by_both! {
        struct Order {
            #[validate(nested)]
            internal: Option<Internal>,
            #[validate(nested)]
            adjacent: Option<Adjacent>,
            #[validate(not_nested)]
            external: Option<External>,
        }

        #[serde(tag = "type")]
        enum Internal {
            Cash,
            Card { number: String },
        }

        #[serde(tag = "type", content = "data")]
        enum Adjacent {
            Cash,
            Card { number: String },
        }

        enum External {
            Cash,
            Card { number: String },
        }
    }

    const BODIES: &[&str] = &[
        r#"{"internal": {"type": "Cash"}}"#,
        r#"{"internal": {"type": "Card", "number": "4111"}}"#,
        r#"{"internal": {"type": 0}}"#,
        r#"{"internal": {"type": 1, "number": "4111"}}"#,
        r#"{"internal": {"type": true}}"#,
        r#"{"internal": ["Card", "4111"]}"#,
        r#"{"adjacent": {"type": "Cash"}}"#,
        r#"{"adjacent": {"type": "Card", "data": {"number": "4111"}}}"#,
        r#"{"adjacent": {"type": 0}}"#,
        r#"{"adjacent": {"type": {"Cash": null}}}"#,
        r#"{"adjacent": {"type": "Card", "data": ["4111"]}}"#,
        r#"{"adjacent": ["Cash", null]}"#,
        r#"{"adjacent": [0, null]}"#,
        r#"{"external": "Cash"}"#,
        r#"{"external": 0}"#,
        r#"{"external": {"Card": {"number": "4111"}}}"#,
        r#"{"external": {"Card": ["4111"]}}"#,
        r#"{"external": {"1": ["4111"]}}"#,
        "{}",
    ];

    /// annotated reads a tag given as an integer as a variant's index, which `serde_json` refuses, and refuses a tag given
    /// as a unit variant, which serde reads.
    const TAGS_READ_FROM_THE_BUFFER: &[KnownDifference] = &[
        in_json(r#"{"internal": {"type": 0}}"#),
        in_json(r#"{"internal": {"type": 1, "number": "4111"}}"#),
        in_json(r#"{"adjacent": {"type": 0}}"#),
        in_json(r#"{"adjacent": [0, null]}"#),
        in_both_formats(r#"{"adjacent": {"type": {"Cash": null}}}"#),
    ];

    /// annotated's buffer reads a variant's name only, where MessagePack also reads its index.
    const INDICES_AS_TAGS_IN_THE_BUFFER: &[KnownDifference] = &[in_message_pack(r#"{"external": 0}"#)];

    #[test]
    fn tagged_enums_in_a_struct_read_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Order, by_annotated::Order>(
            "tagged enums in a struct",
            BODIES,
            &[TAGS_READ_FROM_THE_BUFFER],
        );
    }

    #[test]
    fn tagged_enums_in_a_struct_collecting_type_errors_read_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Order, collecting_at_the_root::Order>(
            "tagged enums in a struct collecting type errors",
            BODIES,
            &[TAGS_READ_FROM_THE_BUFFER, INDICES_AS_TAGS_IN_THE_BUFFER],
        );
    }

    #[test]
    fn tagged_enums_collecting_type_errors_in_a_struct_collecting_them_read_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Order, collecting_throughout::Order>(
            "tagged enums collecting type errors in a struct collecting them",
            BODIES,
            &[TAGS_READ_FROM_THE_BUFFER, INDICES_AS_TAGS_IN_THE_BUFFER],
        );
    }
}

mod flattened_fields {
    use super::*;

    read_by_both! {
        struct Order {
            id: u8,
            #[serde(flatten)]
            #[validate(nested)]
            payment: Payment,
            #[serde(flatten)]
            #[validate(nested)]
            delivery: Option<Delivery>,
            #[serde(flatten)]
            labels: BTreeMap<String, String>,
        }

        enum Payment {
            Cash,
            Card { number: String },
        }

        struct Delivery {
            street: String,
        }
    }

    const BODIES: &[&str] = &[
        r#"{"id": 1, "Cash": null}"#,
        r#"{"id": 1, "Card": {"number": "4111"}}"#,
        r#"{"Card": {"number": "4111"}, "id": 1}"#,
        r#"{"id": 1, "Card": {"number": "4111"}, "note": "leave at the door"}"#,
        r#"{"note": "leave at the door", "id": 1, "Cash": null}"#,
        r#"{"id": 1, "Cash": null, "Card": {"number": "4111"}}"#,
        r#"{"id": 1, "Card": ["4111"]}"#,
        r#"{"id": 1, "Card": {"number": 4111}}"#,
        r#"{"id": 1, "Cash": null, "street": "Gran Vía"}"#,
        r#"{"id": 1, "Cash": null, "street": 1}"#,
        r#"{"id": 1, "Cash": null, "street": "Gran Vía", "street": "Alcalá"}"#,
        r#"{"id": 1}"#,
        r#"{"id": 1, "Unknown": null}"#,
        r#"{"Cash": null}"#,
        r#"{"id": 1, "id": 2, "Cash": null}"#,
        "[1]",
        "null",
    ];

    #[test]
    fn flattened_fields_read_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Order, by_annotated::Order>(
            "flattened fields",
            BODIES,
            &[],
        );
    }

    #[test]
    fn flattened_fields_collecting_type_errors_read_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Order, collecting_at_the_root::Order>(
            "flattened fields collecting type errors",
            BODIES,
            &[],
        );
    }

    #[test]
    fn flattened_fields_collecting_type_errors_throughout_read_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Order, collecting_throughout::Order>(
            "flattened fields collecting type errors throughout",
            BODIES,
            &[],
        );
    }
}

mod flattened_tagged_enums {
    use super::*;

    read_by_both! {
        struct Order {
            id: u8,
            #[serde(flatten)]
            #[validate(nested)]
            payment: Payment,
        }

        #[serde(tag = "type")]
        enum Payment {
            Cash,
            Card { number: String },
        }
    }

    const BODIES: &[&str] = &[
        r#"{"id": 1, "type": "Cash"}"#,
        r#"{"type": "Card", "id": 1, "number": "4111"}"#,
        r#"{"id": 1, "type": "Card", "number": "4111", "note": "leave at the door"}"#,
        r#"{"id": 1, "type": 0}"#,
        r#"{"id": 1, "type": 1, "number": "4111"}"#,
        r#"{"id": 1, "type": true}"#,
        r#"{"id": 1, "type": "Unknown"}"#,
        r#"{"id": 1}"#,
        r#"{"id": 1, "type": "Card"}"#,
    ];

    #[test]
    fn flattened_tagged_enums_read_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Order, by_annotated::Order>(
            "flattened tagged enums",
            BODIES,
            &[],
        );
    }

    #[test]
    fn flattened_tagged_enums_collecting_type_errors_read_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Order, collecting_at_the_root::Order>(
            "flattened tagged enums collecting type errors",
            BODIES,
            &[],
        );
    }

    #[test]
    fn flattened_tagged_enums_collecting_type_errors_throughout_read_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Order, collecting_throughout::Order>(
            "flattened tagged enums collecting type errors throughout",
            BODIES,
            &[],
        );
    }
}

mod options {
    use super::*;

    read_by_both! {
        #[allow(clippy::option_option)]
        struct Profile {
            nickname: Option<String>,
            age: Option<Option<u8>>,
            nothing: Option<()>,
            #[validate(nested)]
            card: Option<Card>,
            labels: Option<BTreeMap<String, u8>>,
        }

        struct Card {
            number: String,
        }
    }

    const BODIES: &[&str] = &[
        "{}",
        r#"{"nickname": null, "age": null, "nothing": null, "card": null, "labels": null}"#,
        r#"{"nickname": "Al", "age": 30, "card": {"number": "4111"}, "labels": {"en": 1}}"#,
        r#"{"nothing": {}}"#,
        r#"{"nothing": []}"#,
        r#"{"nothing": 1}"#,
        r#"{"age": "30"}"#,
        r#"{"age": [30]}"#,
        r#"{"card": {}}"#,
        r#"{"card": "4111"}"#,
        r#"{"card": ["4111"]}"#,
        r#"{"labels": []}"#,
        r#"{"labels": {"en": "one"}}"#,
        "[null, null, null, null, null]",
        "[]",
        "null",
    ];

    /// An `Option` field whose element is missing from the end of a sequence is `None` to annotated. serde takes it
    /// for a sequence of the wrong length, unless the field has a default.
    const OPTIONS_MISSING_FROM_A_SEQUENCE: &[KnownDifference] = &[in_both_formats("[]")];

    #[test]
    fn options_read_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Profile, by_annotated::Profile>(
            "options",
            BODIES,
            &[OPTIONS_MISSING_FROM_A_SEQUENCE],
        );
    }

    #[test]
    fn options_collecting_type_errors_read_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Profile, collecting_at_the_root::Profile>(
            "options collecting type errors",
            BODIES,
            &[OPTIONS_MISSING_FROM_A_SEQUENCE],
        );
    }

    #[test]
    fn options_collecting_type_errors_throughout_read_each_body_as_serde_reads_it() {
        assert_each_body_reads_as_serde_reads_it::<by_serde::Profile, collecting_throughout::Profile>(
            "options collecting type errors throughout",
            BODIES,
            &[OPTIONS_MISSING_FROM_A_SEQUENCE],
        );
    }
}
