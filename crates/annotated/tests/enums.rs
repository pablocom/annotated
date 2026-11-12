//! An enum is validated through the variant it holds. Each issue is at the path where serde reads its value, which
//! depends on how the enum is tagged.

mod support;

use annotated::{Problem, Validate};
use support::{issues_of, paths_and_codes};

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
enum Payment {
    Card {
        #[validate(length(min = 16))]
        number: String,
        #[validate(required)]
        holder: String,
    },
    BankTransfer {
        #[validate(required)]
        iban: String,
    },
}

#[test]
fn a_variant_whose_fields_follow_their_rules_has_no_issues() {
    let payment = Payment::Card {
        number: "4111111111111111".to_owned(),
        holder: "Ada Lovelace".to_owned(),
    };

    assert_eq!(payment.validate(), Ok(()));
}

#[test]
fn an_issue_in_a_variant_is_under_the_variant_s_name() {
    let payment = Payment::Card {
        number: "4111".to_owned(),
        holder: "Ada Lovelace".to_owned(),
    };

    assert_eq!(issues_of(&payment), ["card.number: length"]);
}

#[test]
fn every_field_of_the_variant_is_checked() {
    let payment = Payment::Card {
        number: "4111".to_owned(),
        holder: String::new(),
    };

    assert_eq!(issues_of(&payment), ["card.number: length", "card.holder: required"]);
}

#[test]
fn a_variant_is_reported_by_the_name_the_enum_s_rename_all_gives_it() {
    let payment = Payment::BankTransfer { iban: String::new() };

    assert_eq!(issues_of(&payment), ["bankTransfer.iban: required"]);
}

