use std::borrow::Cow;
use std::fmt::{self, Display};

/// What kind of violation happened: a stable code, its named parameters and an English message.
///
/// Code that translates messages or reacts to a violation reads [`Problem::code`] and [`Problem::params`];
/// people read its [`Display`], the English message. New kinds of problem may be added without a breaking
/// change, so a `match` on it needs a wildcard arm.
///
/// A rule of your own reports with [`Problem::custom`]:
///
/// ```
/// use annotated::Problem;
///
/// let problem = Problem::custom("reserved_name", "is reserved").with_param("name", "admin");
///
/// assert_eq!(problem.code(), "reserved_name");
/// assert_eq!(problem.to_string(), "is reserved");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Problem {
    /// A required value is missing: `required`, code `required`.
    Required,
    /// A length is out of bounds: `length`, code `length`, with the params `min` and `max` it has, `equal`
    /// when they are the same, and `unit`.
    #[non_exhaustive]
    Length {
        /// The lengths accepted.
        bounds: Bounds<usize>,
        /// What the length was counted in.
        unit: LengthUnit,
    },
    /// A problem reported by a rule of your own, made with [`Problem::custom`].
    #[non_exhaustive]
    Custom {
        /// The stable code it is known by.
        code: &'static str,
        /// The English message for people.
        message: Cow<'static, str>,
        /// The named parameters, in the order they were added.
        params: Vec<(&'static str, String)>,
    },
}

/// The values a rule accepts, inclusive at both ends.
///
/// They read as `at least 1`, `at most 50`, `between 1 and 50`, or `exactly 5` when both ends are the same.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bounds<T> {
    /// No less than this.
    AtLeast(T),
    /// No more than this.
    AtMost(T),
    /// No less than `min` and no more than `max`.
    Between {
        /// The least value accepted.
        min: T,
        /// The greatest value accepted.
        max: T,
    },
}

impl<T> Bounds<T> {
    fn farthest(&self) -> &T {
        match self {
            Self::AtLeast(farthest) | Self::AtMost(farthest) | Self::Between { max: farthest, .. } => farthest,
        }
    }

    pub(crate) fn of(min: Option<T>, max: Option<T>) -> Option<Self> {
        match (min, max) {
            (Some(min), Some(max)) => Some(Self::Between { min, max }),
            (Some(min), None) => Some(Self::AtLeast(min)),
            (None, Some(max)) => Some(Self::AtMost(max)),
            (None, None) => None,
        }
    }
}

impl<T: Display> Bounds<T> {
    fn params(&self) -> Vec<(&'static str, String)> {
        match self {
            Self::AtLeast(min) => vec![("min", min.to_string())],
            Self::AtMost(max) => vec![("max", max.to_string())],
            Self::Between { min, max } => vec![("min", min.to_string()), ("max", max.to_string())],
        }
    }
}

impl<T: Display + PartialEq> Display for Bounds<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AtLeast(min) => write!(formatter, "at least {min}"),
            Self::AtMost(max) => write!(formatter, "at most {max}"),
            Self::Between { min, max } if min == max => write!(formatter, "exactly {min}"),
            Self::Between { min, max } => write!(formatter, "between {min} and {max}"),
        }
    }
}

/// What a length is counted in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LengthUnit {
    /// A string's `char`s, its Unicode scalar values.
    Chars,
    /// A string's UTF-8 bytes.
    Bytes,
    /// A string's grapheme clusters, what a person sees as one character.
    Graphemes,
    /// A collection's elements, or a map's entries.
    Elements,
}

impl LengthUnit {
    fn name(self) -> &'static str {
        match self {
            Self::Chars => "chars",
            Self::Bytes => "bytes",
            Self::Graphemes => "graphemes",
            Self::Elements => "elements",
        }
    }

    fn counted_in(self, count: usize) -> &'static str {
        let is_singular = count == 1;
        match self {
            Self::Chars | Self::Graphemes if is_singular => "character",
            Self::Chars | Self::Graphemes => "characters",
            Self::Bytes if is_singular => "byte",
            Self::Bytes => "bytes",
            Self::Elements if is_singular => "element",
            Self::Elements => "elements",
        }
    }
}

impl Problem {
    /// A problem of a rule of your own, known by the stable `code` and explained to people by `message`.
    #[must_use]
    pub fn custom(code: &'static str, message: impl Into<Cow<'static, str>>) -> Self {
        Self::Custom {
            code,
            message: message.into(),
            params: Vec::new(),
        }
    }

    /// This problem with the named parameter `name` added, rendered with [`Display`], for messages to quote
    /// and code to read.
    ///
    /// Only a [custom](Problem::custom) problem takes parameters; a built-in problem's parameters are fixed,
    /// and it is returned unchanged.
    #[must_use]
    pub fn with_param(mut self, name: &'static str, value: impl Display) -> Self {
        if let Self::Custom { params, .. } = &mut self {
            params.push((name, value.to_string()));
        }
        self
    }

    /// The stable code this kind of problem is known by, in `snake_case`.
    #[must_use]
    pub fn code(&self) -> &str {
        match self {
            Self::Required => "required",
            Self::Length { .. } => "length",
            Self::Custom { code, .. } => code,
        }
    }

    /// The named parameters of this problem, such as a length's `max`.
    #[must_use]
    pub fn params(&self) -> Vec<(&'static str, String)> {
        match self {
            Self::Required => Vec::new(),
            Self::Length { bounds, unit } => {
                let mut params = bounds.params();
                if let Bounds::Between { min, max } = bounds
                    && min == max
                {
                    params.push(("equal", min.to_string()));
                }
                params.push(("unit", unit.name().to_owned()));
                params
            }
            Self::Custom { params, .. } => params.clone(),
        }
    }
}

