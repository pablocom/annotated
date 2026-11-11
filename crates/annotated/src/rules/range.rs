use std::fmt::Display;

use crate::{Bounds, ParamValue, Problem, Rule};

/// The rule that a value is within bounds, inclusive: `range().min(18).max(120)`.
///
/// It checks anything `PartialOrd` and `Display`, against bounds of the same type; `#[validate(range(min =
/// 18))]` takes the bounds' type from the field. A value that can't be compared with a bound, like a float's
/// NaN, is out of range. A range with no bounds accepts any value.
///
/// Its problem's bounds are [`ParamValue`] numbers when the type is a primitive number type, like `u8` or
/// `f64`, and text rendered with `Display` for any other type.
///
/// ```
/// use annotated::{Rule, rules};
///
/// assert_eq!(rules::range().min(18).max(120).check(&30), Ok(()));
///
/// let too_young = rules::range().min(18).max(120).check(&12).unwrap_err();
/// assert_eq!(too_young.to_string(), "must be between 18 and 120");
/// ```
#[must_use]
pub const fn range<T>() -> Range<T> {
    Range { min: None, max: None }
}

/// Checks that a value is within bounds; made by [`range`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range<T> {
    min: Option<T>,
    max: Option<T>,
}

impl<T> Range<T> {
    /// The rule with `min` as the least value it accepts.
    #[must_use]
    pub fn min(self, min: T) -> Self {
        Self { min: Some(min), ..self }
    }

    /// The rule with `max` as the greatest value it accepts.
    #[must_use]
    pub fn max(self, max: T) -> Self {
        Self { max: Some(max), ..self }
    }
}

#[diagnostic::do_not_recommend]
impl<T: PartialOrd + Display + 'static> Rule<T> for Range<T> {
    fn check(&self, value: &T) -> Result<(), Problem> {
        let is_high_enough = self.min.as_ref().is_none_or(|min| value >= min);
        let is_low_enough = self.max.as_ref().is_none_or(|max| value <= max);
        if is_high_enough && is_low_enough {
            return Ok(());
        }
        let bounds = Bounds::of(
            self.min.as_ref().map(ParamValue::number_or_text),
            self.max.as_ref().map(ParamValue::number_or_text),
        )
        .expect("a value out of range has bounds");
        Err(Problem::Range { bounds })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out_of_range(bounds: Bounds<ParamValue>) -> Result<(), Problem> {
        Err(Problem::Range { bounds })
    }

    #[test]
    fn a_value_within_the_bounds_follows_the_rule() {
        assert_eq!(range().min(18).max(120).check(&30), Ok(()));
    }

    #[test]
    fn the_bounds_are_inclusive() {
        assert_eq!(
            (
                range().min(18).max(120).check(&18),
                range().min(18).max(120).check(&120)
            ),
            (Ok(()), Ok(()))
        );
    }

    #[test]
    fn a_value_below_the_min_is_out_of_range() {
        assert_eq!(
            range().min(18).max(120).check(&12),
            out_of_range(Bounds::Between {
                min: ParamValue::Integer(18),
                max: ParamValue::Integer(120)
            })
        );
    }

    #[test]
    fn a_value_above_the_max_is_out_of_range() {
        assert_eq!(
            range().max(120).check(&121),
            out_of_range(Bounds::AtMost(ParamValue::Integer(120)))
        );
    }

    #[test]
    fn a_range_with_only_a_min_is_bounded_below() {
        assert_eq!(
            range().min(0.5).check(&0.25),
            out_of_range(Bounds::AtLeast(ParamValue::Float(0.5)))
        );
    }

    #[test]
    fn a_value_not_comparable_with_the_bounds_is_out_of_range() {
        assert_eq!(
            range().min(0.0).max(1.0).check(&f64::NAN),
            out_of_range(Bounds::Between {
                min: ParamValue::Float(0.0),
                max: ParamValue::Float(1.0)
            })
        );
    }

    #[test]
    fn a_range_without_bounds_accepts_any_value() {
        assert_eq!(range().check(&f64::NAN), Ok(()));
    }

    #[test]
    fn the_bounds_of_every_primitive_integer_type_are_integers() {
        assert_eq!(
            [
                range().min(1_u8).check(&0),
                range().min(1_u16).check(&0),
                range().min(1_u32).check(&0),
                range().min(1_u64).check(&0),
                range().min(1_u128).check(&0),
                range().min(1_usize).check(&0),
                range().min(1_i8).check(&0),
                range().min(1_i16).check(&0),
                range().min(1_i32).check(&0),
                range().min(1_i64).check(&0),
                range().min(1_i128).check(&0),
                range().min(1_isize).check(&0),
            ],
            [
                out_of_range(Bounds::AtLeast(ParamValue::Integer(1))),
                out_of_range(Bounds::AtLeast(ParamValue::Integer(1))),
                out_of_range(Bounds::AtLeast(ParamValue::Integer(1))),
                out_of_range(Bounds::AtLeast(ParamValue::Integer(1))),
                out_of_range(Bounds::AtLeast(ParamValue::Integer(1))),
                out_of_range(Bounds::AtLeast(ParamValue::Integer(1))),
                out_of_range(Bounds::AtLeast(ParamValue::Integer(1))),
                out_of_range(Bounds::AtLeast(ParamValue::Integer(1))),
                out_of_range(Bounds::AtLeast(ParamValue::Integer(1))),
                out_of_range(Bounds::AtLeast(ParamValue::Integer(1))),
                out_of_range(Bounds::AtLeast(ParamValue::Integer(1))),
                out_of_range(Bounds::AtLeast(ParamValue::Integer(1))),
            ]
        );
    }

    #[test]
    fn the_bounds_of_both_float_types_are_the_floats_they_read_as() {
        assert_eq!(
            [range().max(0.1_f32).check(&0.5), range().max(0.1_f64).check(&0.5)],
            [
                out_of_range(Bounds::AtMost(ParamValue::Float(0.1))),
                out_of_range(Bounds::AtMost(ParamValue::Float(0.1))),
            ]
        );
    }

    #[test]
    fn the_bounds_of_a_type_other_than_a_number_are_text() {
        assert_eq!(
            range().min('A').max('F').check(&'G'),
            out_of_range(Bounds::Between {
                min: ParamValue::Text("A".into()),
                max: ParamValue::Text("F".into())
            })
        );
    }

    #[test]
    fn a_bound_reads_as_its_type_renders_it() {
        assert_eq!(
            range().max(0.1_f32).check(&0.5).unwrap_err().to_string(),
            "must be at most 0.1"
        );
    }
}
