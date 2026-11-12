use std::fmt::{self, Debug, Display};

use crate::{Path, PathSegment};

/// The place in a value that validation has reached. It is kept on the stack while validation descends into the value.
///
/// A location is the root, or a parent location plus one step: a field, an element's index or a map's key. Descending
/// allocates nothing. A location becomes an owned [`Path`] only when a violation is found there.
///
/// ```
/// use annotated::{Location, Path};
///
/// let root = Location::root();
/// let members = root.field("members");
/// let second_member = members.index(1);
///
/// assert_eq!(second_member.field("name").to_path(), Path::root().field("members").index(1).field("name"));
/// ```
#[derive(Clone, Copy)]
pub struct Location<'a> {
    parent_and_step: Option<(&'a Location<'a>, Step<'a>)>,
}

#[derive(Clone, Copy)]
enum Step<'a> {
    Field(&'static str),
    Index(usize),
    Key(&'a dyn Display),
}

impl Location<'static> {
    /// The location of the validated value itself.
    #[must_use]
    pub const fn root() -> Self {
        Self { parent_and_step: None }
    }
}

impl<'a> Location<'a> {
    /// The location of this value's field `name`, which is the name serde reads the field by.
    #[must_use]
    pub const fn field(&'a self, name: &'static str) -> Self {
        self.followed_by(Step::Field(name))
    }

    /// The location of this sequence's element at `index`.
    #[must_use]
    pub const fn index(&'a self, index: usize) -> Self {
        self.followed_by(Step::Index(index))
    }

    /// The location of this map's value at `key`. The key is rendered with [`Display`] if a violation is found there.
    #[must_use]
    pub const fn key(&'a self, key: &'a dyn Display) -> Self {
        self.followed_by(Step::Key(key))
    }

    /// The owned path from the root down to this location.
    #[must_use]
    pub fn to_path(&self) -> Path {
        match self.parent_and_step {
            None => Path::root(),
            Some((parent, step)) => parent.to_path().followed_by(step.to_segment()),
        }
    }

    /// Whether `path` is the path of this location or of a location under it.
    pub(crate) fn leads_to(&self, path: &Path) -> bool {
        self.segments_matched_at_the_start_of(path.segments()).is_some()
    }

    fn segments_matched_at_the_start_of(&self, segments: &[PathSegment]) -> Option<usize> {
        let Some((parent, step)) = self.parent_and_step else {
            return Some(0);
        };
        let matched_by_parent = parent.segments_matched_at_the_start_of(segments)?;
        let segment = segments.get(matched_by_parent)?;
        step.is(segment).then_some(matched_by_parent + 1)
    }

    const fn followed_by(&'a self, step: Step<'a>) -> Self {
        Self {
            parent_and_step: Some((self, step)),
        }
    }
}

impl Step<'_> {
    fn is(&self, segment: &PathSegment) -> bool {
        match (self, segment) {
            (Self::Field(name), PathSegment::Field(segment_name)) => name == segment_name,
            (Self::Index(index), PathSegment::Index(segment_index)) => index == segment_index,
            (Self::Key(key), PathSegment::Key(rendered_key)) => renders_as(*key, rendered_key),
            _ => false,
        }
    }

    fn to_segment(self) -> PathSegment {
        match self {
            Self::Field(name) => PathSegment::Field(name),
            Self::Index(index) => PathSegment::Index(index),
            Self::Key(key) => PathSegment::Key(key.to_string()),
        }
    }
}

fn renders_as(key: &dyn Display, rendered: &str) -> bool {
    struct RenderingCompared<'a> {
        rest: &'a str,
    }

    impl fmt::Write for RenderingCompared<'_> {
        fn write_str(&mut self, written: &str) -> fmt::Result {
            self.rest = self.rest.strip_prefix(written).ok_or(fmt::Error)?;
            Ok(())
        }
    }

    let mut comparison = RenderingCompared { rest: rendered };
    fmt::write(&mut comparison, format_args!("{key}")).is_ok() && comparison.rest.is_empty()
}

impl Debug for Location<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_tuple("Location").field(&self.to_path()).finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_root_location_is_the_root_path() {
        assert_eq!(Location::root().to_path(), Path::root());
    }

    #[test]
    fn a_location_becomes_the_path_from_the_root_down_to_it() {
        let root = Location::root();
        let members = root.field("members");
        let second_member = members.index(1);

        let path = second_member.field("name").to_path();

        assert_eq!(path, Path::root().field("members").index(1).field("name"));
    }

    #[test]
    fn a_key_is_rendered_when_its_location_becomes_a_path() {
        let root = Location::root();
        let labels = root.field("labels");

        let path = labels.key(&"en").to_path();

        assert_eq!(
            path.segments(),
            [PathSegment::Field("labels"), PathSegment::Key("en".to_owned())]
        );
    }

    #[test]
    fn a_location_leads_to_its_own_path() {
        let root = Location::root();
        let stock = root.field("stock");

        assert!(stock.key(&"sku-1").leads_to(&Path::root().field("stock").key("sku-1")));
    }

    #[test]
    fn a_location_leads_to_the_paths_under_it() {
        let root = Location::root();
        let members = root.field("members");

        assert!(
            members
                .index(1)
                .leads_to(&Path::root().field("members").index(1).field("name"))
        );
    }

    #[test]
    fn a_location_does_not_lead_to_the_path_of_its_parent() {
        let root = Location::root();
        let members = root.field("members");

        assert!(!members.index(1).leads_to(&Path::root().field("members")));
    }

    #[test]
    fn a_location_does_not_lead_to_a_path_through_another_field() {
        let root = Location::root();
        let members = root.field("members");

        assert!(!members.index(1).leads_to(&Path::root().field("guests").index(1)));
    }

    #[test]
    fn a_location_does_not_lead_to_a_path_through_another_index() {
        let root = Location::root();
        let members = root.field("members");

        assert!(!members.index(1).leads_to(&Path::root().field("members").index(2)));
    }

    #[test]
    fn a_location_does_not_lead_to_a_path_through_another_key() {
        let root = Location::root();
        let stock = root.field("stock");

        assert!(!stock.key(&"sku-1").leads_to(&Path::root().field("stock").key("sku-2")));
    }

    #[test]
    fn a_location_does_not_lead_to_a_path_through_a_key_its_own_key_starts() {
        let root = Location::root();
        let stock = root.field("stock");

        assert!(!stock.key(&"sku-1").leads_to(&Path::root().field("stock").key("sku-10")));
    }

    #[test]
    fn a_location_does_not_lead_to_a_path_with_an_index_where_it_has_a_key() {
        let root = Location::root();
        let shifts = root.field("shifts");

        assert!(!shifts.key(&0).leads_to(&Path::root().field("shifts").index(0)));
    }

    #[test]
    fn a_location_deeper_than_three_steps_becomes_the_path_from_the_root_down_to_it() {
        let root = Location::root();
        let orders = root.field("orders");
        let first_order = orders.index(0);
        let lines = first_order.field("lines");
        let third_line = lines.index(2);

        let path = third_line.field("sku").to_path();

        assert_eq!(
            path.segments(),
            [
                PathSegment::Field("orders"),
                PathSegment::Index(0),
                PathSegment::Field("lines"),
                PathSegment::Index(2),
                PathSegment::Field("sku"),
            ]
        );
    }
}
