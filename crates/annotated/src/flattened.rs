use std::marker::PhantomData;

use serde::de::{self, DeserializeSeed, Deserializer, MapAccess, Visitor};

use crate::collecting_type_errors::invalid_type;
use crate::content::{Content, ValueKind};
use crate::content_deserializer::{ContentDeserializer, WrongType};
use crate::{Location, Report};

/// The entries of a map that aren't fields of the struct being read. A struct with a flattened field keeps them, and
/// reads each flattened field from them once the map is read, as serde's derive does.
pub struct Leftovers<'de> {
    entries: Vec<Option<(Content<'de>, Content<'de>)>>,
}

impl<'de> Leftovers<'de> {
    /// No entries yet.
    #[must_use]
    pub const fn new() -> Self {
        Self { entries: Vec::new() }
    }

    /// Keeps `key` and the value `map` has for it.
    ///
    /// # Errors
    ///
    /// Returns the format's error when the value isn't well formed.
    pub fn keep<A: MapAccess<'de>>(&mut self, key: Key<'de>, map: &mut A) -> Result<(), A::Error> {
        let value = map.next_value::<Content<'de>>()?;
        self.entries.push(Some((key.0, value)));
        Ok(())
    }

    /// A deserializer for a flattened field. A struct takes the entries whose keys are its fields, and a map reads
    /// every entry still kept, leaving them for the flattened fields after it.
    pub fn flattened<E: de::Error>(&mut self) -> FlatMapDeserializer<'_, 'de, E> {
        FlatMapDeserializer {
            entries: &mut self.entries,
            error: PhantomData,
        }
    }

    /// Reads a flattened `T`, and pushes into `found`, at `at`, a [`Problem::InvalidType`](crate::Problem) when the
    /// entries don't fit it. `type_name` says what was expected when serde's error doesn't.
    pub fn flattened_collecting_type_errors<T: serde::Deserialize<'de>>(
        &mut self,
        type_name: &'static str,
        at: &Location<'_>,
        found: &mut Report,
    ) -> Option<T> {
        match T::deserialize(self.flattened::<WrongType>()) {
            Ok(value) => Some(value),
            Err(wrong_type) => {
                found.push(at, invalid_type(wrong_type, type_name, ValueKind::Map));
                None
            }
        }
    }
}

impl Default for Leftovers<'_> {
    fn default() -> Self {
        Self::new()
    }
}

/// A key of a map read into a struct with a flattened field, kept whole in case it isn't one of the struct's.
pub struct Key<'de>(Content<'de>);

impl Key<'_> {
    /// The key as a field name, when it is text.
    #[must_use]
    pub fn as_field_name(&self) -> Option<&str> {
        field_name_of(&self.0)
    }
}

fn field_name_of<'a>(key: &'a Content<'_>) -> Option<&'a str> {
    match key {
        Content::Str(name) => Some(name),
        Content::String(name) => Some(name),
        Content::Bytes(name) => std::str::from_utf8(name).ok(),
        Content::ByteBuf(name) => std::str::from_utf8(name).ok(),
        _ => None,
    }
}

macro_rules! only_structs_and_maps_for {
    ($($method:ident($($argument:ty),*))*) => {
        $(
            fn $method<V: Visitor<'de>>(self, $(_: $argument,)* _visitor: V) -> Result<V::Value, E> {
                Err(E::custom("can only flatten structs and maps"))
            }
        )*
    };
}

impl<'de> serde::Deserialize<'de> for Key<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Content::deserialize(deserializer).map(Self)
    }
}

/// Reads a flattened field from the entries a struct kept.
pub struct FlatMapDeserializer<'a, 'de, E> {
    entries: &'a mut Vec<Option<(Content<'de>, Content<'de>)>>,
    error: PhantomData<E>,
}

