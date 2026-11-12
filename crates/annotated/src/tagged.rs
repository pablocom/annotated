use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;

use serde::Deserialize as _;
use serde::de::{self, DeserializeSeed, Deserializer, Error as _, MapAccess, SeqAccess, Visitor};

use crate::Problem;
use crate::collecting_type_errors::problem_of;
use crate::content::{Content, ValueKind};
use crate::content_deserializer::{ContentDeserializer, WrongType};
use crate::flattened::field_name_of;

/// A value read whole into a buffer, to be read again once its tag says which variant it is.
pub struct Buffered<'de> {
    content: Content<'de>,
    is_human_readable: bool,
}

impl<'de> Buffered<'de> {
    /// A deserializer that reads the buffered value, ending with an error of type `E` at the first failure.
    #[must_use]
    pub fn deserializer<E: de::Error>(self) -> impl Deserializer<'de, Error = E> {
        ContentDeserializer::new(self.content, self.is_human_readable)
    }
}

/// What reading an internally tagged enum found: the variant its tag names, with every other entry buffered, or the
/// issue to report instead.
pub enum Tagged<'de, V> {
    /// The tag names the variant `V`. The other entries wait in the buffer.
    Variant(V, Buffered<'de>),
    /// The tag is missing, or names no variant. The problem is reported at the tag's name.
    IssueAtTheTag(Problem),
    /// The value isn't the enum at all, under `collect_type_errors`. The problem is reported at the enum's location.
    IssueAtTheEnum(Problem),
}

/// Reads an internally tagged enum, as serde's derive does: the value of the key named `tag`, which names the variant
/// `V`, and every other entry into a buffer. A value given as a sequence starts with its tag.
///
/// A missing tag, or one that names none of the `variants`, is returned as an issue, so that reading can go on. A
/// value that is neither a map nor a sequence is the format's error, unless `collects_type_errors`.
///
/// # Errors
///
/// The format's own error, when the value isn't well-formed or has the tag twice.
pub fn read_internally_tagged<'de, D, V>(
    deserializer: D,
    tag: &'static str,
    variants: &'static [&'static str],
    expecting: &'static str,
    collects_type_errors: bool,
) -> Result<Tagged<'de, V>, D::Error>
where
    D: Deserializer<'de>,
    V: serde::Deserialize<'de>,
{
    let is_human_readable = deserializer.is_human_readable();
    let (tag_value, content) = if collects_type_errors {
        let content = <Content as serde::Deserialize>::deserialize(deserializer)?;
        match tag_split_from::<D::Error>(content, tag)? {
            Ok(split) => split,
            Err(kind_found) => {
                return Ok(Tagged::IssueAtTheEnum(Problem::InvalidType {
                    expected: Cow::Borrowed(expecting),
                    found: kind_found.name(),
                }));
            }
        }
    } else {
        deserializer.deserialize_any(TaggedContentVisitor { tag, expecting })?
    };
    let Some(tag_value) = tag_value else {
        return Ok(Tagged::IssueAtTheTag(Problem::Missing));
    };
    let content = Buffered {
        content,
        is_human_readable,
    };
    match V::deserialize(ContentDeserializer::<WrongType>::new(tag_value, is_human_readable)) {
        Ok(variant) => Ok(Tagged::Variant(variant, content)),
        Err(_) => Ok(Tagged::IssueAtTheTag(Problem::UnknownVariant { expected: variants })),
    }
}

type TagAndContent<'de> = (Option<Content<'de>>, Content<'de>);

fn tag_split_from<'de, E: de::Error>(
    content: Content<'de>,
    tag: &'static str,
) -> Result<Result<TagAndContent<'de>, ValueKind>, E> {
    match content {
        Content::Map(mut entries) => {
            let mut tag_positions = entries
                .iter()
                .enumerate()
                .filter(|(_, (key, _))| field_name_of(key) == Some(tag))
                .map(|(position, _)| position);
            let tag_position = tag_positions.next();
            if tag_positions.next().is_some() {
                return Err(E::duplicate_field(tag));
            }
            let tag_value = tag_position.map(|position| entries.remove(position).1);
            Ok(Ok((tag_value, Content::Map(entries))))
        }
        Content::Seq(mut elements) => {
            let tag_value = (!elements.is_empty()).then(|| elements.remove(0));
            Ok(Ok((tag_value, Content::Seq(elements))))
        }
        other => Ok(Err(other.kind())),
    }
}

struct TaggedContentVisitor {
    tag: &'static str,
    expecting: &'static str,
}

impl<'de> Visitor<'de> for TaggedContentVisitor {
    type Value = TagAndContent<'de>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.expecting)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut tag_value = None;
        let mut entries = Vec::new();
        while let Some(key) = map.next_key::<Content<'de>>()? {
            if field_name_of(&key) == Some(self.tag) {
                if tag_value.is_some() {
                    return Err(A::Error::duplicate_field(self.tag));
                }
                tag_value = Some(map.next_value::<Content<'de>>()?);
            } else {
                entries.push((key, map.next_value::<Content<'de>>()?));
            }
        }
        Ok((tag_value, Content::Map(entries)))
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let tag_value = seq.next_element::<Content<'de>>()?;
        let mut elements = Vec::new();
        while let Some(element) = seq.next_element::<Content<'de>>()? {
            elements.push(element);
        }
        Ok((tag_value, Content::Seq(elements)))
    }
}

