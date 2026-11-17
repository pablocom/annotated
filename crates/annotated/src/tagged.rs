use std::borrow::Cow;
use std::fmt;
use std::marker::PhantomData;

use serde::de::{
    self, DeserializeSeed, Deserializer, EnumAccess, Error as _, IgnoredAny, MapAccess, SeqAccess, Unexpected,
    VariantAccess, Visitor,
};

use crate::Problem;
use crate::collecting_type_errors::problem_or_error;
use crate::content::{Content, ValueKind};
use crate::content_deserializer::{ContentDeserializer, ReadAs, TypeError, WrongType, is_annotated_s_buffer};
use crate::content_ref_deserializer::ContentRefDeserializer;
use crate::deserialize_containers::room_for;
use crate::flattened::field_name_of;

/// A value read whole into a buffer, to be read again once its tag says which variant it is.
pub struct Buffered<'de> {
    content: Content<'de>,
    read_as: ReadAs,
}

impl<'de> Buffered<'de> {
    /// A deserializer that reads the buffered value, ending with an error of type `E` at the first failure.
    #[must_use]
    pub fn deserializer<E: de::Error>(self) -> impl Deserializer<'de, Error = E> {
        ContentDeserializer::new(self.content, self.read_as)
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
/// `V` as the deserializer reads a name, and every other entry into a buffer. A value given as a sequence starts with
/// its tag.
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
    let reading_the_enum = TaggedContentVisitor {
        tag,
        expecting,
        collects_type_errors,
        variant: PhantomData,
    };
    let (tag_found, content) = match deserializer.deserialize_any(reading_the_enum)? {
        TaggedContent::Read { tag_found, content } => (tag_found, content),
        TaggedContent::NotTheEnum(kind_found) => {
            return Ok(Tagged::IssueAtTheEnum(Problem::InvalidType {
                expected: Cow::Borrowed(expecting),
                found: kind_found.name(),
            }));
        }
    };
    match tag_found {
        TagFound::Missing => Ok(Tagged::IssueAtTheTag(Problem::Missing)),
        TagFound::NamingNoVariant => Ok(Tagged::IssueAtTheTag(Problem::UnknownVariant { expected: variants })),
        TagFound::Naming(variant) => {
            let content = Buffered {
                content,
                read_as: ReadAs::SerdesBuffer { is_human_readable },
            };
            Ok(Tagged::Variant(variant, content))
        }
    }
}

/// What reading an internally tagged enum's value found: its tag, beside every other entry, or a value of another kind
/// when type errors are collected.
enum TaggedContent<'de, V> {
    Read {
        tag_found: TagFound<V>,
        content: Content<'de>,
    },
    NotTheEnum(ValueKind),
}

/// What an internally tagged enum's tag says.
enum TagFound<V> {
    Missing,
    NamingNoVariant,
    Naming(V),
}

impl<V> TagFound<V> {
    fn naming(variant: Option<V>) -> Self {
        variant.map_or(Self::NamingNoVariant, Self::Naming)
    }
}

struct TaggedContentVisitor<V> {
    tag: &'static str,
    expecting: &'static str,
    collects_type_errors: bool,
    variant: PhantomData<fn() -> V>,
}

impl<'de, V: serde::Deserialize<'de>> TaggedContentVisitor<V> {
    fn not_the_enum<E: de::Error>(
        self,
        kind: ValueKind,
        unexpected: Unexpected<'_>,
    ) -> Result<TaggedContent<'de, V>, E> {
        if self.collects_type_errors {
            return Ok(TaggedContent::NotTheEnum(kind));
        }
        Err(E::invalid_type(unexpected, &self))
    }

    fn not_the_enum_holding<D: Deserializer<'de>>(
        self,
        held: D,
        unexpected: Unexpected<'_>,
    ) -> Result<TaggedContent<'de, V>, D::Error> {
        if !self.collects_type_errors {
            return Err(D::Error::invalid_type(unexpected, &self));
        }
        let held = <Content as serde::Deserialize>::deserialize(held)?;
        Ok(TaggedContent::NotTheEnum(held.kind()))
    }
}