impl<'de, E: de::Error> Deserializer<'de> for FlatMapDeserializer<'_, 'de, E> {
    type Error = E;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        self.deserialize_map(visitor)
    }

    fn deserialize_map<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        visitor.visit_map(FlatMapAccess {
            entries: self.entries.iter(),
            value: None,
            error: PhantomData,
        })
    }

    fn deserialize_struct<V: Visitor<'de>>(
        self,
        _name: &'static str,
        fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, E> {
        visitor.visit_map(FlatStructAccess {
            entries: self.entries.iter_mut(),
            fields,
            value: None,
            error: PhantomData,
        })
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        name: &'static str,
        variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, E> {
        let variant = self.entries.iter_mut().find_map(|entry| take_if_named(entry, variants));
        let Some(variant) = variant else {
            return Err(E::custom(format_args!(
                "no variant of enum {name} found in flattened data"
            )));
        };
        ContentDeserializer::new(Content::Map(vec![variant]), true).deserialize_enum(name, variants, visitor)
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value, E> {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        visitor.visit_some(self)
    }

    fn deserialize_unit<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V: Visitor<'de>>(self, _name: &'static str, visitor: V) -> Result<V::Value, E> {
        visitor.visit_unit()
    }

    fn deserialize_ignored_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, E> {
        visitor.visit_unit()
    }

    only_structs_and_maps_for! {
        deserialize_bool()
        deserialize_i8()
        deserialize_i16()
        deserialize_i32()
        deserialize_i64()
        deserialize_i128()
        deserialize_u8()
        deserialize_u16()
        deserialize_u32()
        deserialize_u64()
        deserialize_u128()
        deserialize_f32()
        deserialize_f64()
        deserialize_char()
        deserialize_str()
        deserialize_string()
        deserialize_bytes()
        deserialize_byte_buf()
        deserialize_seq()
        deserialize_tuple(usize)
        deserialize_tuple_struct(&'static str, usize)
        deserialize_identifier()
    }
}

/// Reads every entry still kept, leaving each for the flattened fields after it.
struct FlatMapAccess<'a, 'de, E> {
    entries: std::slice::Iter<'a, Option<(Content<'de>, Content<'de>)>>,
    value: Option<&'a Content<'de>>,
    error: PhantomData<E>,
}

impl<'de, E: de::Error> MapAccess<'de> for FlatMapAccess<'_, 'de, E> {
    type Error = E;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, E> {
        let Some((key, value)) = self.entries.by_ref().flatten().next() else {
            return Ok(None);
        };
        self.value = Some(value);
        seed.deserialize(ContentDeserializer::new(key.clone(), true)).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, E> {
        let value = self.value.take().ok_or_else(|| E::custom("value is missing"))?;
        seed.deserialize(ContentDeserializer::new(value.clone(), true))
    }
}

/// Takes the entries whose keys are a struct's fields.
struct FlatStructAccess<'a, 'de, E> {
    entries: std::slice::IterMut<'a, Option<(Content<'de>, Content<'de>)>>,
    fields: &'static [&'static str],
    value: Option<Content<'de>>,
    error: PhantomData<E>,
}

impl<'de, E: de::Error> MapAccess<'de> for FlatStructAccess<'_, 'de, E> {
    type Error = E;

    fn next_key_seed<K: DeserializeSeed<'de>>(&mut self, seed: K) -> Result<Option<K::Value>, E> {
        let fields = self.fields;
        let Some((key, value)) = self.entries.by_ref().find_map(|entry| take_if_named(entry, fields)) else {
            return Ok(None);
        };
        self.value = Some(value);
        seed.deserialize(ContentDeserializer::new(key, true)).map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, E> {
        let value = self.value.take().ok_or_else(|| E::custom("value is missing"))?;
        seed.deserialize(ContentDeserializer::new(value, true))
    }
}

fn take_if_named<'de>(
    entry: &mut Option<(Content<'de>, Content<'de>)>,
    names: &[&str],
) -> Option<(Content<'de>, Content<'de>)> {
    let (key, _) = entry.as_ref()?;
    let is_named = field_name_of(key).is_some_and(|name| names.contains(&name));
    if is_named { entry.take() } else { None }
}
