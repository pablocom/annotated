use std::error::Error;
use std::fmt::{self, Display};
use std::marker::PhantomData;

use serde::de::value::{
    BorrowedStrDeserializer, MapAccessDeserializer, MapDeserializer, SeqDeserializer, StringDeserializer,
};
use serde::de::{self, Deserializer, Expected, IntoDeserializer, Unexpected, Visitor};

use crate::content::{Content, ValueKind};

/// Reads a [`Content`] again, as the type it's meant to be, accepting what a self-describing format accepts for it.
pub(crate) struct ContentDeserializer<'de, E> {
    content: Content<'de>,
    is_human_readable: bool,
    error: PhantomData<E>,
}

impl<'de, E> ContentDeserializer<'de, E> {
    /// Reads `content`, answering `is_human_readable` as the format it came from does.
    pub(crate) const fn new(content: Content<'de>, is_human_readable: bool) -> Self {
        Self {
            content,
            is_human_readable,
            error: PhantomData,
        }
    }
}

impl<'de, E: de::Error> ContentDeserializer<'de, E> {
    #[cold]
    fn invalid_type(&self, expected: &dyn Expected) -> E {
        E::invalid_type(self.content.unexpected(), expected)
    }

    fn deserialize_integer<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.content {
            Content::U8(integer) => visitor.visit_u8(integer),
            Content::U16(integer) => visitor.visit_u16(integer),
            Content::U32(integer) => visitor.visit_u32(integer),
            Content::U64(integer) => visitor.visit_u64(integer),
            Content::U128(integer) => visit_u128(visitor, integer),
            Content::I8(integer) => visitor.visit_i8(integer),
            Content::I16(integer) => visitor.visit_i16(integer),
            Content::I32(integer) => visitor.visit_i32(integer),
            Content::I64(integer) => visitor.visit_i64(integer),
            Content::I128(integer) => visit_i128(visitor, integer),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_float<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.content {
            Content::F32(float) => visitor.visit_f32(float),
            Content::F64(float) => visitor.visit_f64(float),
            _ => self.deserialize_integer(visitor),
        }
    }

    fn visit_elements<V: Visitor<'de>>(
        elements: Vec<Content<'de>>,
        is_human_readable: bool,
        visitor: V,
    ) -> Result<V::Value, E> {
        let mut elements = SeqDeserializer::new(
            elements
                .into_iter()
                .map(|element| Self::new(element, is_human_readable)),
        );
        let value = visitor.visit_seq(&mut elements)?;
        elements.end()?;
        Ok(value)
    }

    fn visit_entries<V: Visitor<'de>>(
        entries: Vec<(Content<'de>, Content<'de>)>,
        is_human_readable: bool,
        visitor: V,
    ) -> Result<V::Value, E> {
        let mut entries = Self::entries(entries, is_human_readable);
        let value = visitor.visit_map(&mut entries)?;
        entries.end()?;
        Ok(value)
    }

    fn entries(
        entries: Vec<(Content<'de>, Content<'de>)>,
        is_human_readable: bool,
    ) -> MapDeserializer<'de, impl Iterator<Item = (MapKeyDeserializer<'de, E>, Self)>, E> {
        MapDeserializer::new(entries.into_iter().map(move |(key, value)| {
            let key = MapKeyDeserializer {
                key: Self::new(key, is_human_readable),
            };
            (key, Self::new(value, is_human_readable))
        }))
    }
}

// serde's visitors for integers narrower than 128 bits take every integer of up to 64 bits, but refuse a 128-bit one
// whatever its value.
fn visit_u128<'de, V: Visitor<'de>, E: de::Error>(visitor: V, integer: u128) -> Result<V::Value, E> {
    match u64::try_from(integer) {
        Ok(integer) => visitor.visit_u64(integer),
        Err(_) => visitor.visit_u128(integer),
    }
}

fn visit_i128<'de, V: Visitor<'de>, E: de::Error>(visitor: V, integer: i128) -> Result<V::Value, E> {
    match i64::try_from(integer) {
        Ok(integer) => visitor.visit_i64(integer),
        Err(_) => visitor.visit_i128(integer),
    }
}

