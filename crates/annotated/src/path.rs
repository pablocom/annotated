use std::borrow::Cow;
use std::fmt::{self, Debug, Display, Write};
use std::hash::{Hash, Hasher};

/// Where in a value an issue was found: the segments that lead from the validated value down to it.
///
/// The root path has no segments and stands for the validated value itself. An issue a struct-level rule finds on that
/// value has the root path. [`Display`] prints a path in dotted form, and [`Path::json_pointer`] returns it as an RFC
/// 6901 JSON Pointer.
///
/// A map's key comes from the input, so [`Display`] prints it with `\` and every char that could break a line or hide
/// text escaped, as [`str::escape_debug`] escapes them: a newline is printed `\n` and U+202E `\u{202e}`. Quotes and
/// other printable chars are printed as they are. So a key can't forge another line of [`Issues`](crate::Issues).
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
    ///
    /// A key in a path is at most [`MAX_KEY_CHARS`](Self::MAX_KEY_CHARS) `char`s: a longer one is cut to its first
    /// that many, followed by `…`. So an issue holds a bounded copy of its key, however long the key in the input is.
    ///
    /// The segment's [`Display`] prints the key escaped, as a [`Path`] does.
    Key(String),
}

impl PathSegment {
    /// The most `char`s of a map's key that a path keeps. A longer key is cut to its first 256 `char`s followed by
    /// `…`.
    pub const MAX_KEY_CHARS: usize = 256;

    pub(crate) fn key(key: &dyn Display) -> Self {
        let mut rendered = String::new();
        write_cut_key(key, &mut rendered).expect("a Display implementation returned an error unexpectedly");
        Self::Key(rendered)
    }

    fn text(&self) -> Cow<'_, str> {
        match self {
            Self::Field(name) => Cow::Borrowed(name),
            Self::Index(index) => Cow::Owned(index.to_string()),
            Self::Key(key) => Cow::Borrowed(key),
        }
    }
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
    ///
    /// A key longer than [`PathSegment::MAX_KEY_CHARS`] `char`s is cut as in an issue's path, so this path equals the
    /// path of an issue found under the same key.
    #[must_use]
    pub fn key(self, key: impl Display) -> Self {
        self.followed_by(PathSegment::key(&key))
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
    /// The root path is the empty pointer, `""`. A `~` in a segment is written `~0` and a `/` is written `~1`. A map's
    /// key is otherwise kept as it is, unlike in [`Display`]: a newline in it stays a newline.
    #[must_use]
    pub fn json_pointer(&self) -> String {
        let mut pointer = String::new();
        for segment in self.segments() {
            pointer.push('/');
            pointer.push_str(&segment.text().replace('~', "~0").replace('/', "~1"));
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
            Self::Key(key) => write_escaped(key, formatter),
        }
    }
}

/// Writes `key` with `\` and every char that could break a line or hide text escaped, as
/// [`str::escape_debug`] escapes them, and its quotes as they are.
fn write_escaped(key: &str, written_to: &mut impl Write) -> fmt::Result {
    const QUOTES: [char; 2] = ['\'', '"'];
    for text_then_quote in key.split_inclusive(QUOTES) {
        let text = text_then_quote.strip_suffix(QUOTES).unwrap_or(text_then_quote);
        write!(written_to, "{}", text.escape_debug())?;
        written_to.write_str(&text_then_quote[text.len()..])?;
    }
    Ok(())
}

/// Writes how `key` renders, cut to its first [`PathSegment::MAX_KEY_CHARS`] `char`s followed by `…` when it is
/// longer. Rendering stops there, so a long key is never rendered whole.
pub(crate) fn write_cut_key(key: &dyn Display, written_to: &mut impl Write) -> fmt::Result {
    let mut first_chars = FirstChars {
        written_to,
        chars_left: PathSegment::MAX_KEY_CHARS,
        is_cut: false,
    };
    let rendering = fmt::write(&mut first_chars, format_args!("{key}"));
    if first_chars.is_cut {
        return first_chars.written_to.write_char('…');
    }
    rendering
}

struct FirstChars<'a, W> {
    written_to: &'a mut W,
    chars_left: usize,
    is_cut: bool,
}

impl<W: Write> Write for FirstChars<'_, W> {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        if let Some((end_of_first_chars, _)) = text.char_indices().nth(self.chars_left) {
            self.written_to.write_str(&text[..end_of_first_chars])?;
            self.chars_left = 0;
            self.is_cut = true;
            return Err(fmt::Error);
        }
        self.written_to.write_str(text)?;
        self.chars_left -= text.chars().count();
        Ok(())
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