#[test]
fn failing_fast_an_enum_answers_its_first_issue() {
    let payment = Payment::Card {
        number: "4111".to_owned(),
        holder: String::new(),
    };

    assert_eq!(paths_and_codes(payment.validate_fail_fast()), ["card.number: length"]);
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
enum Shape {
    Circle(#[validate(range(min = 1))] u32),
    Rectangle(#[validate(range(min = 1))] u32, #[validate(range(min = 1))] u32),
}

#[test]
fn a_newtype_variant_s_value_is_at_the_variant_s_name() {
    assert_eq!(issues_of(&Shape::Circle(0)), ["circle: range"]);
}

#[test]
fn a_tuple_variant_s_elements_are_at_their_index() {
    assert_eq!(issues_of(&Shape::Rectangle(3, 0)), ["rectangle[1]: range"]);
}

#[derive(Validate)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Notification {
    Email {
        #[validate(email)]
        address: String,
    },
    Push(#[validate(nested)] Device),
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
struct Device {
    #[validate(required)]
    token: String,
}

#[test]
fn an_issue_in_an_internally_tagged_variant_is_at_the_enum_s_location() {
    let notification = Notification::Email {
        address: "ada".to_owned(),
    };

    assert_eq!(issues_of(&notification), ["address: email"]);
}

#[test]
fn an_internally_tagged_newtype_variant_s_value_is_at_the_enum_s_location() {
    let notification = Notification::Push(Device { token: String::new() });

    assert_eq!(issues_of(&notification), ["token: required"]);
}

#[derive(Validate)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
enum Event {
    SignedUp {
        #[validate(email)]
        email: String,
    },
    Renamed(#[validate(required)] String),
}

#[test]
fn an_issue_in_an_adjacently_tagged_variant_is_under_the_content_s_name() {
    let event = Event::SignedUp {
        email: "ada".to_owned(),
    };

    assert_eq!(issues_of(&event), ["data.email: email"]);
}

#[test]
fn an_adjacently_tagged_newtype_variant_s_value_is_at_the_content_s_name() {
    assert_eq!(issues_of(&Event::Renamed(String::new())), ["data: required"]);
}

#[derive(Validate)]
#[serde(untagged)]
enum Recipient {
    ByEmail {
        #[validate(email)]
        email: String,
    },
    ById(#[validate(range(min = 1))] u64),
}

#[test]
fn an_issue_in_an_untagged_variant_is_at_the_enum_s_location() {
    let recipient = Recipient::ByEmail {
        email: "ada".to_owned(),
    };

    assert_eq!(issues_of(&recipient), ["email: email"]);
}

#[test]
fn an_untagged_newtype_variant_s_value_is_at_the_enum_s_location() {
    assert_eq!(issues_of(&Recipient::ById(0)), ["range"]);
}

#[derive(Validate)]
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
enum Quantity {
    Exact(#[validate(range(min = 1))] u32),
    #[serde(untagged)]
    Described(#[validate(required)] String),
}

#[test]
fn an_untagged_variant_of_a_tagged_enum_is_at_the_enum_s_location() {
    assert_eq!(issues_of(&Quantity::Described(String::new())), ["required"]);
}

#[derive(Validate)]
#[serde(rename_all_fields = "camelCase")]
enum Delivery {
    PickUp {
        #[validate(required)]
        store_id: String,
    },
    #[serde(rename = "home")]
    Home {
        #[validate(required)]
        street: String,
    },
    #[serde(rename_all = "kebab-case")]
    Locker {
        #[validate(required)]
        locker_code: String,
    },
}

#[test]
fn a_variant_s_fields_are_reported_by_the_names_the_enum_s_rename_all_fields_gives_them() {
    let delivery = Delivery::PickUp {
        store_id: String::new(),
    };

    assert_eq!(issues_of(&delivery), ["PickUp.storeId: required"]);
}

#[test]
fn a_renamed_variant_is_reported_by_its_new_name() {
    let delivery = Delivery::Home { street: String::new() };

    assert_eq!(issues_of(&delivery), ["home.street: required"]);
}

#[test]
fn a_variant_s_own_rename_all_names_its_fields_over_the_enum_s_rename_all_fields() {
    let delivery = Delivery::Locker {
        locker_code: String::new(),
    };

    assert_eq!(issues_of(&delivery), ["Locker.locker-code: required"]);
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
struct Address {
    #[validate(required)]
    street: String,
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
enum Shipping {
    Courier {
        #[validate(nested)]
        address: Address,
    },
    Locker(#[validate(nested)] Address),
    Pallet {
        #[serde(flatten)]
        #[validate(nested)]
        address: Address,
    },
}

#[test]
fn a_nested_value_in_a_variant_is_validated_at_its_full_path() {
    let shipping = Shipping::Courier {
        address: Address { street: String::new() },
    };

    assert_eq!(issues_of(&shipping), ["courier.address.street: required"]);
}

#[test]
fn a_nested_value_in_a_newtype_variant_is_validated_at_the_variant_s_name() {
    let shipping = Shipping::Locker(Address { street: String::new() });

    assert_eq!(issues_of(&shipping), ["locker.street: required"]);
}

#[test]
fn a_flattened_field_in_a_variant_is_validated_at_the_variant_s_location() {
    let shipping = Shipping::Pallet {
        address: Address { street: String::new() },
    };

    assert_eq!(issues_of(&shipping), ["pallet.street: required"]);
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
struct Order {
    #[validate(nested)]
    shipping: Shipping,
}

#[test]
fn an_enum_nested_in_a_struct_is_validated_under_the_field_s_name() {
    let order = Order {
        shipping: Shipping::Courier {
            address: Address { street: String::new() },
        },
    };

    assert_eq!(issues_of(&order), ["shipping.courier.address.street: required"]);
}

#[derive(Validate)]
#[allow(dead_code)]
enum Lookup<T: Validate> {
    Found(#[validate(nested)] T),
    NotFound,
}

#[test]
fn an_enum_with_a_type_parameter_validates_the_value_it_holds() {
    let lookup = Lookup::Found(Address { street: String::new() });

    assert_eq!(issues_of(&lookup), ["Found.street: required"]);
}

fn within_the_limit(refund: &Refund) -> Result<(), Problem> {
    match refund {
        Refund::Partial { cents, .. } if *cents > 10_000 => {
            Err(Problem::custom("over_the_limit", "must not exceed the limit"))
        }
        Refund::Partial { .. } | Refund::Full => Ok(()),
    }
}

#[derive(Validate)]
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
#[validate(custom(within_the_limit))]
enum Refund {
    Partial {
        #[validate(range(max = 5_000))]
        cents: u32,
        #[validate(required)]
        reason: String,
    },
    Full,
}

#[test]
fn an_enum_level_rule_reports_at_the_enum_s_location_after_the_variant_s_fields() {
    let refund = Refund::Partial {
        cents: 20_000,
        reason: String::new(),
    };

    assert_eq!(
        issues_of(&refund),
        ["partial.cents: range", "partial.reason: required", "over_the_limit"]
    );
}

#[derive(Validate)]
#[validate(report_all)]
enum Username {
    Chosen(#[validate(required, length(min = 3))] String),
}

#[test]
fn report_all_on_an_enum_reports_every_rule_a_field_breaks() {
    let username = Username::Chosen(String::new());

    assert_eq!(issues_of(&username), ["Chosen: required", "Chosen: length"]);
}