impl<'de, V: serde::Deserialize<'de>> Visitor<'de> for TaggedContentVisitor<V> {
    type Value = TaggedContent<'de, V>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.expecting)
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
        let mut tag_found = TagFound::Missing;
        let mut entries = Vec::with_capacity(room_for::<(Content, Content)>(map.size_hint()));
        while let Some(key) = map.next_key::<Content<'de>>()? {
            if field_name_of(&key) != Some(self.tag) {
                entries.push((key, map.next_value::<Content<'de>>()?));
                continue;
            }
            if !matches!(tag_found, TagFound::Missing) {
                return Err(A::Error::duplicate_field(self.tag));
            }
            tag_found = TagFound::naming(map.next_value_seed(TagName::<V>::new())?);
        }
        Ok(TaggedContent::Read {
            tag_found,
            content: Content::Map(entries),
        })
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let tag_found = seq
            .next_element_seed(TagName::<V>::new())?
            .map_or(TagFound::Missing, TagFound::naming);
        let mut elements = Vec::with_capacity(room_for::<Content>(seq.size_hint()));
        while let Some(element) = seq.next_element::<Content<'de>>()? {
            elements.push(element);
        }
        Ok(TaggedContent::Read {
            tag_found,
            content: Content::Seq(elements),
        })
    }

    fn visit_bool<E: de::Error>(self, boolean: bool) -> Result<Self::Value, E> {
        self.not_the_enum(ValueKind::Boolean, Unexpected::Bool(boolean))
    }

    fn visit_i64<E: de::Error>(self, integer: i64) -> Result<Self::Value, E> {
        self.not_the_enum(ValueKind::Integer, Unexpected::Signed(integer))
    }

    fn visit_i128<E: de::Error>(self, _: i128) -> Result<Self::Value, E> {
        self.not_the_enum(ValueKind::Integer, Unexpected::Other("integer"))
    }

    fn visit_u64<E: de::Error>(self, integer: u64) -> Result<Self::Value, E> {
        self.not_the_enum(ValueKind::Integer, Unexpected::Unsigned(integer))
    }

    fn visit_u128<E: de::Error>(self, _: u128) -> Result<Self::Value, E> {
        self.not_the_enum(ValueKind::Integer, Unexpected::Other("integer"))
    }

    fn visit_f64<E: de::Error>(self, float: f64) -> Result<Self::Value, E> {
        self.not_the_enum(ValueKind::Float, Unexpected::Float(float))
    }

    fn visit_char<E: de::Error>(self, character: char) -> Result<Self::Value, E> {
        self.not_the_enum(ValueKind::Character, Unexpected::Char(character))
    }

    fn visit_str<E: de::Error>(self, string: &str) -> Result<Self::Value, E> {
        self.not_the_enum(ValueKind::String, Unexpected::Str(string))
    }

    fn visit_bytes<E: de::Error>(self, bytes: &[u8]) -> Result<Self::Value, E> {
        self.not_the_enum(ValueKind::Bytes, Unexpected::Bytes(bytes))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
        self.not_the_enum(ValueKind::Null, Unexpected::Unit)
    }

    fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
        self.not_the_enum(ValueKind::Null, Unexpected::Option)
    }

    fn visit_some<D: Deserializer<'de>>(self, held: D) -> Result<Self::Value, D::Error> {
        self.not_the_enum_holding(held, Unexpected::Option)
    }

    fn visit_newtype_struct<D: Deserializer<'de>>(self, held: D) -> Result<Self::Value, D::Error> {
        self.not_the_enum_holding(held, Unexpected::NewtypeStruct)
    }
}

impl<'de> serde::Deserialize<'de> for Buffered<'de> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let is_human_readable = deserializer.is_human_readable();
        let content = <Content as serde::Deserialize>::deserialize(deserializer)?;
        Ok(Self {
            content,
            read_as: ReadAs::SerdesBuffer { is_human_readable },
        })
    }
}

impl<'de> Buffered<'de> {
    /// The buffered value, read as the format it came from reads it, rather than as serde's buffer: the content of an
    /// adjacently tagged enum collecting type errors, which serde reads from the format when it follows the tag.
    #[must_use]
    pub fn read_as_the_format(self) -> Self {
        Self {
            content: self.content,
            read_as: ReadAs::TheFormat {
                is_human_readable: self.read_as.is_human_readable(),
            },
        }
    }

