use std::borrow::Cow;

use crate::Problem;

/// A type's rules, as data: what `#[validate(...)]` requires of each of its fields.
///
/// [`Validate::rules`](crate::Validate::rules) and [`Deserialize::rules`](crate::Deserialize::rules) return it. Both
/// derives implement `rules()` for a struct:
///
/// ```
/// use annotated::{FieldRules, Problem, Rules, Validate, ValueRules};
///
/// #[derive(Validate)]
/// struct Subscriber {
///     #[validate(required, email)]
///     email: String,
/// }
///
/// let email_rules = ValueRules::new().with_problem(Problem::Required).with_problem(Problem::Email);
///
/// assert_eq!(Subscriber::rules(), Rules::of_fields([FieldRules::new("email", email_rules)]));
/// ```
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rules {
    fields: Vec<FieldRules>,
}

impl Rules {
    /// The rules of a type whose fields have these rules.
    pub fn of_fields(fields: impl IntoIterator<Item = FieldRules>) -> Self {
        Self {
            fields: fields.into_iter().collect(),
        }
    }

    /// The rules of each field that has some, in the order the fields are declared.
    #[must_use]
    pub fn fields(&self) -> &[FieldRules] {
        &self.fields
    }
}

/// The rules of one field: its name, and what they require of its value.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldRules {
    name: Cow<'static, str>,
    has_default: bool,
    rules: ValueRules,
}

impl FieldRules {
    /// The rules of the field that serde reads by `name`.
    pub fn new(name: impl Into<Cow<'static, str>>, rules: ValueRules) -> Self {
        Self {
            name: name.into(),
            has_default: false,
            rules,
        }
    }

    /// The same rules, on a field that takes a default when the input lacks it.
    #[must_use]
    pub fn with_default(self) -> Self {
        Self {
            has_default: true,
            ..self
        }
    }

    /// The name serde reads the field by.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Whether the field takes a default when the input lacks it: `#[serde(default)]` on the field or on its struct.
    /// Its rules then check that default, so the input may leave out even a `required` field.
    #[must_use]
    pub fn has_default(&self) -> bool {
        self.has_default
    }

    /// What the rules require of the field's value.
    #[must_use]
    pub fn rules(&self) -> &ValueRules {
        &self.rules
    }
}

/// What rules require of one value: a field's, or an element's.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ValueRules {
    is_optional: bool,
    problems: Vec<Problem>,
    elements: Option<Box<ValueRules>>,
}

impl ValueRules {
    /// No rules yet, on a value that is not an `Option`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// No rules yet, on a value that is an `Option`.
    #[must_use]
    pub fn optional() -> Self {
        Self {
            is_optional: true,
            ..Self::default()
        }
    }

    /// The same rules, and one more that reports `problem`.
    #[must_use]
    pub fn with_problem(mut self, problem: Problem) -> Self {
        self.problems.push(problem);
        self
    }

    /// The same rules, and `elements` for each of the value's elements.
    #[must_use]
    pub fn with_elements(self, elements: ValueRules) -> Self {
        Self {
            elements: Some(Box::new(elements)),
            ..self
        }
    }

    /// Whether the value is an `Option`. `required` then asks for a `Some`, and every other rule checks the value
    /// inside and accepts a `None`.
    #[must_use]
    pub fn is_optional(&self) -> bool {
        self.is_optional
    }

    /// The problem of each rule that [says one](crate::Rule::problem), in the order the rules are written. A rule
    /// that doesn't, like `custom(...)`, isn't listed.
    #[must_use]
    pub fn problems(&self) -> &[Problem] {
        &self.problems
    }

    /// What `each(...)` requires of every element of a collection, or of every value of a map.
    #[must_use]
    pub fn elements(&self) -> Option<&ValueRules> {
        self.elements.as_deref()
    }
}