impl Display for Problem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Required => formatter.write_str("is required"),
            Self::Length { bounds, unit } => {
                let counted_in = unit.counted_in(*bounds.farthest());
                match unit {
                    LengthUnit::Elements => write!(formatter, "must have {bounds} {counted_in}"),
                    LengthUnit::Chars | LengthUnit::Bytes | LengthUnit::Graphemes => {
                        write!(formatter, "must be {bounds} {counted_in} long")
                    }
                }
            }
            Self::Custom { message, .. } => formatter.write_str(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_custom_problem_is_known_by_its_code() {
        let problem = Problem::custom("reserved_name", "is reserved");

        assert_eq!(problem.code(), "reserved_name");
    }

    #[test]
    fn a_custom_problem_reads_as_its_message() {
        let problem = Problem::custom("reserved_name", "is reserved");

        assert_eq!(problem.to_string(), "is reserved");
    }

    #[test]
    fn a_custom_problem_has_no_params_until_given_some() {
        let problem = Problem::custom("reserved_name", "is reserved");

        assert_eq!(problem.params(), []);
    }

    #[test]
    fn a_custom_problem_answers_its_params_in_the_order_they_were_added() {
        let problem = Problem::custom("reserved_name", "is reserved")
            .with_param("name", "admin")
            .with_param("reserved_names", 3_usize);

        assert_eq!(
            problem.params(),
            [("name", "admin".to_owned()), ("reserved_names", "3".to_owned()),]
        );
    }

    #[test]
    fn a_built_in_problem_takes_no_params_of_yours() {
        let problem = Problem::Required.with_param("name", "admin");

        assert_eq!(problem.params(), []);
    }

    #[test]
    fn a_missing_required_value_is_known_as_required() {
        assert_eq!(Problem::Required.code(), "required");
    }

    #[test]
    fn a_missing_required_value_reads_as_required() {
        assert_eq!(Problem::Required.to_string(), "is required");
    }

    fn length_problem(bounds: Bounds<usize>, unit: LengthUnit) -> Problem {
        Problem::Length { bounds, unit }
    }

    #[test]
    fn a_length_out_of_bounds_is_known_as_length() {
        assert_eq!(length_problem(Bounds::AtMost(50), LengthUnit::Chars).code(), "length");
    }

    #[test]
    fn a_string_too_long_reads_as_its_max_in_characters() {
        let problem = length_problem(Bounds::AtMost(50), LengthUnit::Chars);

        assert_eq!(problem.to_string(), "must be at most 50 characters long");
    }

    #[test]
    fn a_string_too_short_reads_as_its_min_in_characters() {
        let problem = length_problem(Bounds::AtLeast(2), LengthUnit::Chars);

        assert_eq!(problem.to_string(), "must be at least 2 characters long");
    }

    #[test]
    fn a_length_of_one_reads_in_the_singular() {
        let problem = length_problem(Bounds::AtLeast(1), LengthUnit::Chars);

        assert_eq!(problem.to_string(), "must be at least 1 character long");
    }

    #[test]
    fn a_length_between_bounds_reads_as_both() {
        let problem = length_problem(Bounds::Between { min: 1, max: 50 }, LengthUnit::Chars);

        assert_eq!(problem.to_string(), "must be between 1 and 50 characters long");
    }

    #[test]
    fn a_length_with_equal_bounds_reads_as_exact() {
        let problem = length_problem(Bounds::Between { min: 5, max: 5 }, LengthUnit::Chars);

        assert_eq!(problem.to_string(), "must be exactly 5 characters long");
    }

    #[test]
    fn a_length_in_bytes_reads_in_bytes() {
        let problem = length_problem(Bounds::AtMost(50), LengthUnit::Bytes);

        assert_eq!(problem.to_string(), "must be at most 50 bytes long");
    }

    #[test]
    fn a_length_in_graphemes_reads_in_characters() {
        let problem = length_problem(Bounds::AtMost(50), LengthUnit::Graphemes);

        assert_eq!(problem.to_string(), "must be at most 50 characters long");
    }

    #[test]
    fn a_collection_s_length_reads_in_elements() {
        let problem = length_problem(Bounds::AtMost(3), LengthUnit::Elements);

        assert_eq!(problem.to_string(), "must have at most 3 elements");
    }

    #[test]
    fn a_collection_s_length_of_one_reads_in_the_singular() {
        let problem = length_problem(Bounds::AtLeast(1), LengthUnit::Elements);

        assert_eq!(problem.to_string(), "must have at least 1 element");
    }

    #[test]
    fn a_length_s_params_are_its_bounds_and_unit() {
        let problem = length_problem(Bounds::Between { min: 1, max: 50 }, LengthUnit::Bytes);

        assert_eq!(
            problem.params(),
            [
                ("min", "1".to_owned()),
                ("max", "50".to_owned()),
                ("unit", "bytes".to_owned()),
            ]
        );
    }

    #[test]
    fn a_length_with_only_a_max_has_no_min_param() {
        let problem = length_problem(Bounds::AtMost(50), LengthUnit::Chars);

        assert_eq!(
            problem.params(),
            [("max", "50".to_owned()), ("unit", "chars".to_owned()),]
        );
    }

    #[test]
    fn a_length_with_equal_bounds_has_an_equal_param_too() {
        let problem = length_problem(Bounds::Between { min: 5, max: 5 }, LengthUnit::Elements);

        assert_eq!(
            problem.params(),
            [
                ("min", "5".to_owned()),
                ("max", "5".to_owned()),
                ("equal", "5".to_owned()),
                ("unit", "elements".to_owned()),
            ]
        );
    }
}
