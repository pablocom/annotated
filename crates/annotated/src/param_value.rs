use std::any::Any;
use std::borrow::Cow;
use std::fmt::{self, Display};

/// The value of one of a [`Problem`](crate::Problem)'s named parameters: a number or text.
///
/// Because it is typed, code that translates a message can pick a plural form or compare a bound without parsing text.
/// In a message, a placeholder is replaced by the value as [`Display`] prints it: `{max}` becomes `50`.
///
/// ```
/// use annotated::{ParamValue, Problem};
///
/// let problem = Problem::custom("too_many_guests", "has too many guests").with_param("max", 8);
///
/// assert_eq!(problem.params(), [("max", ParamValue::Integer(8))]);
/// ```
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ParamValue {
    /// A whole number, like a length's `max`. Every primitive integer is kept exactly, except a `u128` too large for an
    /// `i128`, which is kept as its digits in [`ParamValue::Text`].
    Integer(i128),
    /// A floating-point number, like a `range` bound on an `f64`.
    Float(f64),
    /// Text, like a `pattern`'s regular expression.
    Text(Cow<'static, str>),
}

impl ParamValue {
    pub(crate) const fn text(text: &'static str) -> Self {
        Self::Text(Cow::Borrowed(text))
    }

    /// `value` as a number when its type is a primitive number type, or else as text rendered with
    /// [`Display`].
    pub(crate) fn number_or_text<T: Display + Any>(value: &T) -> Self {
        // Without specialization, a primitive number type can only be told apart from any other `Display`
        // type at run time; this runs only once a value has broken a rule.
        macro_rules! first_number_type_of {
            ($any:expr, $($number:ty),*) => {
                $(
                    if let Some(number) = $any.downcast_ref::<$number>() {
                        return Self::from(*number);
                    }
                )*
            };
        }

        let any: &dyn Any = value;
        first_number_type_of!(
            any, u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize, f32, f64
        );
        Self::Text(value.to_string().into())
    }
}

impl Display for ParamValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Integer(integer) => Display::fmt(integer, formatter),
            Self::Float(float) => Display::fmt(float, formatter),
            Self::Text(text) => Display::fmt(text, formatter),
        }
    }
}

macro_rules! integer_param_values {
    ($($integer:ty),*) => {
        $(
            impl From<$integer> for ParamValue {
                fn from(integer: $integer) -> Self {
                    Self::Integer(i128::try_from(integer).expect("every integer of 64 bits or fewer fits in an i128"))
                }
            }
        )*
    };
}

integer_param_values!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

impl From<i128> for ParamValue {
    fn from(integer: i128) -> Self {
        Self::Integer(integer)
    }
}

impl From<u128> for ParamValue {
    fn from(integer: u128) -> Self {
        i128::try_from(integer).map_or_else(|_| Self::Text(integer.to_string().into()), Self::Integer)
    }
}

impl From<f64> for ParamValue {
    fn from(float: f64) -> Self {
        Self::Float(float)
    }
}

impl From<f32> for ParamValue {
    fn from(float: f32) -> Self {
        // Widening would turn 0.1 into 0.10000000149011612; the f64 of its rendering keeps the decimal it reads as.
        let decimal = float.to_string();
        Self::Float(decimal.parse().expect("an f32 renders as a decimal an f64 parses"))
    }
}

impl From<&str> for ParamValue {
    fn from(text: &str) -> Self {
        Self::Text(Cow::Owned(text.to_owned()))
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
    fn a_whole_number_is_an_integer_param() {
        assert_eq!(ParamValue::from(-7_i64), ParamValue::Integer(-7));
    }

    #[test]
    fn the_greatest_u64_is_an_exact_integer_param() {
        assert_eq!(
            ParamValue::from(u64::MAX),
            ParamValue::Integer(18_446_744_073_709_551_615)
        );
    }

    #[test]
    fn the_least_i64_is_an_exact_integer_param() {
        assert_eq!(
            ParamValue::from(i64::MIN),
            ParamValue::Integer(-9_223_372_036_854_775_808)
        );
    }

    #[test]
    fn a_u128_an_i128_can_hold_is_an_integer_param() {
        assert_eq!(ParamValue::from(50_u128), ParamValue::Integer(50));
    }

    #[test]
    fn a_u128_beyond_what_an_i128_holds_keeps_its_digits_as_text() {
        assert_eq!(
            ParamValue::from(u128::MAX),
            ParamValue::Text("340282366920938463463374607431768211455".into())
        );
    }

    #[test]
    fn an_f64_is_a_float_param() {
        assert_eq!(ParamValue::from(0.5_f64), ParamValue::Float(0.5));
    }

    #[test]
    fn an_f32_is_the_float_param_it_reads_as() {
        assert_eq!(ParamValue::from(0.1_f32), ParamValue::Float(0.1));
    }

    #[test]
    fn a_borrowed_string_is_a_text_param() {
        let name = "admin".to_owned();

        assert_eq!(ParamValue::from(name.as_str()), ParamValue::Text("admin".into()));
    }

    #[test]
    fn an_owned_string_is_a_text_param() {
        assert_eq!(ParamValue::from("admin".to_owned()), ParamValue::Text("admin".into()));
    }

    #[test]
    fn an_integer_param_reads_as_its_number() {
        assert_eq!(ParamValue::Integer(-50).to_string(), "-50");
    }

    #[test]
    fn a_float_param_reads_as_its_number() {
        assert_eq!(ParamValue::Float(0.5).to_string(), "0.5");
    }

    #[test]
    fn a_text_param_reads_as_its_text() {
        assert_eq!(ParamValue::Text("admin".into()).to_string(), "admin");
    }
}
