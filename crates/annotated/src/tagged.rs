use std::borrow::Cow;
use std::fmt;

use serde::de::{self, Deserializer, Error as _, MapAccess, SeqAccess, Visitor};

use crate::Problem;
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
