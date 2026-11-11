//! `validate_fail_fast()` stops at the first violation and answers it alone: the first that `validate()` lists.

mod support;

use std::collections::BTreeMap;

use annotated::{Problem, Validate};
use support::{paths_and_codes, violations_of};

fn fail_fast_violations_of(value: &impl Validate) -> Vec<String> {
    paths_and_codes(value.validate_fail_fast())
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
struct NewMember {
    #[validate(required, length(max = 50))]
    full_name: String,
    #[validate(length(min = 2))]
    nickname: Option<String>,
    #[validate(email)]
    email: String,
    #[validate(range(min = 18))]
    age: u8,
}

fn new_member() -> NewMember {
    NewMember {
        full_name: "Alex Morgan".to_owned(),
        nickname: None,
        email: "alex@example.com".to_owned(),
        age: 30,
    }
}

#[test]
fn a_value_following_every_rule_has_no_violations_when_failing_fast() {
    assert_eq!(new_member().validate_fail_fast(), Ok(()));
}

#[test]
fn failing_fast_answers_only_the_first_field_that_breaks_a_rule() {
    let member = NewMember {
        email: "alex".to_owned(),
        age: 12,
        ..new_member()
    };

    assert_eq!(violations_of(&member), ["email: email", "age: range"]);
    assert_eq!(fail_fast_violations_of(&member), ["email: email"]);
}

#[test]
fn failing_fast_answers_the_value_inside_an_optional_field_when_it_breaks_first() {
    let member = NewMember {
        nickname: Some("A".to_owned()),
        age: 12,
        ..new_member()
    };

    assert_eq!(violations_of(&member), ["nickname: length", "age: range"]);
    assert_eq!(fail_fast_violations_of(&member), ["nickname: length"]);
}

#[derive(Validate)]
#[validate(report_all)]
struct Profile {
    #[validate(length(min = 3), prefix = "@")]
    handle: String,
    #[validate(each(length(min = 3), prefix = "#"))]
    tags: Vec<String>,
}

#[test]
fn failing_fast_answers_only_the_first_rule_a_field_breaks_under_report_all() {
    let profile = Profile {
        handle: "a".to_owned(),
        tags: vec!["b".to_owned()],
    };

    assert_eq!(
        violations_of(&profile),
        ["handle: length", "handle: prefix", "tags[0]: length", "tags[0]: prefix"]
    );
    assert_eq!(fail_fast_violations_of(&profile), ["handle: length"]);
}

#[test]
fn failing_fast_answers_only_the_first_rule_an_element_breaks_under_report_all() {
    let profile = Profile {
        handle: "@alex".to_owned(),
        tags: vec!["b".to_owned(), "c".to_owned()],
    };

    assert_eq!(
        violations_of(&profile),
        [
            "tags[0]: length",
            "tags[0]: prefix",
            "tags[1]: length",
            "tags[1]: prefix"
        ]
    );
    assert_eq!(fail_fast_violations_of(&profile), ["tags[0]: length"]);
}

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

fn address_without_street_or_postal_code() -> Address {
    Address {
        street: String::new(),
        postal_code: String::new(),
    }
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
struct Order {
    #[validate(nested)]
    shipping_address: Address,
    #[validate(nested)]
    billing_address: Option<Address>,
    #[validate(each(nested))]
    pickup_points: BTreeMap<String, Address>,
    #[validate(range(min = 1))]
    items: u16,
}

fn order() -> Order {
    Order {
        shipping_address: address(),
        billing_address: None,
        pickup_points: BTreeMap::new(),
        items: 1,
    }
}

#[test]
fn failing_fast_answers_only_the_first_violation_of_a_nested_struct() {
    let order = Order {
        shipping_address: address_without_street_or_postal_code(),
        items: 0,
        ..order()
    };

    assert_eq!(
        violations_of(&order),
        [
            "shippingAddress.street: required",
            "shippingAddress.postalCode: length",
            "items: range"
        ]
    );
    assert_eq!(fail_fast_violations_of(&order), ["shippingAddress.street: required"]);
}

#[test]
fn failing_fast_answers_the_first_violation_inside_an_optional_nested_struct() {
    let order = Order {
        billing_address: Some(address_without_street_or_postal_code()),
        items: 0,
        ..order()
    };

    assert_eq!(
        violations_of(&order),
        [
            "billingAddress.street: required",
            "billingAddress.postalCode: length",
            "items: range"
        ]
    );
    assert_eq!(fail_fast_violations_of(&order), ["billingAddress.street: required"]);
}

#[test]
fn failing_fast_answers_only_the_first_violation_of_the_structs_in_a_map() {
    let order = Order {
        pickup_points: BTreeMap::from([
            ("depot".to_owned(), address_without_street_or_postal_code()),
            ("locker".to_owned(), address_without_street_or_postal_code()),
        ]),
        items: 0,
        ..order()
    };

    assert_eq!(
        violations_of(&order),
        [
            "pickupPoints[depot].street: required",
            "pickupPoints[depot].postalCode: length",
            "pickupPoints[locker].street: required",
            "pickupPoints[locker].postalCode: length",
            "items: range"
        ]
    );
    assert_eq!(
        fail_fast_violations_of(&order),
        ["pickupPoints[depot].street: required"]
    );
}

#[derive(Validate)]
struct Post {
    #[validate(each(required, length(max = 10)))]
    tags: Vec<String>,
    #[validate(each(each(length(max = 3))))]
    grid: Vec<Vec<String>>,
}

#[test]
fn failing_fast_answers_only_the_first_element_that_breaks_a_rule() {
    let post = Post {
        tags: vec!["rust".to_owned(), String::new(), "validation-rules".to_owned()],
        grid: vec![vec!["abcd".to_owned()]],
    };

    assert_eq!(
        violations_of(&post),
        ["tags[1]: required", "tags[2]: length", "grid[0][0]: length"]
    );
    assert_eq!(fail_fast_violations_of(&post), ["tags[1]: required"]);
}

#[test]
fn failing_fast_answers_only_the_first_element_of_an_element_that_breaks_a_rule() {
    let post = Post {
        tags: Vec::new(),
        grid: vec![
            vec!["ab".to_owned(), "abcd".to_owned(), "abcde".to_owned()],
            vec!["abcd".to_owned()],
        ],
    };

    assert_eq!(
        violations_of(&post),
        ["grid[0][1]: length", "grid[0][2]: length", "grid[1][0]: length"]
    );
    assert_eq!(fail_fast_violations_of(&post), ["grid[0][1]: length"]);
}

fn passwords_match(new_password: &NewPassword) -> Result<(), Problem> {
    if new_password.repeated_password != new_password.password {
        return Err(Problem::custom("passwords_differ", "must match the password"));
    }
    Ok(())
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
#[validate(custom(passwords_match, field = repeated_password))]
struct NewPassword {
    #[validate(length(min = 8))]
    password: String,
    #[validate(required)]
    repeated_password: String,
}

#[test]
fn failing_fast_answers_a_field_s_own_rule_before_a_struct_level_rule_at_a_field() {
    let new_password = NewPassword {
        password: "horse".to_owned(),
        repeated_password: "battery".to_owned(),
    };

    assert_eq!(
        violations_of(&new_password),
        ["password: length", "repeatedPassword: passwords_differ"]
    );
    assert_eq!(fail_fast_violations_of(&new_password), ["password: length"]);
}

fn youngest_is_not_above_oldest(limits: &AgeLimits) -> Result<(), Problem> {
    if limits.youngest > limits.oldest {
        return Err(Problem::custom(
            "youngest_above_oldest",
            "must not have its youngest age above its oldest",
        ));
    }
    Ok(())
}

fn oldest_is_an_adult(limits: &AgeLimits) -> Result<(), Problem> {
    if limits.oldest < 18 {
        return Err(Problem::custom("not_an_adult", "must be 18 or older"));
    }
    Ok(())
}

#[derive(Validate)]
#[validate(custom(youngest_is_not_above_oldest))]
#[validate(custom(oldest_is_an_adult, field = oldest))]
struct AgeLimits {
    #[validate(range(max = 120))]
    youngest: u8,
    oldest: u8,
}

#[test]
fn failing_fast_answers_a_field_s_own_rule_before_a_struct_level_rule_at_the_struct() {
    let limits = AgeLimits {
        youngest: 130,
        oldest: 18,
    };

    assert_eq!(violations_of(&limits), ["youngest: range", "youngest_above_oldest"]);
    assert_eq!(fail_fast_violations_of(&limits), ["youngest: range"]);
}

#[test]
fn failing_fast_answers_the_first_struct_level_rule_written_that_breaks() {
    let limits = AgeLimits {
        youngest: 16,
        oldest: 15,
    };

    assert_eq!(
        violations_of(&limits),
        ["youngest_above_oldest", "oldest: not_an_adult"]
    );
    assert_eq!(fail_fast_violations_of(&limits), ["youngest_above_oldest"]);
}

fn must_not_be_checked(_: &str) -> Result<(), Problem> {
    panic!("checked after failing fast found its violation")
}

#[derive(Validate)]
struct Signpost {
    #[validate(required)]
    destination: String,
    #[validate(custom(must_not_be_checked))]
    note: String,
}

#[test]
fn failing_fast_checks_no_field_after_the_violation() {
    let signpost = Signpost {
        destination: String::new(),
        note: String::new(),
    };

    assert_eq!(fail_fast_violations_of(&signpost), ["destination: required"]);
}

#[derive(Validate)]
struct Parcel {
    #[validate(nested)]
    sender: Address,
    #[validate(required)]
    note: String,
}

#[test]
fn failing_fast_records_nothing_after_a_nested_struct_found_its_violation() {
    let parcel = Parcel {
        sender: address_without_street_or_postal_code(),
        note: String::new(),
    };

    assert_eq!(fail_fast_violations_of(&parcel), ["sender.street: required"]);
}

#[derive(Validate)]
struct Route {
    #[validate(each(required, custom(must_not_be_checked)))]
    stops: Vec<String>,
}

#[test]
fn failing_fast_checks_no_element_after_the_violation() {
    let route = Route {
        stops: vec![String::new(), "Lyon".to_owned()],
    };

    assert_eq!(fail_fast_violations_of(&route), ["stops[0]: required"]);
}

#[derive(Validate)]
struct Journey {
    #[validate(each(required))]
    stops: Vec<String>,
    #[validate(custom(must_not_be_checked))]
    note: String,
}

#[test]
fn failing_fast_checks_no_field_after_a_violation_in_an_element() {
    let journey = Journey {
        stops: vec![String::new()],
        note: String::new(),
    };

    assert_eq!(fail_fast_violations_of(&journey), ["stops[0]: required"]);
}

#[derive(Validate)]
#[validate(report_all)]
struct Poster {
    #[validate(required, custom(must_not_be_checked))]
    title: String,
}

#[test]
fn failing_fast_checks_no_rule_after_the_violation_under_report_all() {
    let poster = Poster { title: String::new() };

    assert_eq!(fail_fast_violations_of(&poster), ["title: required"]);
}

#[derive(Validate)]
#[validate(report_all)]
struct Playlist {
    #[validate(each(required, custom(must_not_be_checked)))]
    tracks: Vec<String>,
}

#[test]
fn failing_fast_checks_no_rule_of_an_element_after_the_violation_under_report_all() {
    let playlist = Playlist {
        tracks: vec![String::new()],
    };

    assert_eq!(fail_fast_violations_of(&playlist), ["tracks[0]: required"]);
}
