//! The built-in rules, written in `#[validate(...)]`.

mod support;

use annotated::Validate;
use support::violations_of;

#[derive(Validate)]
struct NewMember {
    #[validate(range(min = 18, max = 120))]
    age: u8,
    #[validate(range(min = 0.0))]
    balance: Option<f64>,
}

fn new_member() -> NewMember {
    NewMember {
        age: 34,
        balance: Some(10.5),
    }
}

#[test]
fn a_range_takes_its_bounds_type_from_the_field() {
    let member = NewMember {
        age: 12,
        ..new_member()
    };

    assert_eq!(
        member.validate().unwrap_err().to_string(),
        "age: must be between 18 and 120"
    );
}

#[test]
fn a_range_on_an_option_checks_the_number_inside() {
    let member = NewMember {
        balance: Some(-0.5),
        ..new_member()
    };

    assert_eq!(violations_of(&member), ["balance: range"]);
}