impl<'de> serde::Deserialize<'de> for Buffered<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let is_human_readable = deserializer.is_human_readable();
        let content = <Content as serde::Deserialize>::deserialize(deserializer)?;
        Ok(Self {
            content,
            is_human_readable,
        })
    }
}

impl<'de> Buffered<'de> {
    /// Reads the buffered value with `seed`. A value that doesn't fit is returned as the [`Problem`] to report, instead
    /// of ending deserialization. `type_name` says what was expected when serde's error doesn't.
    ///
    /// # Errors
    ///
    /// The problem, when the value doesn't fit what `seed` reads.
    pub fn read_collecting_type_errors<S, T>(self, seed: S, type_name: &'static str) -> Result<Option<T>, Problem>
    where
        S: DeserializeSeed<'de, Value = Option<T>>,
    {
        let kind_found = self.content.kind();
        seed.deserialize(ContentDeserializer::<WrongType>::new(
            self.content,
            self.is_human_readable,
        ))
        .map_err(|wrong_type| problem_of(wrong_type, type_name, kind_found))
    }
}

/// A key of an adjacently tagged enum's map.
pub enum TagOrContent {
    /// The key naming the tag.
    Tag,
    /// The key naming the content.
    Content,
    /// Any other key.
    Other,
}

/// Reads a key of an adjacently tagged enum's map, as serde's derive does. Any key other than the tag and the content
/// is an unknown field when `is_denying_unknown_fields`.
pub struct TagOrContentKey {
    tag: &'static str,
    content: &'static str,
    fields: &'static [&'static str],
    is_denying_unknown_fields: bool,
}

impl TagOrContentKey {
    /// Reads the key named `tag`, the key named `content`, and any other key. `fields` names the two.
    #[must_use]
    pub const fn new(
        tag: &'static str,
        content: &'static str,
        fields: &'static [&'static str],
        is_denying_unknown_fields: bool,
    ) -> Self {
        Self {
            tag,
            content,
            fields,
            is_denying_unknown_fields,
        }
    }

    fn key_named<E: de::Error>(&self, name: &str) -> Result<TagOrContent, E> {
        if name == self.tag {
            return Ok(TagOrContent::Tag);
        }
        if name == self.content {
            return Ok(TagOrContent::Content);
        }
        if self.is_denying_unknown_fields {
            return Err(E::unknown_field(name, self.fields));
        }
        Ok(TagOrContent::Other)
    }
}

impl<'de> DeserializeSeed<'de> for TagOrContentKey {
    type Value = TagOrContent;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<TagOrContent, D::Error> {
        deserializer.deserialize_identifier(self)
    }
}

impl Visitor<'_> for TagOrContentKey {
    type Value = TagOrContent;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?} or {:?}", self.tag, self.content)
    }

    fn visit_u64<E: de::Error>(self, index: u64) -> Result<TagOrContent, E> {
        match index {
            0 => Ok(TagOrContent::Tag),
            1 => Ok(TagOrContent::Content),
            _ if self.is_denying_unknown_fields => Err(E::invalid_value(de::Unexpected::Unsigned(index), &self)),
            _ => Ok(TagOrContent::Other),
        }
    }

    fn visit_str<E: de::Error>(self, name: &str) -> Result<TagOrContent, E> {
        self.key_named(name)
    }

    fn visit_bytes<E: de::Error>(self, name: &[u8]) -> Result<TagOrContent, E> {
        self.key_named(&String::from_utf8_lossy(name))
    }
}

/// Reads a tag's value as the variant `V` it names, or as `None` when it names none, whatever its type, so that the
/// enum can report it and reading can go on.
pub struct TagValue<V>(PhantomData<fn() -> V>);

impl<V> TagValue<V> {
    /// A seed reading a tag naming a `V`.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<V> Default for TagValue<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'de, V: serde::Deserialize<'de>> DeserializeSeed<'de> for TagValue<V> {
    type Value = Option<V>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Option<V>, D::Error> {
        let Buffered {
            content,
            is_human_readable,
        } = Buffered::deserialize(deserializer)?;
        Ok(V::deserialize(ContentDeserializer::<WrongType>::new(content, is_human_readable)).ok())
    }
}

/// Reads a unit variant's content in an adjacently tagged or untagged enum: a unit, or nothing, as with serde.
pub struct UnitVariantVisitor {
    enum_name: &'static str,
    variant_name: &'static str,
}

impl UnitVariantVisitor {
    /// Reads the content of the unit variant `variant_name` of the enum `enum_name`.
    #[must_use]
    pub const fn new(enum_name: &'static str, variant_name: &'static str) -> Self {
        Self {
            enum_name,
            variant_name,
        }
    }
}

impl Visitor<'_> for UnitVariantVisitor {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unit variant {}::{}", self.enum_name, self.variant_name)
    }

    fn visit_unit<E: de::Error>(self) -> Result<(), E> {
        Ok(())
    }

    fn visit_none<E: de::Error>(self) -> Result<(), E> {
        Ok(())
    }
}
