use std::fmt::{self, Debug, Display};
use std::hash::{Hash, Hasher};

/// Where in a value a violation was found: the segments that lead from the validated value down to it.
///
/// The root path has no segments and stands for the validated value itself. A violation of a struct-level rule on that
/// value has the root path. [`Display`] prints a path in dotted form, and [`Path::json_pointer`] returns it as an RFC
/// 6901 JSON Pointer.
///
/// ```
/// use annotated::Path;
///
/// let path = Path::root().field("members").index(1).field("name");
///
/// assert_eq!(path.to_string(), "members[1].name");
/// assert_eq!(path.json_pointer(), "/members/1/name");
/// ```
#[derive(Clone, Default)]
pub struct Path {
    segments: Segments,
}

#[derive(Clone, Default)]
enum Segments {
    #[default]
    None,
    One([PathSegment; 1]),
    Two([PathSegment; 2]),
    Three([PathSegment; 3]),
    More(Vec<PathSegment>),
}

/// One step of a [`Path`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum PathSegment {
    /// A struct's field, under the name serde reads it by.
    Field(&'static str),
    /// An element of a sequence, by its position, counting from zero.
    Index(usize),
    /// A value of a map, by its key rendered with [`Display`].
    Key(String),
}

impl Path {
    /// The path of the validated value itself, with no segments.
    #[must_use]
    pub const fn root() -> Self {
        Self {
            segments: Segments::None,
        }
    }

    /// This path followed by the field `name`.
    #[must_use]
    pub fn field(self, name: &'static str) -> Self {
        self.followed_by(PathSegment::Field(name))
    }

    /// This path followed by the element at `index`.
    #[must_use]
    pub fn index(self, index: usize) -> Self {
        self.followed_by(PathSegment::Index(index))
    }

    /// This path followed by the map value at `key`, rendered with [`Display`].
    #[must_use]
    pub fn key(self, key: impl Display) -> Self {
        self.followed_by(PathSegment::Key(key.to_string()))
    }

    /// The segments from the root down.
    #[must_use]
    pub fn segments(&self) -> &[PathSegment] {
        self.segments.as_slice()
    }

    /// Whether this is the root path, with no segments.
    #[must_use]
    pub fn is_root(&self) -> bool {
        matches!(self.segments, Segments::None)
    }

    /// The path as an [RFC 6901](https://www.rfc-editor.org/rfc/rfc6901) JSON Pointer: `/members/1/name`.
    ///
    /// The root path is the empty pointer, `""`. A `~` in a segment is written `~0` and a `/` is written `~1`.
    #[must_use]
    pub fn json_pointer(&self) -> String {
        let mut pointer = String::new();
        for segment in self.segments() {
            pointer.push('/');
            pointer.push_str(&segment.to_string().replace('~', "~0").replace('/', "~1"));
        }
        pointer
    }

    pub(crate) fn followed_by(self, segment: PathSegment) -> Self {
        Self {
            segments: self.segments.followed_by(segment),
        }
    }
}

impl Segments {
    fn followed_by(self, segment: PathSegment) -> Self {
        match self {
            Self::None => Self::One([segment]),
            Self::One([first]) => Self::Two([first, segment]),
            Self::Two([first, second]) => Self::Three([first, second, segment]),
            Self::Three(three) => Self::More(three.into_iter().chain([segment]).collect()),
            Self::More(mut more) => {
                more.push(segment);
                Self::More(more)
            }
        }
    }

    fn as_slice(&self) -> &[PathSegment] {
        match self {
            Self::None => &[],
            Self::One(one) => one,
            Self::Two(two) => two,
            Self::Three(three) => three,
            Self::More(more) => more,
        }
    }
}

impl PartialEq for Path {
    fn eq(&self, other: &Self) -> bool {
        self.segments() == other.segments()
    }
}

impl Eq for Path {}

impl Hash for Path {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.segments().hash(state);
    }
}

impl Debug for Path {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Path")
            .field("segments", &self.segments())
            .finish()
    }
}

impl Display for Path {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (position, segment) in self.segments().iter().enumerate() {
            match segment {
                PathSegment::Field(name) if position == 0 => formatter.write_str(name)?,
                PathSegment::Field(name) => write!(formatter, ".{name}")?,
                PathSegment::Index(_) | PathSegment::Key(_) => write!(formatter, "[{segment}]")?,
            }
        }
        Ok(())
    }
}

impl Display for PathSegment {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Field(name) => formatter.write_str(name),
            Self::Index(index) => write!(formatter, "{index}"),
            Self::Key(key) => formatter.write_str(key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_root_path_renders_as_nothing() {
        assert_eq!(Path::root().to_string(), "");
    }

    #[test]
    fn fields_render_joined_by_dots() {
        let path = Path::root().field("address").field("postalCode");

        assert_eq!(path.to_string(), "address.postalCode");
    }

    #[test]
    fn an_index_renders_in_brackets_after_its_sequence() {
        let path = Path::root().field("members").index(1).field("name");

        assert_eq!(path.to_string(), "members[1].name");
    }

    #[test]
    fn a_key_renders_in_brackets_after_its_map() {
        let path = Path::root().field("labels").key("en");

        assert_eq!(path.to_string(), "labels[en]");
    }

    #[test]
    fn a_path_starting_with_an_index_renders_without_a_leading_dot() {
        let path = Path::root().index(0).field("name");

        assert_eq!(path.to_string(), "[0].name");
    }

    #[test]
    fn a_key_is_kept_as_its_display_rendering() {
        let path = Path::root().key(42);

        assert_eq!(path.segments(), [PathSegment::Key("42".to_owned())]);
    }

    #[test]
    fn the_root_path_is_the_empty_json_pointer() {
        assert_eq!(Path::root().json_pointer(), "");
    }

    #[test]
    fn a_json_pointer_starts_every_segment_with_a_slash() {
        let path = Path::root().field("members").index(1).field("name");

        assert_eq!(path.json_pointer(), "/members/1/name");
    }

    #[test]
    fn a_json_pointer_escapes_tildes_and_slashes() {
        let path = Path::root().field("labels").key("a/b~c");

        assert_eq!(path.json_pointer(), "/labels/a~1b~0c");
    }

    #[test]
    fn only_the_path_without_segments_is_the_root() {
        assert_eq!((Path::root().is_root(), Path::root().index(0).is_root()), (true, false));
    }

    #[test]
    fn paths_with_different_segments_are_different() {
        assert_ne!(Path::root().field("name"), Path::root().field("email"));
    }

    #[test]
    fn a_path_deeper_than_three_segments_renders_every_segment() {
        let path = Path::root()
            .field("orders")
            .index(0)
            .field("lines")
            .index(2)
            .field("sku");

        assert_eq!(path.to_string(), "orders[0].lines[2].sku");
    }

    #[test]
    fn a_path_debugs_as_its_segments() {
        let path = Path::root().field("members").index(1);

        assert_eq!(
            format!("{path:?}"),
            r#"Path { segments: [Field("members"), Index(1)] }"#
        );
    }
}
