use std::marker::PhantomData;

use serde::de::value::{
    BorrowedStrDeserializer, MapAccessDeserializer, MapDeserializer, SeqDeserializer, StrDeserializer,
};
use serde::de::{self, Deserializer, Expected, IntoDeserializer, Visitor};

use crate::content::Content;
use crate::content_deserializer::{
    ContentDeserializer, boolean_written_in, integer_written_in, visit_i128, visit_u128,
};

/// Reads a [`Content`] by reference, so the same buffer can be read again: each variant of an untagged enum is tried
/// on it in turn, as serde's derive does with its own buffer.
pub(crate) struct ContentRefDeserializer<'a, 'de, E> {
    content: &'a Content<'de>,
    is_human_readable: bool,
    error: PhantomData<E>,
}

impl<'a, 'de, E> ContentRefDeserializer<'a, 'de, E> {
    /// Reads `content`, and reports `is_human_readable` as the format it came from does.
    pub(crate) const fn new(content: &'a Content<'de>, is_human_readable: bool) -> Self {
        Self {
            content,
            is_human_readable,
            error: PhantomData,
        }
    }
}

impl<'a, 'de, E: de::Error> ContentRefDeserializer<'a, 'de, E> {
    #[cold]
    fn invalid_type(&self, expected: &dyn Expected) -> E {
        E::invalid_type(self.content.unexpected(), expected)
    }

    fn deserialize_integer<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match *self.content {
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
        match *self.content {
            Content::F32(float) => visitor.visit_f32(float),
            Content::F64(float) => visitor.visit_f64(float),
            _ => self.deserialize_integer(visitor),
        }
    }

    fn visit_elements<V: Visitor<'de>>(
        elements: &'a [Content<'de>],
        is_human_readable: bool,
        visitor: V,
    ) -> Result<V::Value, E> {
        let mut elements = SeqDeserializer::new(elements.iter().map(|element| Self::new(element, is_human_readable)));
        let value = visitor.visit_seq(&mut elements)?;
        elements.end()?;
        Ok(value)
    }

    fn visit_entries<V: Visitor<'de>>(
        entries: &'a [(Content<'de>, Content<'de>)],
        is_human_readable: bool,
        visitor: V,
    ) -> Result<V::Value, E> {
        let mut entries = Self::entries(entries, is_human_readable);
        let value = visitor.visit_map(&mut entries)?;
        entries.end()?;
        Ok(value)
    }

    fn entries(
        entries: &'a [(Content<'de>, Content<'de>)],
        is_human_readable: bool,
    ) -> MapDeserializer<'de, impl Iterator<Item = (MapKeyRefDeserializer<'a, 'de, E>, Self)>, E> {
        MapDeserializer::new(entries.iter().map(move |(key, value)| {
            let key = MapKeyRefDeserializer {
                key: Self::new(key, is_human_readable),
            };
            (key, Self::new(value, is_human_readable))
        }))
    }
}

impl<'de, E: de::Error> Deserializer<'de> for ContentRefDeserializer<'_, 'de, E> {
    type Error = E;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.content {
            Content::Bool(boolean) => visitor.visit_bool(*boolean),
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
            Content::F32(float) => visitor.visit_f32(*float),
            Content::F64(float) => visitor.visit_f64(*float),
            Content::Char(character) => visitor.visit_char(*character),
            Content::String(string) => visitor.visit_str(string),
            Content::Str(string) => visitor.visit_borrowed_str(string),
            Content::ByteBuf(bytes) => visitor.visit_bytes(bytes),
            Content::Bytes(bytes) => visitor.visit_borrowed_bytes(bytes),
            Content::None => visitor.visit_none(),
            Content::Some(inner) => visitor.visit_some(Self::new(inner, self.is_human_readable)),
            Content::Unit => visitor.visit_unit(),
            Content::Newtype(inner) => visitor.visit_newtype_struct(Self::new(inner, self.is_human_readable)),
            Content::Seq(elements) => Self::visit_elements(elements, self.is_human_readable, visitor),
            Content::Map(entries) => Self::visit_entries(entries, self.is_human_readable, visitor),
        }
    }

    fn deserialize_bool<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match *self.content {
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
            Content::Char(character) => visitor.visit_char(*character),
            Content::String(string) => visitor.visit_str(string),
            Content::Str(string) => visitor.visit_borrowed_str(string),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_str<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.deserialize_string(visitor)
    }

    fn deserialize_string<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.content {
            Content::String(string) => visitor.visit_str(string),
            Content::Str(string) => visitor.visit_borrowed_str(string),
            Content::ByteBuf(bytes) => visitor.visit_bytes(bytes),
            Content::Bytes(bytes) => visitor.visit_borrowed_bytes(bytes),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_bytes<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.deserialize_byte_buf(visitor)
    }

    fn deserialize_byte_buf<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.content {
            Content::ByteBuf(bytes) => visitor.visit_bytes(bytes),
            Content::Bytes(bytes) => visitor.visit_borrowed_bytes(bytes),
            Content::String(string) => visitor.visit_bytes(string.as_bytes()),
            Content::Str(string) => visitor.visit_borrowed_bytes(string.as_bytes()),
            Content::Seq(elements) => Self::visit_elements(elements, self.is_human_readable, visitor),
            _ => Err(self.invalid_type(&visitor)),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        match self.content {
            Content::None | Content::Unit => visitor.visit_none(),
            Content::Some(inner) => visitor.visit_some(Self::new(inner, self.is_human_readable)),
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
            Content::Newtype(inner) => visitor.visit_newtype_struct(Self::new(inner, self.is_human_readable)),
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
            Content::String(variant) => visitor.visit_enum(StrDeserializer::new(variant)),
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

impl<'de, E: de::Error> IntoDeserializer<'de, E> for ContentRefDeserializer<'_, 'de, E> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self {
        self
    }
}

/// Reads a map's key by reference, accepting an integer or a boolean written as a string, as
/// [`ContentDeserializer`]'s keys do.
struct MapKeyRefDeserializer<'a, 'de, E> {
    key: ContentRefDeserializer<'a, 'de, E>,
}

impl<'de, E: de::Error> MapKeyRefDeserializer<'_, 'de, E> {
    fn written_as(&self, read_from_text: fn(&str) -> Option<Content<'de>>) -> Option<ContentDeserializer<'de, E>> {
        let read = match self.key.content {
            Content::Str(text) => read_from_text(text),
            Content::String(text) => read_from_text(text),
            _ => None,
        };
        read.map(|read| ContentDeserializer::new(read, self.key.is_human_readable))
    }
}

macro_rules! read_as_written {
    ($($method:ident: $read_from_text:ident),* $(,)?) => {
        $(
            fn $method<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
                match self.written_as($read_from_text) {
                    Some(read) => read.$method(visitor),
                    None => self.key.$method(visitor),
                }
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

impl<'de, E: de::Error> Deserializer<'de> for MapKeyRefDeserializer<'_, 'de, E> {
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

impl<'de, E: de::Error> IntoDeserializer<'de, E> for MapKeyRefDeserializer<'_, 'de, E> {
    type Deserializer = Self;

    fn into_deserializer(self) -> Self {
        self
    }
}
