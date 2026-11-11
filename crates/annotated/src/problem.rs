use std::borrow::Cow;
use std::fmt::{self, Display};

use crate::ParamValue;

/// What kind of violation happened: a stable code, its named parameters and an English message.
///
/// Code that translates messages or reacts to a violation reads [`Problem::code`] and [`Problem::params`], each
/// param a [`ParamValue`], a number or a text;
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
#[derive(Debug, Clone, PartialEq)]
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
    /// A value is out of bounds: `range`, code `range`, with the params `min` and `max` it has.
    #[non_exhaustive]
    Range {
        /// The values accepted: numbers when the value checked is of a primitive number type, or else text,
        /// rendered with [`Display`].
        bounds: Bounds<ParamValue>,
    },
    /// A string isn't an email address: `email`, code `email`.
    Email,
    /// A string isn't a URL of an accepted scheme: `url`, code `url`, with the param `schemes`, comma
    /// separated: `http, https, ftp` unless the rule was given others.
    #[non_exhaustive]
    Url {
        /// The schemes accepted: `http`, `https` and `ftp`, or those the rule was given.
        schemes: &'static [&'static str],
    },
    /// A string isn't an IP address of either version: `ip`, code `ip`.
    Ip,
    /// A string isn't an IPv4 address: `ipv4`, code `ipv4`.
    Ipv4,
    /// A string isn't an IPv6 address: `ipv6`, code `ipv6`.
    Ipv6,
    /// A string doesn't match a regular expression: `pattern`, code `pattern`, with the param `pattern`.
    #[non_exhaustive]
    Pattern {
        /// The regular expression it must match.
        pattern: &'static str,
    },
    /// A string doesn't contain a text: `contains`, code `contains`, with the param `text`.
    #[non_exhaustive]
    Contains {
        /// The text it must contain.
        text: &'static str,
    },
    /// A string doesn't start with a prefix: `prefix`, code `prefix`, with the param `prefix`.
    #[non_exhaustive]
    Prefix {
        /// The text it must start with.
        prefix: &'static str,
    },
    /// A string doesn't end with a suffix: `suffix`, code `suffix`, with the param `suffix`.
    #[non_exhaustive]
    Suffix {
        /// The text it must end with.
        suffix: &'static str,
    },
    /// A problem reported by a rule of your own, made with [`Problem::custom`].
    #[non_exhaustive]
    Custom {
        /// The stable code it is known by.
        code: &'static str,
        /// The English message for people.
        message: Cow<'static, str>,
        /// The named parameters, in the order they were added.
        params: Vec<(&'static str, ParamValue)>,
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

fn either_of(choices: &[&str]) -> String {
    match choices {
        [] => String::new(),
        [only] => (*only).to_owned(),
        [all_but_last @ .., last] => format!("{} or {last}", all_but_last.join(", ")),
    }
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

impl<T: Clone + Into<ParamValue>> Bounds<T> {
    fn params(&self) -> Vec<(&'static str, ParamValue)> {
        match self {
            Self::AtLeast(min) => vec![("min", min.clone().into())],
            Self::AtMost(max) => vec![("max", max.clone().into())],
            Self::Between { min, max } => vec![("min", min.clone().into()), ("max", max.clone().into())],
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

    /// This problem with the named parameter `name` added, for messages to quote and code to read: a number,
    /// like `50` or `0.5`, or a text, like `"admin"`.
    ///
    /// Only a [custom](Problem::custom) problem takes parameters; a built-in problem's parameters are fixed,
    /// and it is returned unchanged.
    #[must_use]
    pub fn with_param(mut self, name: &'static str, value: impl Into<ParamValue>) -> Self {
        if let Self::Custom { params, .. } = &mut self {
            params.push((name, value.into()));
        }
        self
    }

    /// The stable code this kind of problem is known by, in `snake_case`.
    #[must_use]
    pub fn code(&self) -> &str {
        match self {
            Self::Required => "required",
            Self::Length { .. } => "length",
            Self::Range { .. } => "range",
            Self::Email => "email",
            Self::Url { .. } => "url",
            Self::Ip => "ip",
            Self::Ipv4 => "ipv4",
            Self::Ipv6 => "ipv6",
            Self::Pattern { .. } => "pattern",
            Self::Contains { .. } => "contains",
            Self::Prefix { .. } => "prefix",
            Self::Suffix { .. } => "suffix",
            Self::Custom { code, .. } => code,
        }
    }

    /// The named parameters of this problem, such as a length's `max`, each a number or a text.
    #[must_use]
    pub fn params(&self) -> Vec<(&'static str, ParamValue)> {
        match self {
            Self::Length { bounds, unit } => {
                let mut params = bounds.params();
                if let Bounds::Between { min, max } = bounds
                    && min == max
                {
                    params.push(("equal", ParamValue::from(*min)));
                }
                params.push(("unit", ParamValue::text(unit.name())));
                params
            }
            Self::Range { bounds } => bounds.params(),
            Self::Required | Self::Email | Self::Ip | Self::Ipv4 | Self::Ipv6 => Vec::new(),
            Self::Url { schemes } => vec![("schemes", ParamValue::from(schemes.join(", ")))],
            Self::Pattern { pattern } => vec![("pattern", ParamValue::text(pattern))],
            Self::Contains { text } => vec![("text", ParamValue::text(text))],
            Self::Prefix { prefix } => vec![("prefix", ParamValue::text(prefix))],
            Self::Suffix { suffix } => vec![("suffix", ParamValue::text(suffix))],
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
            Self::Range { bounds } => write!(formatter, "must be {bounds}"),
            Self::Email => formatter.write_str("must be an email address"),
            Self::Url { schemes } => write!(formatter, "must be a URL using {}", either_of(schemes)),
            Self::Ip => formatter.write_str("must be an IP address"),
            Self::Ipv4 => formatter.write_str("must be an IPv4 address"),
            Self::Ipv6 => formatter.write_str("must be an IPv6 address"),
            Self::Pattern { pattern } => write!(formatter, "must match the pattern {pattern}"),
            Self::Contains { text } => write!(formatter, "must contain {text:?}"),
            Self::Prefix { prefix } => write!(formatter, "must start with {prefix:?}"),
            Self::Suffix { suffix } => write!(formatter, "must end with {suffix:?}"),
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
            [
                ("name", ParamValue::Text("admin".into())),
                ("reserved_names", ParamValue::Integer(3)),
            ]
        );
    }

    #[test]
    fn a_custom_problem_takes_a_float_param() {
        let problem = Problem::custom("too_risky", "is too risky").with_param("max_ratio", 0.5);

        assert_eq!(problem.params(), [("max_ratio", ParamValue::Float(0.5))]);
    }

    fn range_problem(bounds: Bounds<ParamValue>) -> Problem {
        Problem::Range { bounds }
    }

    #[test]
    fn a_value_out_of_range_is_known_as_range() {
        assert_eq!(range_problem(Bounds::AtLeast(ParamValue::Integer(18))).code(), "range");
    }

    #[test]
    fn a_value_below_its_only_bound_reads_as_its_min() {
        assert_eq!(
            range_problem(Bounds::AtLeast(ParamValue::Integer(18))).to_string(),
            "must be at least 18"
        );
    }

    #[test]
    fn a_value_above_its_only_bound_reads_as_its_max() {
        assert_eq!(
            range_problem(Bounds::AtMost(ParamValue::Float(0.5))).to_string(),
            "must be at most 0.5"
        );
    }

    #[test]
    fn a_value_out_of_two_bounds_reads_as_both() {
        assert_eq!(
            range_problem(Bounds::Between {
                min: ParamValue::Integer(18),
                max: ParamValue::Integer(120)
            })
            .to_string(),
            "must be between 18 and 120"
        );
    }

    #[test]
    fn a_range_s_params_are_its_bounds() {
        assert_eq!(
            range_problem(Bounds::Between {
                min: ParamValue::Integer(18),
                max: ParamValue::Integer(120)
            })
            .params(),
            [("min", ParamValue::Integer(18)), ("max", ParamValue::Integer(120))]
        );
    }

    #[test]
    fn a_string_that_is_not_an_email_address_is_known_as_email() {
        assert_eq!(Problem::Email.code(), "email");
    }

    #[test]
    fn a_string_that_is_not_an_email_address_reads_as_such() {
        assert_eq!(Problem::Email.to_string(), "must be an email address");
    }

    #[test]
    fn a_string_that_is_not_a_url_is_known_as_url() {
        assert_eq!(Problem::Url { schemes: &["https"] }.code(), "url");
    }

    #[test]
    fn a_string_that_is_not_a_url_reads_as_the_schemes_accepted() {
        let problem = Problem::Url {
            schemes: &["ftp", "http", "https"],
        };

        assert_eq!(problem.to_string(), "must be a URL using ftp, http or https");
    }

    #[test]
    fn a_string_that_is_not_a_url_of_one_scheme_reads_as_that_scheme() {
        assert_eq!(
            Problem::Url { schemes: &["https"] }.to_string(),
            "must be a URL using https"
        );
    }

    #[test]
    fn a_url_s_params_are_the_schemes_accepted() {
        let problem = Problem::Url {
            schemes: &["http", "https"],
        };

        assert_eq!(problem.params(), [("schemes", ParamValue::Text("http, https".into()))]);
    }

    #[test]
    fn a_string_that_is_not_an_ip_address_is_known_as_ip() {
        assert_eq!(Problem::Ip.code(), "ip");
    }

    #[test]
    fn a_string_that_is_not_an_ip_address_reads_as_such() {
        assert_eq!(Problem::Ip.to_string(), "must be an IP address");
    }

    #[test]
    fn a_string_that_is_not_an_ipv4_address_is_known_as_ipv4() {
        assert_eq!(Problem::Ipv4.code(), "ipv4");
    }

    #[test]
    fn a_string_that_is_not_an_ipv4_address_reads_as_such() {
        assert_eq!(Problem::Ipv4.to_string(), "must be an IPv4 address");
    }

    #[test]
    fn a_string_that_is_not_an_ipv6_address_is_known_as_ipv6() {
        assert_eq!(Problem::Ipv6.code(), "ipv6");
    }

    #[test]
    fn a_string_that_is_not_an_ipv6_address_reads_as_such() {
        assert_eq!(Problem::Ipv6.to_string(), "must be an IPv6 address");
    }

    #[test]
    fn an_ip_address_of_one_version_has_no_params() {
        assert_eq!(Problem::Ipv4.params(), []);
    }

    #[test]
    fn a_string_not_matching_a_pattern_is_known_as_pattern() {
        assert_eq!(Problem::Pattern { pattern: r"^\d{5}$" }.code(), "pattern");
    }

    #[test]
    fn a_string_not_matching_a_pattern_reads_as_the_pattern() {
        assert_eq!(
            Problem::Pattern { pattern: r"^\d{5}$" }.to_string(),
            r"must match the pattern ^\d{5}$"
        );
    }

    #[test]
    fn a_pattern_s_param_is_its_regular_expression() {
        assert_eq!(
            Problem::Pattern { pattern: r"^\d{5}$" }.params(),
            [("pattern", ParamValue::Text(r"^\d{5}$".into()))]
        );
    }

    #[test]
    fn a_string_without_a_text_is_known_as_contains() {
        assert_eq!(Problem::Contains { text: "@" }.code(), "contains");
    }

    #[test]
    fn a_string_without_a_text_reads_as_the_text_quoted() {
        assert_eq!(Problem::Contains { text: "@" }.to_string(), r#"must contain "@""#);
    }

    #[test]
    fn a_contains_param_is_its_text() {
        assert_eq!(
            Problem::Contains { text: "@" }.params(),
            [("text", ParamValue::Text("@".into()))]
        );
    }

    #[test]
    fn a_string_without_a_prefix_is_known_as_prefix() {
        assert_eq!(Problem::Prefix { prefix: "SKU-" }.code(), "prefix");
    }

    #[test]
    fn a_string_without_a_prefix_reads_as_the_prefix_quoted() {
        assert_eq!(
            Problem::Prefix { prefix: "SKU-" }.to_string(),
            r#"must start with "SKU-""#
        );
    }

    #[test]
    fn a_prefix_param_is_its_prefix() {
        assert_eq!(
            Problem::Prefix { prefix: "SKU-" }.params(),
            [("prefix", ParamValue::Text("SKU-".into()))]
        );
    }

    #[test]
    fn a_string_without_a_suffix_is_known_as_suffix() {
        assert_eq!(Problem::Suffix { suffix: ".pdf" }.code(), "suffix");
    }

    #[test]
    fn a_string_without_a_suffix_reads_as_the_suffix_quoted() {
        assert_eq!(
            Problem::Suffix { suffix: ".pdf" }.to_string(),
            r#"must end with ".pdf""#
        );
    }

    #[test]
    fn a_suffix_param_is_its_suffix() {
        assert_eq!(
            Problem::Suffix { suffix: ".pdf" }.params(),
            [("suffix", ParamValue::Text(".pdf".into()))]
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
                ("min", ParamValue::Integer(1)),
                ("max", ParamValue::Integer(50)),
                ("unit", ParamValue::Text("bytes".into())),
            ]
        );
    }

    #[test]
    fn a_length_with_only_a_max_has_no_min_param() {
        let problem = length_problem(Bounds::AtMost(50), LengthUnit::Chars);

        assert_eq!(
            problem.params(),
            [
                ("max", ParamValue::Integer(50)),
                ("unit", ParamValue::Text("chars".into()))
            ]
        );
    }

    #[test]
    fn a_length_with_equal_bounds_has_an_equal_param_too() {
        let problem = length_problem(Bounds::Between { min: 5, max: 5 }, LengthUnit::Elements);

        assert_eq!(
            problem.params(),
            [
                ("min", ParamValue::Integer(5)),
                ("max", ParamValue::Integer(5)),
                ("equal", ParamValue::Integer(5)),
                ("unit", ParamValue::Text("elements".into())),
            ]
        );
    }
}
