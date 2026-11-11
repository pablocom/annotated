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

/// The value of one of a [`Problem`]'s named parameters.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum ParamValue {
    /// A whole number, like a length's maximum.
    Integer(i128),
    /// Text, like a value rendered with [`Display`].
    Text(Cow<'static, str>),
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

    /// This problem with the named parameter `name` added, for messages to quote and code to read.
    ///
    /// Only a [custom](Problem::custom) problem takes parameters; a built-in problem's parameters are fixed,
    /// and it is returned unchanged.
    #[must_use]
    pub fn with_param(mut self, name: &'static str, value: impl Into<ParamValue>) -> Self {
        let Self::Custom { params, .. } = &mut self;
        params.push((name, value.into()));
        self
    }

    /// The stable code this kind of problem is known by, in `snake_case`.
    #[must_use]
    pub fn code(&self) -> &str {
        match self {
            Self::Custom { code, .. } => code,
        }
    }

    /// The named parameters of this problem, such as a length's `max`.
    #[must_use]
    pub fn params(&self) -> Vec<(&'static str, ParamValue)> {
        match self {
            Self::Custom { params, .. } => params.clone(),
        }
    }
}

impl Display for Problem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Custom { message, .. } => formatter.write_str(message),
        }
    }
}

impl Display for ParamValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(integer) => write!(formatter, "{integer}"),
            Self::Text(text) => formatter.write_str(text),
        }
    }
}

macro_rules! integer_param_values {
    ($($integer:ty),*) => {
        $(
            impl From<$integer> for ParamValue {
                fn from(value: $integer) -> Self {
                    Self::Integer(i128::try_from(value).expect("every integer of 64 bits or fewer fits in an i128"))
                }
            }
        )*
    };
}

integer_param_values!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

impl From<&'static str> for ParamValue {
    fn from(text: &'static str) -> Self {
        Self::Text(Cow::Borrowed(text))
    }
}

impl From<String> for ParamValue {
    fn from(text: String) -> Self {
        Self::Text(Cow::Owned(text))
    }
}

impl From<Cow<'static, str>> for ParamValue {
    fn from(text: Cow<'static, str>) -> Self {
        Self::Text(text)
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
                ("name", ParamValue::Text(Cow::Borrowed("admin"))),
                ("reserved_names", ParamValue::Integer(3)),
            ]
        );
    }

    #[test]
    fn a_whole_number_is_an_integer_param() {
        assert_eq!(ParamValue::from(-7_i64), ParamValue::Integer(-7));
    }

    #[test]
    fn an_owned_string_is_a_text_param() {
        assert_eq!(
            ParamValue::from("admin".to_owned()),
            ParamValue::Text(Cow::Borrowed("admin"))
        );
    }

    #[test]
    fn an_integer_param_reads_as_its_number() {
        assert_eq!(ParamValue::Integer(50).to_string(), "50");
    }

    #[test]
    fn a_text_param_reads_as_its_text() {
        assert_eq!(ParamValue::Text(Cow::Borrowed("admin")).to_string(), "admin");
    }
}