impl<'de, E: de::Error> Deserializer<'de> for ContentDeserializer<'de, E> {
    type Error = E;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.content {
            Content::Bool(boolean) => visitor.visit_bool(boolean),
            Content::U8(_)
            | Content::U16(_)
            | Content::U32(_)
            | Content::U64(_)
            | Content::U128(_)
            | Content::I8(_)
            | Content::I16(_)
            | Content::I32(_)
            | Content::I64(_)
            | Content::I128(_) => self.deserialize_integer(visitor),
            Content::F32(float) => visitor.visit_f32(float),
            Content::F64(float) => visitor.visit_f64(float),
            Content::Char(character) => visitor.visit_char(character),
            Content::String(string) => visitor.visit_string(string),
            Content::Str(string) => visitor.visit_borrowed_str(string),
            Content::ByteBuf(bytes) => visitor.visit_byte_buf(bytes),
            Content::Bytes(bytes) => visitor.visit_borrowed_bytes(bytes),
            Content::None => visitor.visit_none(),
            Content::Some(inner) => visitor.visit_some(Self::new(*inner, self.is_human_readable)),
            Content::Unit => visitor.visit_unit(),
            Content::Newtype(inner) => visitor.visit_newtype_struct(Self::new(*inner, self.is_human_readable)),
            Content::Seq(elements) => Self::visit_elements(elements, self.is_human_readable, visitor),
            Content::Map(entries) => Self::visit_entries(entries, self.is_human_readable, visitor),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.content {
            Content::Bool(boolean) => visitor.visit_bool(boolean),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_i8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.deserialize_integer(visitor)
    }

    fn deserialize_i16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.deserialize_integer(visitor)
    }

    fn deserialize_i32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.deserialize_integer(visitor)
    }

    fn deserialize_i64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.deserialize_integer(visitor)
    }

    fn deserialize_i128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.deserialize_integer(visitor)
    }

    fn deserialize_u8<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.deserialize_integer(visitor)
    }

    fn deserialize_u16<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.deserialize_integer(visitor)
    }

    fn deserialize_u32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.deserialize_integer(visitor)
    }

    fn deserialize_u64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.deserialize_integer(visitor)
    }

    fn deserialize_u128<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.deserialize_integer(visitor)
    }

    fn deserialize_f32<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.deserialize_float(visitor)
    }

    fn deserialize_f64<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.deserialize_float(visitor)
    }

    fn deserialize_char<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.content {
            Content::Char(character) => visitor.visit_char(character),
            Content::String(string) => visitor.visit_string(string),
            Content::Str(string) => visitor.visit_borrowed_str(string),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.content {
            Content::String(string) => visitor.visit_string(string),
            Content::Str(string) => visitor.visit_borrowed_str(string),
            Content::ByteBuf(bytes) => visitor.visit_byte_buf(bytes),
            Content::Bytes(bytes) => visitor.visit_borrowed_bytes(bytes),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.content {
            Content::ByteBuf(bytes) => visitor.visit_byte_buf(bytes),
            Content::Bytes(bytes) => visitor.visit_borrowed_bytes(bytes),
            Content::String(string) => visitor.visit_byte_buf(string.into_bytes()),
            Content::Str(string) => visitor.visit_borrowed_bytes(string.as_bytes()),
            Content::Seq(elements) => Self::visit_elements(elements, self.is_human_readable, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.content {
            Content::None | Content::Unit => visitor.visit_none(),
            Content::Some(inner) => visitor.visit_some(Self::new(*inner, self.is_human_readable)),
            _ => visitor.visit_some(self),
        }
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.content {
            Content::Unit => visitor.visit_unit(),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value, E> {
        self.deserialize_unit(visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value, E> {
        match self.content {
            Content::Newtype(inner) => visitor.visit_newtype_struct(Self::new(*inner, self.is_human_readable)),
            _ => visitor.visit_newtype_struct(self),
        }
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.content {
            Content::Seq(elements) => Self::visit_elements(elements, self.is_human_readable, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_tuple<V: Visitor<'de>>(self, _length: usize, visitor: V) -> Result<V::Value, E> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_tuple_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _length: usize,
        visitor: V,
    ) -> Result<V::Value, E> {
        self.deserialize_seq(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.content {
            Content::Map(entries) => Self::visit_entries(entries, self.is_human_readable, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, E> {
        match self.content {
            Content::Seq(elements) => Self::visit_elements(elements, self.is_human_readable, visitor),
            _ => self.deserialize_map(visitor),
        }
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, E> {
        match self.content {
            Content::Str(variant) => visitor.visit_enum(BorrowedStrDeserializer::new(variant)),
            Content::String(variant) => visitor.visit_enum(StringDeserializer::new(variant)),
            Content::Map(entries) if entries.len() == 1 => visitor.visit_enum(MapAccessDeserializer::new(
                Self::entries(entries, self.is_human_readable),
            )),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_identifier<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.content {
            Content::String(_) | Content::Str(_) | Content::ByteBuf(_) | Content::Bytes(_) => {
                self.deserialize_string(visitor)
            }
            _ => self.deserialize_integer(visitor),
        }
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        visitor.visit_unit()
    }

    fn is_human_readable(&self) -> bool {
        self.is_human_readable
    }
}

impl<'de, E: de::Error> IntoDeserializer<'de, E> for ContentDeserializer<'de, E> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self {
        self
    }
}

/// Reads a map's key, taking an integer or a boolean written as a string, as JSON writes every key and `serde_json`
/// reads it.
struct MapKeyDeserializer<'de, E> {
    key: ContentDeserializer<'de, E>,
}

impl<'de, E> MapKeyDeserializer<'de, E> {
    fn written_as(self, read_from_text: fn(&str) -> Option<Content<'de>>) -> ContentDeserializer<'de, E> {
        let ContentDeserializer {
            content,
            is_human_readable,
            ..
        } = self.key;
        let read = match &content {
            Content::Str(text) => read_from_text(text),
            Content::String(text) => read_from_text(text),
            _ => None,
        };
        ContentDeserializer::new(read.unwrap_or(content), is_human_readable)
    }
}

fn integer_written_in<'de>(text: &str) -> Option<Content<'de>> {
    if !text.starts_with(|first: char| first.is_ascii_digit() || first == '-') {
        return None;
    }
    text.parse()
        .map(Content::U64)
        .or_else(|_| text.parse().map(Content::I64))
        .or_else(|_| text.parse().map(Content::U128))
        .or_else(|_| text.parse().map(Content::I128))
        .ok()
}

fn boolean_written_in<'de>(text: &str) -> Option<Content<'de>> {
    match text {
        "true" => Some(Content::Bool(true)),
        "false" => Some(Content::Bool(false)),
        _ => None,
    }
}

macro_rules! read_as_written {
    ($($method:ident: $read_from_text:ident),* $(,)?) => {
        $(
            fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
                self.written_as($read_from_text).$method(visitor)
            }
        )*
    };
}

macro_rules! read_as_it_came {
    ($($method:ident($($argument:ident: $argument_type:ty),*)),* $(,)?) => {
        $(
            fn $method<V: Visitor<'de>>(self, $($argument: $argument_type,)* visitor: V) -> Result<V::Value, E> {
                self.key.$method($($argument,)* visitor)
            }
        )*
    };
}

impl<'de, E: de::Error> Deserializer<'de> for MapKeyDeserializer<'de, E> {
    type Error = E;

    read_as_written! {
        deserialize_i8: integer_written_in,
        deserialize_i16: integer_written_in,
        deserialize_i32: integer_written_in,
        deserialize_i64: integer_written_in,
        deserialize_i128: integer_written_in,
        deserialize_u8: integer_written_in,
        deserialize_u16: integer_written_in,
        deserialize_u32: integer_written_in,
        deserialize_u64: integer_written_in,
        deserialize_u128: integer_written_in,
        deserialize_bool: boolean_written_in,
    }

    read_as_it_came! {
        deserialize_any(),
        deserialize_f32(),
        deserialize_f64(),
        deserialize_char(),
        deserialize_str(),
        deserialize_string(),
        deserialize_bytes(),
        deserialize_byte_buf(),
        deserialize_unit(),
        deserialize_unit_struct(name: &'static str),
        deserialize_seq(),
        deserialize_tuple(length: usize),
        deserialize_tuple_struct(name: &'static str, length: usize),
        deserialize_map(),
        deserialize_struct(name: &'static str, fields: &'static [&'static str]),
        deserialize_enum(name: &'static str, variants: &'static [&'static str]),
        deserialize_identifier(),
        deserialize_ignored_any(),
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        visitor.visit_some(self)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value, E> {
        visitor.visit_newtype_struct(self)
    }

    fn is_human_readable(&self) -> bool {
        self.key.is_human_readable
    }
}

impl<'de, E: de::Error> IntoDeserializer<'de, E> for MapKeyDeserializer<'de, E> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self {
        self
    }
}

/// Why a [`Content`] couldn't be read as a type: what the type expects and the kind of value found, when serde
/// says. It never holds the value, nor a message that might quote it.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct WrongType {
    pub(crate) expected: Option<String>,
    pub(crate) found: Option<ValueKind>,
}

impl WrongType {
    fn of(unexpected: &Unexpected<'_>, expected: &dyn Expected) -> Self {
        Self {
            expected: Some(expected.to_string()),
            found: ValueKind::of_unexpected(unexpected),
        }
    }
}

impl de::Error for WrongType {
    fn custom<T: Display>(_message: T) -> Self {
        Self {
            expected: None,
            found: None,
        }
    }

    fn invalid_type(unexpected: Unexpected<'_>, expected: &dyn Expected) -> Self {
        Self::of(&unexpected, expected)
    }

    fn invalid_value(unexpected: Unexpected<'_>, expected: &dyn Expected) -> Self {
        Self::of(&unexpected, expected)
    }

    fn invalid_length(_length: usize, expected: &dyn Expected) -> Self {
        Self {
            expected: Some(expected.to_string()),
            found: None,
        }
    }
}

impl Display for WrongType {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid type")?;
        if let Some(found) = self.found {
            write!(formatter, ": {}", found.name())?;
        }
        if let Some(expected) = &self.expected {
            write!(formatter, ", expected {expected}")?;
        }
        Ok(())
    }
}

impl Error for WrongType {}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde::Deserialize;
    use serde::de::{Error as _, IgnoredAny, Unexpected};

    use super::*;

    fn read<'de, T: Deserialize<'de>>(content: Content<'de>) -> Result<T, WrongType> {
        T::deserialize(ContentDeserializer::new(content, true))
    }

    fn wrong_type(expected: &str, found: ValueKind) -> WrongType {
        WrongType {
            expected: Some(expected.to_owned()),
            found: Some(found),
        }
    }

    #[derive(Deserialize, Debug, PartialEq)]
    struct Meters(u32);

    #[derive(Deserialize, Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct ShelfId(u32);

    #[derive(Deserialize, Debug, PartialEq)]
    struct Nothing;

    #[derive(Deserialize, Debug, PartialEq)]
    struct Pair(u8, u8);

    #[derive(Deserialize, Debug, PartialEq)]
    struct Point {
        x: i8,
        y: i8,
    }

    #[derive(Deserialize, Debug, PartialEq)]
    enum Status {
        Active,
        Suspended(u8),
        Moved { to: String },
    }

    #[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
    struct HumanReadability(bool);

    impl<'de> Deserialize<'de> for HumanReadability {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            let is_human_readable = deserializer.is_human_readable();
            IgnoredAny::deserialize(deserializer)?;
            Ok(Self(is_human_readable))
        }
    }

    #[derive(Debug, PartialEq)]
    struct Bytes(Vec<u8>);

    impl<'de> Deserialize<'de> for Bytes {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            struct BytesVisitor;

            impl<'de> Visitor<'de> for BytesVisitor {
                type Value = Bytes;

                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("bytes")
                }

                fn visit_bytes<E>(self, bytes: &[u8]) -> Result<Bytes, E> {
                    Ok(Bytes(bytes.to_vec()))
                }

                fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Bytes, A::Error> {
                    let mut bytes = Vec::new();
                    while let Some(byte) = seq.next_element()? {
                        bytes.push(byte);
                    }
                    Ok(Bytes(bytes))
                }
            }

            deserializer.deserialize_byte_buf(BytesVisitor)
        }
    }

    #[test]
    fn an_integer_is_read_as_a_narrower_integer_type_that_holds_it() {
        assert_eq!(read::<u8>(Content::U64(30)), Ok(30));
    }

    #[test]
    fn an_integer_is_read_as_a_wider_integer_type() {
        assert_eq!(read::<i64>(Content::U8(30)), Ok(30));
    }

    #[test]
    fn an_integer_out_of_its_type_s_range_is_refused_as_an_integer() {
        assert_eq!(read::<u8>(Content::U64(300)), Err(wrong_type("u8", ValueKind::Integer)));
    }

    #[test]
    fn a_negative_integer_is_refused_by_an_unsigned_type() {
        assert_eq!(read::<u32>(Content::I8(-1)), Err(wrong_type("u32", ValueKind::Integer)));
    }

    #[test]
    fn a_128_bit_integer_that_fits_in_64_bits_is_read_as_a_narrower_type() {
        assert_eq!(
            (read::<u8>(Content::U128(30)), read::<i8>(Content::I128(-3))),
            (Ok(30), Ok(-3))
        );
    }

    #[test]
    fn a_128_bit_integer_beyond_64_bits_is_read_as_a_128_bit_type() {
        assert_eq!(
            (
                read::<u128>(Content::U128(u128::MAX)),
                read::<i128>(Content::I128(i128::MIN))
            ),
            (Ok(u128::MAX), Ok(i128::MIN))
        );
    }

    #[test]
    fn a_128_bit_integer_beyond_64_bits_is_refused_by_a_narrower_type() {
        assert_eq!(
            read::<u64>(Content::U128(u128::MAX)),
            Err(WrongType {
                expected: Some("u64".to_owned()),
                found: None,
            })
        );
    }

    #[test]
    fn an_integer_is_read_as_a_float() {
        assert_eq!(read::<f64>(Content::U8(3)), Ok(3.0));
    }

    #[test]
    fn a_float_is_read_as_a_float() {
        assert_eq!(
            (read::<f32>(Content::F32(0.5)), read::<f64>(Content::F64(0.25))),
            (Ok(0.5), Ok(0.25))
        );
    }

    #[test]
    fn a_float_is_refused_as_an_integer() {
        assert_eq!(read::<u8>(Content::F64(1.5)), Err(wrong_type("u8", ValueKind::Float)));
    }

    #[test]
    fn a_boolean_is_read_as_a_boolean() {
        assert_eq!(read::<bool>(Content::Bool(true)), Ok(true));
    }

    #[test]
    fn a_string_is_refused_as_a_boolean() {
        assert_eq!(
            read::<bool>(Content::Str("yes")),
            Err(wrong_type("a boolean", ValueKind::String))
        );
    }

    #[test]
    fn a_string_that_was_lent_is_read_borrowed() {
        assert_eq!(read::<&str>(Content::Str("Lucía")), Ok("Lucía"));
    }

    #[test]
    fn an_owned_string_is_read_as_a_string() {
        assert_eq!(
            read::<String>(Content::String("Lucía".to_owned())),
            Ok("Lucía".to_owned())
        );
    }

    #[test]
    fn a_string_is_refused_as_a_number() {
        assert_eq!(
            read::<u8>(Content::Str("twelve")),
            Err(wrong_type("u8", ValueKind::String))
        );
    }

    #[test]
    fn a_number_is_refused_as_a_string() {
        assert_eq!(
            read::<String>(Content::U8(5)),
            Err(wrong_type("a string", ValueKind::Integer))
        );
    }

    #[test]
    fn a_character_is_read_as_a_character() {
        assert_eq!(read::<char>(Content::Char('ñ')), Ok('ñ'));
    }

    #[test]
    fn a_one_character_string_is_read_as_a_character() {
        assert_eq!(
            (
                read::<char>(Content::Str("ñ")),
                read::<char>(Content::String("a".to_owned()))
            ),
            (Ok('ñ'), Ok('a'))
        );
    }

    #[test]
    fn a_number_is_refused_as_a_character() {
        assert_eq!(
            read::<char>(Content::U8(5)),
            Err(wrong_type("a character", ValueKind::Integer))
        );
    }

    #[test]
    fn bytes_are_read_from_bytes_a_string_s_bytes_or_a_sequence() {
        assert_eq!(
            (
                read::<Bytes>(Content::ByteBuf(vec![1, 2])),
                read::<Bytes>(Content::String("ab".to_owned())),
                read::<Bytes>(Content::Seq(vec![Content::U8(1), Content::U8(2)]))
            ),
            (Ok(Bytes(vec![1, 2])), Ok(Bytes(vec![97, 98])), Ok(Bytes(vec![1, 2])))
        );
    }

    #[test]
    fn bytes_that_were_lent_are_read_borrowed_from_bytes_or_a_string() {
        assert_eq!(
            (read::<&[u8]>(Content::Bytes(b"ab")), read::<&[u8]>(Content::Str("ab"))),
            (Ok(&b"ab"[..]), Ok(&b"ab"[..]))
        );
    }

    #[test]
    fn a_number_is_refused_as_bytes() {
        assert_eq!(
            read::<Bytes>(Content::U8(5)),
            Err(wrong_type("bytes", ValueKind::Integer))
        );
    }

    #[test]
    fn bytes_are_read_as_a_string() {
        assert_eq!(
            (
                read::<String>(Content::Bytes(b"ab")),
                read::<String>(Content::ByteBuf(b"ab".to_vec()))
            ),
            (Ok("ab".to_owned()), Ok("ab".to_owned()))
        );
    }

    #[test]
    fn null_or_none_is_read_as_none() {
        assert_eq!(
            (read::<Option<u8>>(Content::Unit), read::<Option<u8>>(Content::None)),
            (Ok(None), Ok(None))
        );
    }

    #[test]
    fn a_value_or_some_value_is_read_as_some() {
        assert_eq!(
            (
                read::<Option<u8>>(Content::U8(1)),
                read::<Option<u8>>(Content::Some(Box::new(Content::U8(1))))
            ),
            (Ok(Some(1)), Ok(Some(1)))
        );
    }

    #[test]
    fn a_value_of_the_wrong_type_inside_an_option_is_refused_by_its_own_kind() {
        assert_eq!(
            read::<Option<u8>>(Content::Some(Box::new(Content::Str("x")))),
            Err(wrong_type("u8", ValueKind::String))
        );
    }

    #[test]
    fn null_is_refused_where_a_value_is_required() {
        assert_eq!(read::<u8>(Content::Unit), Err(wrong_type("u8", ValueKind::Null)));
    }

    #[test]
    fn null_is_read_as_unit() {
        assert_eq!(read::<()>(Content::Unit), Ok(()));
    }

    #[test]
    fn null_is_read_as_a_unit_struct() {
        assert_eq!(read::<Nothing>(Content::Unit), Ok(Nothing));
    }

    #[test]
    fn a_number_is_refused_as_unit() {
        assert_eq!(read::<()>(Content::U8(0)), Err(wrong_type("unit", ValueKind::Integer)));
    }

    #[test]
    fn a_newtype_struct_is_read_from_its_value_or_a_newtype_holding_it() {
        assert_eq!(
            (
                read::<Meters>(Content::U64(5)),
                read::<Meters>(Content::Newtype(Box::new(Content::U64(5))))
            ),
            (Ok(Meters(5)), Ok(Meters(5)))
        );
    }

    #[test]
    fn a_sequence_is_read_as_a_vec() {
        assert_eq!(
            read::<Vec<u8>>(Content::Seq(vec![Content::U8(1), Content::U64(2)])),
            Ok(vec![1, 2])
        );
    }

    #[test]
    fn a_map_is_refused_as_a_sequence() {
        assert_eq!(
            read::<Vec<u8>>(Content::Map(Vec::new())),
            Err(wrong_type("a sequence", ValueKind::Map))
        );
    }

    #[test]
    fn an_element_of_the_wrong_type_is_refused_by_its_own_kind() {
        assert_eq!(
            read::<Vec<u8>>(Content::Seq(vec![Content::U8(1), Content::Str("x")])),
            Err(wrong_type("u8", ValueKind::String))
        );
    }

    #[test]
    fn a_sequence_is_read_as_a_tuple_or_a_tuple_struct_of_its_length() {
        assert_eq!(
            (
                read::<(u8, u8)>(Content::Seq(vec![Content::U8(1), Content::U8(2)])),
                read::<Pair>(Content::Seq(vec![Content::U8(1), Content::U8(2)]))
            ),
            (Ok((1, 2)), Ok(Pair(1, 2)))
        );
    }

    #[test]
    fn a_sequence_longer_than_a_tuple_is_refused_by_the_length_expected() {
        let three_elements = Content::Seq(vec![Content::U8(1), Content::U8(2), Content::U8(3)]);

        assert_eq!(
            read::<(u8, u8)>(three_elements),
            Err(WrongType {
                expected: Some("2 elements in sequence".to_owned()),
                found: None,
            })
        );
    }

    #[test]
    fn a_map_is_read_as_a_map() {
        let entries = Content::Map(vec![
            (Content::Str("b"), Content::U8(2)),
            (Content::String("a".to_owned()), Content::U8(1)),
        ]);

        assert_eq!(
            read::<BTreeMap<String, u8>>(entries),
            Ok(BTreeMap::from([("a".to_owned(), 1), ("b".to_owned(), 2)]))
        );
    }

    #[test]
    fn a_map_key_written_as_a_string_is_read_as_the_integer_its_type_asks_for() {
        let entries = Content::Map(vec![
            (Content::Str("1"), Content::U8(1)),
            (Content::String("-2".to_owned()), Content::U8(2)),
        ]);

        assert_eq!(
            read::<BTreeMap<i32, u8>>(entries),
            Ok(BTreeMap::from([(1, 1), (-2, 2)]))
        );
    }

    #[test]
    fn a_map_key_written_as_a_string_is_read_as_a_128_bit_integer_beyond_64_bits() {
        let largest = Content::Map(vec![(
            Content::Str("340282366920938463463374607431768211455"),
            Content::U8(1),
        )]);
        let smallest = Content::Map(vec![(
            Content::Str("-170141183460469231731687303715884105728"),
            Content::U8(1),
        )]);

        assert_eq!(
            (
                read::<BTreeMap<u128, u8>>(largest),
                read::<BTreeMap<i128, u8>>(smallest)
            ),
            (
                Ok(BTreeMap::from([(u128::MAX, 1)])),
                Ok(BTreeMap::from([(i128::MIN, 1)]))
            )
        );
    }

    #[test]
    fn a_map_key_written_as_a_string_is_read_as_the_boolean_its_type_asks_for() {
        let entries = Content::Map(vec![(Content::Str("true"), Content::U8(1))]);

        assert_eq!(read::<BTreeMap<bool, u8>>(entries), Ok(BTreeMap::from([(true, 1)])));
    }

    #[test]
    fn a_map_key_written_as_a_string_is_read_through_a_newtype_or_an_option() {
        let shelves = || Content::Map(vec![(Content::Str("3"), Content::U8(1))]);

        assert_eq!(
            (
                read::<BTreeMap<ShelfId, u8>>(shelves()),
                read::<BTreeMap<Option<u32>, u8>>(shelves())
            ),
            (
                Ok(BTreeMap::from([(ShelfId(3), 1)])),
                Ok(BTreeMap::from([(Some(3), 1)]))
            )
        );
    }

    #[test]
    fn a_map_key_written_as_a_string_that_is_not_an_integer_is_refused_as_a_string() {
        let entries = Content::Map(vec![(Content::Str("+3"), Content::U8(1))]);

        assert_eq!(
            read::<BTreeMap<u32, u8>>(entries),
            Err(wrong_type("u32", ValueKind::String))
        );
    }

    #[test]
    fn a_map_key_is_read_as_it_came_when_its_type_asks_for_what_came() {
        let integer_key = Content::Map(vec![(Content::U64(3), Content::U8(1))]);
        let string_key = Content::Map(vec![(Content::Str("3"), Content::U8(1))]);

        assert_eq!(
            (
                read::<BTreeMap<u32, u8>>(integer_key),
                read::<BTreeMap<String, u8>>(string_key)
            ),
            (Ok(BTreeMap::from([(3, 1)])), Ok(BTreeMap::from([("3".to_owned(), 1)])))
        );
    }

    #[test]
    fn a_sequence_is_refused_as_a_map() {
        assert_eq!(
            read::<BTreeMap<String, u8>>(Content::Seq(Vec::new())),
            Err(wrong_type("a map", ValueKind::Sequence))
        );
    }

    #[test]
    fn a_struct_is_read_from_a_map_or_a_sequence() {
        let as_map = Content::Map(vec![
            (Content::Str("y"), Content::I8(-2)),
            (Content::Str("x"), Content::U8(1)),
        ]);
        let as_sequence = Content::Seq(vec![Content::U8(1), Content::I8(-2)]);

        assert_eq!(
            (read::<Point>(as_map), read::<Point>(as_sequence)),
            (Ok(Point { x: 1, y: -2 }), Ok(Point { x: 1, y: -2 }))
        );
    }

    #[test]
    fn a_number_is_refused_as_a_struct() {
        assert_eq!(
            read::<Point>(Content::U8(5)),
            Err(wrong_type("struct Point", ValueKind::Integer))
        );
    }

    #[test]
    fn a_struct_missing_a_field_is_refused_without_a_reason_serde_structures() {
        let without_y = Content::Map(vec![(Content::Str("x"), Content::U8(1))]);

        assert_eq!(
            read::<Point>(without_y),
            Err(WrongType {
                expected: None,
                found: None,
            })
        );
    }

    #[test]
    fn a_unit_variant_is_read_from_its_name() {
        assert_eq!(
            (
                read::<Status>(Content::Str("Active")),
                read::<Status>(Content::String("Active".to_owned()))
            ),
            (Ok(Status::Active), Ok(Status::Active))
        );
    }

    #[test]
    fn a_variant_with_a_value_is_read_from_a_map_with_one_entry() {
        let suspended = Content::Map(vec![(Content::Str("Suspended"), Content::U8(3))]);
        let moved = Content::Map(vec![(
            Content::Str("Moved"),
            Content::Map(vec![(Content::Str("to"), Content::Str("Sevilla"))]),
        )]);

        assert_eq!(
            (read::<Status>(suspended), read::<Status>(moved)),
            (
                Ok(Status::Suspended(3)),
                Ok(Status::Moved {
                    to: "Sevilla".to_owned()
                })
            )
        );
    }

    #[test]
    fn a_map_with_more_than_one_entry_is_refused_as_the_enum_expected() {
        let two_variants = Content::Map(vec![
            (Content::Str("Active"), Content::Unit),
            (Content::Str("Suspended"), Content::U8(3)),
        ]);

        assert_eq!(
            read::<Status>(two_variants),
            Err(wrong_type("enum Status", ValueKind::Map))
        );
    }

    #[test]
    fn a_number_is_refused_as_the_enum_expected() {
        assert_eq!(
            read::<Status>(Content::U8(0)),
            Err(wrong_type("enum Status", ValueKind::Integer))
        );
    }

    #[test]
    fn any_value_can_be_ignored() {
        assert_eq!(
            read::<IgnoredAny>(Content::Map(vec![(Content::Str("a"), Content::Unit)])),
            Ok(IgnoredAny)
        );
    }

    #[test]
    fn a_value_read_as_any_value_is_read_whole() {
        let every_kind = || {
            Content::Seq(vec![
                Content::Bool(true),
                Content::U8(8),
                Content::U16(16),
                Content::U32(32),
                Content::U64(64),
                Content::I8(-8),
                Content::I16(-16),
                Content::I32(-32),
                Content::I64(-64),
                Content::U128(u128::MAX),
                Content::I128(i128::MIN),
                Content::F32(0.5),
                Content::F64(0.25),
                Content::Char('ñ'),
                Content::Str("lent"),
                Content::String("owned".to_owned()),
                Content::Bytes(b"lent"),
                Content::ByteBuf(b"owned".to_vec()),
                Content::None,
                Content::Some(Box::new(Content::Unit)),
                Content::Unit,
                Content::Newtype(Box::new(Content::U8(1))),
                Content::Map(vec![(Content::Str("key"), Content::Seq(Vec::new()))]),
            ])
        };

        assert_eq!(read::<Content>(every_kind()), Ok(every_kind()));
    }

    #[test]
    fn a_128_bit_integer_that_fits_in_64_bits_is_read_as_any_value_by_its_value() {
        assert_eq!(
            (read::<Content>(Content::U128(1)), read::<Content>(Content::I128(-1))),
            (Ok(Content::U64(1)), Ok(Content::I64(-1)))
        );
    }

    #[test]
    fn a_field_is_named_by_text_bytes_or_index() {
        let fields = Content::Map(vec![
            (Content::Bytes(b"y"), Content::I8(-2)),
            (Content::U64(0), Content::U8(1)),
        ]);

        assert_eq!(read::<Point>(fields), Ok(Point { x: 1, y: -2 }));
    }

    #[test]
    fn the_format_s_human_readability_is_answered_for_the_value_and_everything_in_it() {
        let deserializer = ContentDeserializer::<WrongType>::new(Content::Seq(vec![Content::Unit]), false);

        assert_eq!(
            Vec::<HumanReadability>::deserialize(deserializer),
            Ok(vec![HumanReadability(false)])
        );
    }

    #[test]
    fn the_format_s_human_readability_is_answered_for_a_map_s_keys() {
        let entries = Content::Map(vec![(Content::Str("key"), Content::U8(1))]);
        let deserializer = ContentDeserializer::<WrongType>::new(entries, false);

        assert_eq!(
            BTreeMap::<HumanReadability, u8>::deserialize(deserializer),
            Ok(BTreeMap::from([(HumanReadability(false), 1)]))
        );
    }

    #[test]
    fn a_type_error_keeps_what_was_expected_and_the_kind_of_value_found_but_not_the_value() {
        assert_eq!(
            WrongType::invalid_type(Unexpected::Str("hunter2"), &"u8"),
            wrong_type("u8", ValueKind::String)
        );
    }

    #[test]
    fn a_value_error_keeps_what_was_expected_and_the_kind_of_value_found_but_not_the_value() {
        assert_eq!(
            WrongType::invalid_value(Unexpected::Unsigned(300), &"u8"),
            wrong_type("u8", ValueKind::Integer)
        );
    }

    #[test]
    fn a_length_error_keeps_what_was_expected() {
        assert_eq!(
            WrongType::invalid_length(3, &"a tuple of size 2"),
            WrongType {
                expected: Some("a tuple of size 2".to_owned()),
                found: None,
            }
        );
    }

    #[test]
    fn a_custom_error_keeps_nothing_of_its_message() {
        assert_eq!(
            WrongType::custom("hunter2 is too weak"),
            WrongType {
                expected: None,
                found: None,
            }
        );
    }

    #[test]
    fn a_wrong_type_reads_as_the_kind_found_and_what_was_expected() {
        assert_eq!(
            wrong_type("u8", ValueKind::String).to_string(),
            "invalid type: string, expected u8"
        );
    }

    #[test]
    fn a_wrong_type_without_a_reason_serde_structures_reads_as_an_invalid_type() {
        let without_a_reason = WrongType {
            expected: None,
            found: None,
        };

        assert_eq!(without_a_reason.to_string(), "invalid type");
    }
}
