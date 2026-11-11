use std::fmt;

use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Unexpected, Visitor};

use crate::deserialize_containers::room_for;

/// Any well-formed value, as a self-describing format presents it, kept to be read again as the type it's meant to
/// be. Strings and bytes the format lends are borrowed, not copied.
#[derive(Debug, PartialEq)]
pub(crate) enum Content<'de> {
    Bool(bool),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    F32(f32),
    F64(f64),
    Char(char),
    String(String),
    Str(&'de str),
    ByteBuf(Vec<u8>),
    Bytes(&'de [u8]),
    None,
    Some(Box<Content<'de>>),
    Unit,
    Newtype(Box<Content<'de>>),
    Seq(Vec<Content<'de>>),
    Map(Vec<(Content<'de>, Content<'de>)>),
}

/// The kind of a value, as a violation names what came where something else was expected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ValueKind {
    Boolean,
    Integer,
    Float,
    Character,
    String,
    Bytes,
    Null,
    Sequence,
    Map,
}

impl ValueKind {
    pub(crate) const fn name(self) -> &'static str {
        match self {
            Self::Boolean => "boolean",
            Self::Integer => "integer",
            Self::Float => "float",
            Self::Character => "character",
            Self::String => "string",
            Self::Bytes => "bytes",
            Self::Null => "null",
            Self::Sequence => "sequence",
            Self::Map => "map",
        }
    }

    /// The kind serde names as unexpected, when it names one: a newtype, an enum's variant or anything `Other` has
    /// no kind of its own.
    pub(crate) const fn of_unexpected(unexpected: &Unexpected<'_>) -> Option<Self> {
        match unexpected {
            Unexpected::Bool(_) => Some(Self::Boolean),
            Unexpected::Unsigned(_) | Unexpected::Signed(_) => Some(Self::Integer),
            Unexpected::Float(_) => Some(Self::Float),
            Unexpected::Char(_) => Some(Self::Character),
            Unexpected::Str(_) => Some(Self::String),
            Unexpected::Bytes(_) => Some(Self::Bytes),
            Unexpected::Unit | Unexpected::Option => Some(Self::Null),
            Unexpected::Seq => Some(Self::Sequence),
            Unexpected::Map => Some(Self::Map),
            Unexpected::NewtypeStruct
            | Unexpected::Enum
            | Unexpected::UnitVariant
            | Unexpected::NewtypeVariant
            | Unexpected::TupleVariant
            | Unexpected::StructVariant
            | Unexpected::Other(_) => None,
        }
    }
}

impl Content<'_> {
    /// The kind of this value, seeing through an option or a newtype to what it holds.
    pub(crate) fn kind(&self) -> ValueKind {
        match self {
            Self::Bool(_) => ValueKind::Boolean,
            Self::U8(_)
            | Self::U16(_)
            | Self::U32(_)
            | Self::U64(_)
            | Self::U128(_)
            | Self::I8(_)
            | Self::I16(_)
            | Self::I32(_)
            | Self::I64(_)
            | Self::I128(_) => ValueKind::Integer,
            Self::F32(_) | Self::F64(_) => ValueKind::Float,
            Self::Char(_) => ValueKind::Character,
            Self::String(_) | Self::Str(_) => ValueKind::String,
            Self::ByteBuf(_) | Self::Bytes(_) => ValueKind::Bytes,
            Self::None | Self::Unit => ValueKind::Null,
            Self::Some(inner) | Self::Newtype(inner) => inner.kind(),
            Self::Seq(_) => ValueKind::Sequence,
            Self::Map(_) => ValueKind::Map,
        }
    }

    /// This value as serde names it in a type error, seeing through an option or a newtype to what it holds.
    pub(crate) fn unexpected(&self) -> Unexpected<'_> {
        match self {
            Self::Bool(boolean) => Unexpected::Bool(*boolean),
            Self::U8(integer) => Unexpected::Unsigned(u64::from(*integer)),
            Self::U16(integer) => Unexpected::Unsigned(u64::from(*integer)),
            Self::U32(integer) => Unexpected::Unsigned(u64::from(*integer)),
            Self::U64(integer) => Unexpected::Unsigned(*integer),
            Self::U128(integer) => u64::try_from(*integer).map_or(Unexpected::Other("integer"), Unexpected::Unsigned),
            Self::I8(integer) => Unexpected::Signed(i64::from(*integer)),
            Self::I16(integer) => Unexpected::Signed(i64::from(*integer)),
            Self::I32(integer) => Unexpected::Signed(i64::from(*integer)),
            Self::I64(integer) => Unexpected::Signed(*integer),
            Self::I128(integer) => i64::try_from(*integer).map_or(Unexpected::Other("integer"), Unexpected::Signed),
            Self::F32(float) => Unexpected::Float(f64::from(*float)),
            Self::F64(float) => Unexpected::Float(*float),
            Self::Char(character) => Unexpected::Char(*character),
            Self::String(string) => Unexpected::Str(string),
            Self::Str(string) => Unexpected::Str(string),
            Self::ByteBuf(bytes) => Unexpected::Bytes(bytes),
            Self::Bytes(bytes) => Unexpected::Bytes(bytes),
            Self::None | Self::Unit => Unexpected::Unit,
            Self::Some(inner) | Self::Newtype(inner) => inner.unexpected(),
            Self::Seq(_) => Unexpected::Seq,
            Self::Map(_) => Unexpected::Map,
        }
    }
}

