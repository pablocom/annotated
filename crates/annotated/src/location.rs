use std::fmt::{self, Debug, Display};

use crate::{Path, PathSegment};

/// Where validation is in a value, kept on the stack while validation descends into it.
///
/// A location is the root, or a parent location plus one step: a field, an element's index or a map's key.
/// Descending costs nothing; a location becomes an owned [`Path`] only when a violation is found there.
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
    /// The location of this value's field `name`, by the name serde reads it by.
    #[must_use]
    pub const fn field(&'a self, name: &'static str) -> Self {
        self.followed_by(Step::Field(name))
    }

    /// The location of this sequence's element at `index`.
    #[must_use]
    pub const fn index(&'a self, index: usize) -> Self {
        self.followed_by(Step::Index(index))
    }

    /// The location of this map's value at `key`, rendered with [`Display`] if a violation is found there.
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

    const fn followed_by(&'a self, step: Step<'a>) -> Self {
        Self {
            parent_and_step: Some((self, step)),
        }
    }
}

impl Step<'_> {
    fn to_segment(self) -> PathSegment {
        match self {
            Self::Field(name) => PathSegment::Field(name),
            Self::Index(index) => PathSegment::Index(index),
            Self::Key(key) => PathSegment::Key(key.to_string()),
        }
    }
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
