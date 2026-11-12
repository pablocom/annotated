//! Rules of your own, named by a path, written with the same syntax as the built-in rules.

mod support;

use annotated::Validate;
use support::issues_of;

mod house_rules {
    use annotated::{Problem, Rule};

    pub fn lowercase() -> Lowercase {
        Lowercase
    }

    pub struct Lowercase;

    impl<T: AsRef<str> + ?Sized> Rule<T> for Lowercase {
        fn check(&self, value: &T) -> Result<(), Problem> {
            if value.as_ref().chars().any(char::is_uppercase) {
                return Err(Problem::custom("lowercase", "must be lowercase"));
            }
            Ok(())
        }
    }

    pub fn word_count() -> WordCount {
        WordCount { max: usize::MAX }
    }

    pub struct WordCount {
        max: usize,
    }

    impl WordCount {
        pub fn max(mut self, max: usize) -> Self {
            self.max = max;
            self
        }
    }

    impl<T: AsRef<str> + ?Sized> Rule<T> for WordCount {
        fn check(&self, value: &T) -> Result<(), Problem> {
            if value.as_ref().split_whitespace().count() > self.max {
                return Err(Problem::custom("word_count", "has too many words").with_param("max", self.max));
            }
            Ok(())
        }
    }

    pub fn divisible_by(divisor: u32) -> DivisibleBy {
        DivisibleBy { divisor }
    }

    pub struct DivisibleBy {
        divisor: u32,
    }

    impl Rule<u32> for DivisibleBy {
        fn check(&self, value: &u32) -> Result<(), Problem> {
            if !value.is_multiple_of(self.divisor) {
                return Err(Problem::custom("divisible_by", "is not divisible").with_param("divisor", self.divisor));
            }
            Ok(())
        }
    }
}

#[derive(Validate)]
struct NewProduct {
    #[validate(house_rules::lowercase)]
    slug: String,
    #[validate(house_rules::word_count(max = 3, message = "use {max} words at most"))]
    name: String,
    #[validate(house_rules::divisible_by = 6)]
    pack_size: u32,
    #[validate(house_rules::divisible_by(5, message = "must be a multiple of {divisor}"))]
    price_in_cents: u32,
}

fn new_product() -> NewProduct {
    NewProduct {
        slug: "garden-chair".to_owned(),
        name: "Garden chair".to_owned(),
        pack_size: 12,
        price_in_cents: 4995,
    }
}

#[test]
fn a_value_following_rules_of_your_own_has_no_issues() {
    assert_eq!(new_product().validate(), Ok(()));
}

#[test]
fn a_rule_named_by_a_path_is_the_one_the_path_names() {
    let product = NewProduct {
        slug: "Garden-Chair".to_owned(),
        ..new_product()
    };

    assert_eq!(issues_of(&product), ["slug: lowercase"]);
}

#[test]
fn named_arguments_of_a_rule_of_your_own_call_its_builder_methods() {
    let product = NewProduct {
        name: "Folding garden chair with cushion".to_owned(),
        ..new_product()
    };

    assert_eq!(product.validate().unwrap_err().to_string(), "name: use 3 words at most");
}

#[test]
fn a_value_after_an_equals_sign_is_passed_to_the_rule_s_constructor() {
    let product = NewProduct {
        pack_size: 10,
        ..new_product()
    };

    assert_eq!(issues_of(&product), ["pack_size: divisible_by"]);
}

#[test]
fn a_value_in_parentheses_is_passed_to_the_rule_s_constructor() {
    let product = NewProduct {
        price_in_cents: 4999,
        ..new_product()
    };

    assert_eq!(
        product.validate().unwrap_err().to_string(),
        "price_in_cents: must be a multiple of 5"
    );
}