impl<'de> Deserialize<'de> for Content<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(ContentVisitor)
    }
}

struct ContentVisitor;

impl<'de> Visitor<'de> for ContentVisitor {
    type Value = Content<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any value")
    }

    fn visit_bool<E>(self, boolean: bool) -> Result<Self::Value, E> {
        Ok(Content::Bool(boolean))
    }

    fn visit_u8<E>(self, integer: u8) -> Result<Self::Value, E> {
        Ok(Content::U8(integer))
    }

    fn visit_u16<E>(self, integer: u16) -> Result<Self::Value, E> {
        Ok(Content::U16(integer))
    }

    fn visit_u32<E>(self, integer: u32) -> Result<Self::Value, E> {
        Ok(Content::U32(integer))
    }

    fn visit_u64<E>(self, integer: u64) -> Result<Self::Value, E> {
        Ok(Content::U64(integer))
    }

    fn visit_u128<E>(self, integer: u128) -> Result<Self::Value, E> {
        Ok(Content::U128(integer))
    }

    fn visit_i8<E>(self, integer: i8) -> Result<Self::Value, E> {
        Ok(Content::I8(integer))
    }

    fn visit_i16<E>(self, integer: i16) -> Result<Self::Value, E> {
        Ok(Content::I16(integer))
    }

    fn visit_i32<E>(self, integer: i32) -> Result<Self::Value, E> {
        Ok(Content::I32(integer))
    }

    fn visit_i64<E>(self, integer: i64) -> Result<Self::Value, E> {
        Ok(Content::I64(integer))
    }

    fn visit_i128<E>(self, integer: i128) -> Result<Self::Value, E> {
        Ok(Content::I128(integer))
    }

    fn visit_f32<E>(self, float: f32) -> Result<Self::Value, E> {
        Ok(Content::F32(float))
    }

    fn visit_f64<E>(self, float: f64) -> Result<Self::Value, E> {
        Ok(Content::F64(float))
    }

    fn visit_char<E>(self, character: char) -> Result<Self::Value, E> {
        Ok(Content::Char(character))
    }

    fn visit_str<E>(self, string: &str) -> Result<Self::Value, E> {
        Ok(Content::String(string.to_owned()))
    }

    fn visit_borrowed_str<E>(self, string: &'de str) -> Result<Self::Value, E> {
        Ok(Content::Str(string))
    }

    fn visit_string<E>(self, string: String) -> Result<Self::Value, E> {
        Ok(Content::String(string))
    }

    fn visit_bytes<E>(self, bytes: &[u8]) -> Result<Self::Value, E> {
        Ok(Content::ByteBuf(bytes.to_vec()))
    }

    fn visit_borrowed_bytes<E>(self, bytes: &'de [u8]) -> Result<Self::Value, E> {
        Ok(Content::Bytes(bytes))
    }

    fn visit_byte_buf<E>(self, bytes: Vec<u8>) -> Result<Self::Value, E> {
        Ok(Content::ByteBuf(bytes))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(Content::None)
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        let inner = Content::deserialize(deserializer)?;
        Ok(Content::Some(Box::new(inner)))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(Content::Unit)
    }

    fn visit_newtype_struct<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
        let inner = Content::deserialize(deserializer)?;
        Ok(Content::Newtype(Box::new(inner)))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut elements = Vec::with_capacity(room_for::<Content>(seq.size_hint()));
        while let Some(element) = seq.next_element()? {
            elements.push(element);
        }
        Ok(Content::Seq(elements))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut entries = Vec::with_capacity(room_for::<(Content, Content)>(map.size_hint()));
        while let Some(entry) = map.next_entry()? {
            entries.push(entry);
        }
        Ok(Content::Map(entries))
    }
}

