//! `nested` validates a field that implements `Validate`, under the field's name.

mod support;

use annotated::{Problem, Validate};
use support::issues_of;

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
struct Address {
    #[validate(required)]
    street: String,
    #[validate(length(min = 4))]
    postal_code: String,
}

fn address() -> Address {
    Address {
        street: "1 Main Street".to_owned(),
        postal_code: "10001".to_owned(),
    }
}

fn not_a_known_address(_: &Address) -> Result<(), Problem> {
    Err(Problem::custom("unknown_address", "is not a known address"))
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
struct Order {
    #[validate(nested)]
    shipping_address: Address,
    #[validate(nested)]
    billing_address: Option<Address>,
    #[validate(nested, custom(not_a_known_address))]
    pickup_point: Address,
}

fn order() -> Order {
    Order {
        shipping_address: address(),
        billing_address: None,
        pickup_point: Address {
            street: String::new(),
            ..address()
        },
    }
}

#[test]
fn a_nested_struct_s_issues_are_reported_under_its_field() {
    let order = Order {
        shipping_address: Address {
            street: String::new(),
            ..address()
        },
        ..order()
    };

    assert_eq!(
        issues_of(&order),
        ["shippingAddress.street: required", "pickupPoint.street: required"]
    );
}

#[test]
fn every_issue_of_a_nested_struct_is_reported() {
    let order = Order {
        billing_address: Some(Address {
            street: String::new(),
            postal_code: "1".to_owned(),
        }),
        ..order()
    };

    assert_eq!(
        issues_of(&order),
        [
            "billingAddress.street: required",
            "billingAddress.postalCode: length",
            "pickupPoint.street: required"
        ]
    );
}

#[test]
fn a_nested_struct_in_an_option_is_skipped_when_there_is_none() {
    let order = Order {
        billing_address: None,
        ..order()
    };

    assert_eq!(issues_of(&order), ["pickupPoint.street: required"]);
}

#[test]
fn rules_after_a_nested_struct_with_issues_are_skipped() {
    assert_eq!(issues_of(&order()), ["pickupPoint.street: required"]);
}

#[test]
fn rules_after_a_valid_nested_struct_are_checked() {
    let order = Order {
        pickup_point: address(),
        ..order()
    };

    assert_eq!(issues_of(&order), ["pickupPoint: unknown_address"]);
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
#[validate(report_all)]
struct Delivery {
    #[validate(nested, custom(not_a_known_address))]
    drop_off: Address,
}

#[test]
fn report_all_checks_the_rules_after_a_nested_struct_with_issues() {
    let delivery = Delivery {
        drop_off: Address {
            street: String::new(),
            ..address()
        },
    };

    assert_eq!(
        issues_of(&delivery),
        ["dropOff.street: required", "dropOff: unknown_address"]
    );
}
