//! `nested` validates a field that implements `Validate`, under the field's name.

mod support;

use std::borrow::Cow;
use std::rc::Rc;
use std::sync::Arc;

use annotated::{Problem, Validate};
use support::issues_of;

#[derive(Validate, Clone)]
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

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
struct Customer {
    #[validate(required)]
    name: String,
    #[serde(flatten)]
    #[validate(nested)]
    address: Address,
}

#[test]
fn a_flattened_nested_struct_is_validated_at_the_struct_s_location() {
    let customer = Customer {
        name: String::new(),
        address: Address {
            street: String::new(),
            ..address()
        },
    };

    assert_eq!(issues_of(&customer), ["name: required", "street: required"]);
}

fn without_a_street() -> Address {
    Address {
        street: String::new(),
        ..address()
    }
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
struct Itinerary<'a> {
    #[validate(nested)]
    boxed_stop: Box<Address>,
    #[validate(nested)]
    shared_stop: Rc<Address>,
    #[validate(nested)]
    thread_shared_stop: Arc<Address>,
    #[validate(nested)]
    borrowed_stop: &'a Address,
    #[validate(nested)]
    copied_on_write_stop: Cow<'a, Address>,
    #[validate(nested)]
    leg: (Address, Address),
}

fn itinerary(valid: &Address) -> Itinerary<'_> {
    Itinerary {
        boxed_stop: Box::new(address()),
        shared_stop: Rc::new(address()),
        thread_shared_stop: Arc::new(address()),
        borrowed_stop: valid,
        copied_on_write_stop: Cow::Owned(address()),
        leg: (address(), address()),
    }
}

#[test]
fn a_nested_struct_behind_a_box_is_validated_under_its_field() {
    let valid = address();
    let itinerary = Itinerary {
        boxed_stop: Box::new(without_a_street()),
        ..itinerary(&valid)
    };

    assert_eq!(issues_of(&itinerary), ["boxedStop.street: required"]);
}

#[test]
fn a_nested_struct_behind_an_rc_is_validated_under_its_field() {
    let valid = address();
    let itinerary = Itinerary {
        shared_stop: Rc::new(without_a_street()),
        ..itinerary(&valid)
    };

    assert_eq!(issues_of(&itinerary), ["sharedStop.street: required"]);
}

#[test]
fn a_nested_struct_behind_an_arc_is_validated_under_its_field() {
    let valid = address();
    let itinerary = Itinerary {
        thread_shared_stop: Arc::new(without_a_street()),
        ..itinerary(&valid)
    };

    assert_eq!(issues_of(&itinerary), ["threadSharedStop.street: required"]);
}

#[test]
fn a_borrowed_nested_struct_is_validated_under_its_field() {
    let valid = address();
    let invalid = without_a_street();
    let itinerary = Itinerary {
        borrowed_stop: &invalid,
        ..itinerary(&valid)
    };

    assert_eq!(issues_of(&itinerary), ["borrowedStop.street: required"]);
}

#[test]
fn a_nested_struct_in_a_cow_is_validated_under_its_field() {
    let valid = address();
    let invalid = without_a_street();
    let itinerary = Itinerary {
        copied_on_write_stop: Cow::Borrowed(&invalid),
        ..itinerary(&valid)
    };

    assert_eq!(issues_of(&itinerary), ["copiedOnWriteStop.street: required"]);
}

#[test]
fn the_elements_of_a_nested_tuple_are_validated_at_their_index() {
    let valid = address();
    let itinerary = Itinerary {
        leg: (without_a_street(), without_a_street()),
        ..itinerary(&valid)
    };

    assert_eq!(
        issues_of(&itinerary),
        ["leg[0].street: required", "leg[1].street: required"]
    );
}

#[test]
fn failing_fast_stops_a_tuple_at_its_first_issue() {
    let leg = (without_a_street(), without_a_street());

    let issues = leg.validate_fail_fast().unwrap_err();

    assert_eq!(issues.to_string(), "[0].street: is required");
}

#[test]
fn a_struct_behind_a_mutable_reference_is_validated() {
    let mut invalid = without_a_street();
    let borrowed = &mut invalid;

    assert_eq!(issues_of(&borrowed), ["street: required"]);
}
