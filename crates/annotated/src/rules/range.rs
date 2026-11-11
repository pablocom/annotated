use std::fmt::Display;

use crate::{Bounds, Problem, Rule};

/// The rule that a value is within bounds, inclusive: `range().min(18).max(120)`.
///
/// It checks anything `PartialOrd` and `Display`, against bounds of the same type; `#[validate(range(min =
/// 18))]` takes the bounds' type from the field. A value that can't be compared with a bound, like a float's
/// NaN, is out of range. A range with no bounds accepts any value.
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
impl<T: PartialOrd + Display> Rule<T> for Range<T> {
    fn check(&self, value: &T) -> Result<(), Problem> {
        let is_high_enough = self.min.as_ref().is_none_or(|min| value >= min);
        let is_low_enough = self.max.as_ref().is_none_or(|max| value <= max);
        if is_high_enough && is_low_enough {
            return Ok(());
        }
        let render = |bound: &T| bound.to_string();
        let bounds = Bounds::of(self.min.as_ref().map(render), self.max.as_ref().map(render))
            .expect("a value out of range has bounds");
        Err(Problem::Range { bounds })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn out_of_range(bounds: Bounds<String>) -> Result<(), Problem> {
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
                min: "18".to_owned(),
                max: "120".to_owned()
            })
        );
    }

    #[test]
    fn a_value_above_the_max_is_out_of_range() {
        assert_eq!(
            range().max(120).check(&121),
            out_of_range(Bounds::AtMost("120".to_owned()))
        );
    }

    #[test]
    fn a_range_with_only_a_min_is_bounded_below() {
        assert_eq!(
            range().min(0.5).check(&0.25),
            out_of_range(Bounds::AtLeast("0.5".to_owned()))
        );
    }

    #[test]
    fn a_value_not_comparable_with_the_bounds_is_out_of_range() {
        assert_eq!(
            range().min(0.0).max(1.0).check(&f64::NAN),
            out_of_range(Bounds::Between {
                min: "0".to_owned(),
                max: "1".to_owned()
            })
        );
    }

    #[test]
    fn a_range_without_bounds_accepts_any_value() {
        assert_eq!(range().check(&f64::NAN), Ok(()));
    }
}