    /// A deserializer that reads the buffered value without taking it, so it can be read again: each variant of an
    /// untagged enum is tried on it in turn.
    #[must_use]
    pub fn by_reference<E: de::Error>(&self) -> impl Deserializer<'de, Error = E> {
        ContentRefDeserializer::new(&self.content, self.read_as.is_human_readable())
    }

    /// The problem of a buffered value that fits none of an untagged enum's variants: what the enum `expecting` says,
    /// and the kind of value found.
    #[cold]
    #[must_use]
    pub fn fitting_no_variant(&self, expecting: &'static str) -> Problem {
        Problem::InvalidType {
            expected: Cow::Borrowed(expecting),
            found: self.content.kind().name(),
        }
    }

    /// Reads the buffered value with `seed`. A value that doesn't fit is returned as the [`Problem`] to report, instead
    /// of ending deserialization. `type_name` says what was expected when serde's error doesn't.
    ///
    /// The types collecting type errors inside the value read their values where they are in the buffer.
    ///
    /// # Errors
    ///
    /// The format's error `E`, for a key a struct in the value refuses: given twice, or one it denies.
    pub fn read_collecting_type_errors<S, T, E: de::Error>(
        self,
        seed: S,
        type_name: &'static str,
    ) -> Result<Result<Option<T>, Problem>, E>
    where
        S: DeserializeSeed<'de, Value = Option<T>>,
    {
        let kind_found = self.content.kind();
        match seed.deserialize(ContentDeserializer::<TypeError>::new(self.content, self.read_as)) {
            Ok(value) => Ok(Ok(value)),
            Err(error) => problem_or_error(error.into_wrong_type(), type_name, kind_found).map(Err),
        }
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

/// Reads the value of an adjacently tagged enum's tag in a map as the variant `V` it names, or as `None` when it names
/// none, whatever its type, so that the enum can report it and reading can go on. As with serde, the value is a unit
/// variant: a name, or a map of a name to a unit, as the deserializer reads an enum.
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
        variant_named_by(deserializer, UnitVariant::<V>(PhantomData))
    }
}

/// Reads a tag as the name of the variant `V` it names, or as `None` when it names none, whatever its type, so that the
/// enum can report it and reading can go on. As with serde, the name is read as the deserializer reads a name: an
/// internal tag, and an adjacent tag at the head of a sequence.
pub struct TagName<V>(PhantomData<fn() -> V>);

impl<V> TagName<V> {
    /// A seed reading a tag naming a `V`.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<V> Default for TagName<V> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'de, V: serde::Deserialize<'de>> DeserializeSeed<'de> for TagName<V> {
    type Value = Option<V>;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Option<V>, D::Error> {
        variant_named_by(deserializer, PhantomData::<V>)
    }
}

/// Reads a tag with `naming`, as `tag` reads it, or as `None` when `naming` refuses it. A format can't go on past a
/// value it refused, so a tag from a format is buffered first, and read as the format reads it. annotated's buffers
/// read it where it is, as the reader they stand for.
fn variant_named_by<'de, D, S, V>(tag: D, naming: S) -> Result<Option<V>, D::Error>
where
    D: Deserializer<'de>,
    S: DeserializeSeed<'de, Value = V>,
    V: serde::Deserialize<'de>,
{
    if is_annotated_s_buffer::<D>() {
        return Ok(naming.deserialize(tag).ok());
    }
    let read_as = ReadAs::TheFormat {
        is_human_readable: tag.is_human_readable(),
    };
    let tag = <Content as serde::Deserialize>::deserialize(tag)?;
    let is_a_name = matches!(tag, Content::Str(_) | Content::String(_));
    let tag = ContentDeserializer::<WrongType>::new(tag, read_as);
    // A name reads as the variant it names whatever `naming` reads, a unit variant included, so it skips the enum
    // access a unit variant is read through.
    let variant = if is_a_name {
        V::deserialize(tag)
    } else {
        naming.deserialize(tag)
    };
    Ok(variant.ok())
}

/// Reads a unit variant of an enum whose variants `V` names. Only annotated's buffers are asked, which read an enum
/// whatever its name.
struct UnitVariant<V>(PhantomData<fn() -> V>);

impl<'de, V: serde::Deserialize<'de>> DeserializeSeed<'de> for UnitVariant<V> {
    type Value = V;

    fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<V, D::Error> {
        deserializer.deserialize_enum("", &[], self)
    }
}

impl<'de, V: serde::Deserialize<'de>> Visitor<'de> for UnitVariant<V> {
    type Value = V;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a unit variant")
    }

    fn visit_enum<A: EnumAccess<'de>>(self, unit_variant: A) -> Result<V, A::Error> {
        let (variant, content) = unit_variant.variant::<V>()?;
        content.unit_variant()?;
        Ok(variant)
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

/// Reads what is beside an internally tagged unit variant's tag, as serde does: any entries of a map, which it skips,
/// or no element of a sequence.
pub struct InternallyTaggedUnitVisitor {
    enum_name: &'static str,
    variant_name: &'static str,
}

impl InternallyTaggedUnitVisitor {
    /// Reads what is beside the tag of the unit variant `variant_name` of the enum `enum_name`.
    #[must_use]
    pub const fn new(enum_name: &'static str, variant_name: &'static str) -> Self {
        Self {
            enum_name,
            variant_name,
        }
    }
}

impl<'de> Visitor<'de> for InternallyTaggedUnitVisitor {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "unit variant {}::{}", self.enum_name, self.variant_name)
    }

    // An element left in the sequence makes it the wrong length.
    fn visit_seq<A: SeqAccess<'de>>(self, _: A) -> Result<(), A::Error> {
        Ok(())
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<(), A::Error> {
        while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(())
    }
}