#[cfg(test)]
mod tests {
    use serde::de::value::{
        BorrowedBytesDeserializer, BytesDeserializer, EnumAccessDeserializer, Error, StrDeserializer,
    };
    use serde::de::{Error as _, IntoDeserializer};
    use serde_test::{Token, assert_de_tokens};

    use super::*;

    fn content_of<'de, D: IntoDeserializer<'de, Error>>(value: D) -> Result<Content<'de>, Error> {
        Content::deserialize(value.into_deserializer())
    }

    #[test]
    fn a_string_the_format_lends_is_borrowed() {
        assert_eq!(
            serde_json::from_str::<Content>(r#""Lucía""#).ok(),
            Some(Content::Str("Lucía"))
        );
    }

    #[test]
    fn a_string_the_format_copies_is_owned() {
        assert_eq!(
            serde_json::from_str::<Content>(r#""Gran Vía\n1""#).ok(),
            Some(Content::String("Gran Vía\n1".to_owned()))
        );
    }

    #[test]
    fn json_null_is_unit() {
        assert_eq!(serde_json::from_str::<Content>("null").ok(), Some(Content::Unit));
    }

    #[test]
    fn a_map_keeps_its_entries_in_the_order_they_came() {
        let json = r#"{ "name": "Lucía", "age": 30, "balance": -12, "ratio": 0.5, "isAdmin": false }"#;

        assert_eq!(
            serde_json::from_str::<Content>(json).ok(),
            Some(Content::Map(vec![
                (Content::Str("name"), Content::Str("Lucía")),
                (Content::Str("age"), Content::U64(30)),
                (Content::Str("balance"), Content::I64(-12)),
                (Content::Str("ratio"), Content::F64(0.5)),
                (Content::Str("isAdmin"), Content::Bool(false)),
            ]))
        );
    }

    #[test]
    fn a_sequence_keeps_its_elements_in_order() {
        assert_eq!(
            serde_json::from_str::<Content>(r#"[2, ["a"], {}]"#).ok(),
            Some(Content::Seq(vec![
                Content::U64(2),
                Content::Seq(vec![Content::Str("a")]),
                Content::Map(Vec::new()),
            ]))
        );
    }

    #[test]
    fn every_integer_up_to_64_bits_keeps_its_width() {
        assert_de_tokens(
            &Content::Seq(vec![
                Content::U8(8),
                Content::U16(16),
                Content::U32(32),
                Content::U64(64),
                Content::I8(-8),
                Content::I16(-16),
                Content::I32(-32),
                Content::I64(-64),
            ]),
            &[
                Token::Seq { len: Some(8) },
                Token::U8(8),
                Token::U16(16),
                Token::U32(32),
                Token::U64(64),
                Token::I8(-8),
                Token::I16(-16),
                Token::I32(-32),
                Token::I64(-64),
                Token::SeqEnd,
            ],
        );
    }

    #[test]
    fn an_unsigned_128_bit_integer_is_kept_whole() {
        assert_eq!(content_of(u128::MAX), Ok(Content::U128(u128::MAX)));
    }

    #[test]
    fn a_signed_128_bit_integer_is_kept_whole() {
        assert_eq!(content_of(i128::MIN), Ok(Content::I128(i128::MIN)));
    }

    #[test]
    fn floats_keep_their_width() {
        assert_de_tokens(
            &Content::Seq(vec![Content::F32(0.5), Content::F64(0.25)]),
            &[
                Token::Seq { len: Some(2) },
                Token::F32(0.5),
                Token::F64(0.25),
                Token::SeqEnd,
            ],
        );
    }

    #[test]
    fn a_character_is_kept() {
        assert_eq!(content_of('ñ'), Ok(Content::Char('ñ')));
    }

    #[test]
    fn strings_are_borrowed_only_when_lent() {
        assert_de_tokens(
            &Content::Seq(vec![
                Content::Str("lent"),
                Content::String("copied".to_owned()),
                Content::String("given".to_owned()),
            ]),
            &[
                Token::Seq { len: Some(3) },
                Token::BorrowedStr("lent"),
                Token::Str("copied"),
                Token::String("given"),
                Token::SeqEnd,
            ],
        );
    }

    #[test]
    fn bytes_the_format_lends_are_borrowed() {
        assert_eq!(
            Content::deserialize(BorrowedBytesDeserializer::<Error>::new(b"\x01\x02")),
            Ok(Content::Bytes(b"\x01\x02"))
        );
    }

    #[test]
    fn bytes_the_format_copies_are_owned() {
        assert_eq!(
            Content::deserialize(BytesDeserializer::<Error>::new(b"\x01\x02")),
            Ok(Content::ByteBuf(vec![1, 2]))
        );
    }

    #[test]
    fn bytes_the_format_gives_are_kept() {
        assert_de_tokens(&Content::ByteBuf(vec![1, 2]), &[Token::ByteBuf(b"\x01\x02")]);
    }

    #[test]
    fn options_keep_what_they_hold() {
        assert_de_tokens(
            &Content::Seq(vec![Content::None, Content::Some(Box::new(Content::U8(1)))]),
            &[
                Token::Seq { len: Some(2) },
                Token::None,
                Token::Some,
                Token::U8(1),
                Token::SeqEnd,
            ],
        );
    }

    #[test]
    fn unit_is_kept() {
        assert_eq!(content_of(()), Ok(Content::Unit));
    }

    #[test]
    fn a_newtype_struct_keeps_what_it_holds() {
        assert_de_tokens(
            &Content::Newtype(Box::new(Content::U32(5))),
            &[Token::NewtypeStruct { name: "Meters" }, Token::U32(5)],
        );
    }

    #[test]
    fn an_enum_the_format_reads_in_its_own_syntax_is_refused() {
        let enum_value = EnumAccessDeserializer::new(StrDeserializer::<Error>::new("Active"));

        assert_eq!(
            Content::deserialize(enum_value),
            Err(Error::custom("invalid type: enum, expected any value"))
        );
    }

    #[test]
    fn every_kind_of_value_is_named() {
        assert_eq!(
            [
                ValueKind::Boolean.name(),
                ValueKind::Integer.name(),
                ValueKind::Float.name(),
                ValueKind::Character.name(),
                ValueKind::String.name(),
                ValueKind::Bytes.name(),
                ValueKind::Null.name(),
                ValueKind::Sequence.name(),
                ValueKind::Map.name(),
            ],
            [
                "boolean",
                "integer",
                "float",
                "character",
                "string",
                "bytes",
                "null",
                "sequence",
                "map"
            ]
        );
    }

    #[test]
    fn the_kind_of_a_value_is_what_a_format_writes() {
        assert_eq!(
            [
                Content::Bool(true).kind(),
                Content::U8(1).kind(),
                Content::I128(-1).kind(),
                Content::F32(0.5).kind(),
                Content::Char('a').kind(),
                Content::Str("a").kind(),
                Content::String("a".to_owned()).kind(),
                Content::Bytes(b"a").kind(),
                Content::ByteBuf(vec![1]).kind(),
                Content::Unit.kind(),
                Content::None.kind(),
                Content::Seq(Vec::new()).kind(),
                Content::Map(Vec::new()).kind(),
            ],
            [
                ValueKind::Boolean,
                ValueKind::Integer,
                ValueKind::Integer,
                ValueKind::Float,
                ValueKind::Character,
                ValueKind::String,
                ValueKind::String,
                ValueKind::Bytes,
                ValueKind::Bytes,
                ValueKind::Null,
                ValueKind::Null,
                ValueKind::Sequence,
                ValueKind::Map,
            ]
        );
    }

    #[test]
    fn the_kind_of_an_option_or_a_newtype_is_the_kind_of_what_it_holds() {
        assert_eq!(
            [
                Content::Some(Box::new(Content::Str("a"))).kind(),
                Content::Newtype(Box::new(Content::U8(1))).kind(),
            ],
            [ValueKind::String, ValueKind::Integer]
        );
    }

    #[test]
    fn a_value_is_unexpected_as_serde_names_it() {
        assert_eq!(
            [
                Content::Bool(true).unexpected(),
                Content::U16(1).unexpected(),
                Content::I8(-1).unexpected(),
                Content::F32(0.5).unexpected(),
                Content::Char('a').unexpected(),
                Content::String("a".to_owned()).unexpected(),
                Content::Bytes(b"a").unexpected(),
                Content::Unit.unexpected(),
                Content::None.unexpected(),
                Content::Seq(Vec::new()).unexpected(),
                Content::Map(Vec::new()).unexpected(),
            ],
            [
                Unexpected::Bool(true),
                Unexpected::Unsigned(1),
                Unexpected::Signed(-1),
                Unexpected::Float(0.5),
                Unexpected::Char('a'),
                Unexpected::Str("a"),
                Unexpected::Bytes(b"a"),
                Unexpected::Unit,
                Unexpected::Unit,
                Unexpected::Seq,
                Unexpected::Map,
            ]
        );
    }

    #[test]
    fn a_128_bit_integer_that_fits_in_64_bits_is_unexpected_by_its_value() {
        assert_eq!(
            [Content::U128(1).unexpected(), Content::I128(-1).unexpected()],
            [Unexpected::Unsigned(1), Unexpected::Signed(-1)]
        );
    }

    #[test]
    fn a_128_bit_integer_beyond_64_bits_is_unexpected_as_an_integer() {
        assert_eq!(
            [
                Content::U128(u128::MAX).unexpected(),
                Content::I128(i128::MIN).unexpected()
            ],
            [Unexpected::Other("integer"), Unexpected::Other("integer")]
        );
    }

    #[test]
    fn an_option_or_a_newtype_is_unexpected_as_what_it_holds() {
        assert_eq!(
            [
                Content::Some(Box::new(Content::Str("a"))).unexpected(),
                Content::Newtype(Box::new(Content::U8(1))).unexpected(),
            ],
            [Unexpected::Str("a"), Unexpected::Unsigned(1)]
        );
    }

    #[test]
    fn what_serde_names_unexpected_is_known_by_its_kind() {
        assert_eq!(
            [
                ValueKind::of_unexpected(&Unexpected::Bool(true)),
                ValueKind::of_unexpected(&Unexpected::Unsigned(1)),
                ValueKind::of_unexpected(&Unexpected::Signed(-1)),
                ValueKind::of_unexpected(&Unexpected::Float(0.5)),
                ValueKind::of_unexpected(&Unexpected::Char('a')),
                ValueKind::of_unexpected(&Unexpected::Str("a")),
                ValueKind::of_unexpected(&Unexpected::Bytes(b"a")),
                ValueKind::of_unexpected(&Unexpected::Unit),
                ValueKind::of_unexpected(&Unexpected::Option),
                ValueKind::of_unexpected(&Unexpected::Seq),
                ValueKind::of_unexpected(&Unexpected::Map),
            ],
            [
                Some(ValueKind::Boolean),
                Some(ValueKind::Integer),
                Some(ValueKind::Integer),
                Some(ValueKind::Float),
                Some(ValueKind::Character),
                Some(ValueKind::String),
                Some(ValueKind::Bytes),
                Some(ValueKind::Null),
                Some(ValueKind::Null),
                Some(ValueKind::Sequence),
                Some(ValueKind::Map),
            ]
        );
    }

    #[test]
    fn what_serde_names_unexpected_without_a_kind_of_its_own_has_none() {
        assert_eq!(
            [
                ValueKind::of_unexpected(&Unexpected::NewtypeStruct),
                ValueKind::of_unexpected(&Unexpected::Enum),
                ValueKind::of_unexpected(&Unexpected::UnitVariant),
                ValueKind::of_unexpected(&Unexpected::NewtypeVariant),
                ValueKind::of_unexpected(&Unexpected::TupleVariant),
                ValueKind::of_unexpected(&Unexpected::StructVariant),
                ValueKind::of_unexpected(&Unexpected::Other("integer `1` as u128")),
            ],
            [None, None, None, None, None, None, None]
        );
    }
}
