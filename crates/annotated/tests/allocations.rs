//! Validating a valid value allocates nothing, deserializing one allocates only what its fields hold, even when it
//! collects type errors, buffering it once however deeply such types nest, and finding an issue allocates little.

use std::collections::BTreeMap;
use std::hint::black_box;

use annotated::{Location, Problem, Report, Validate};

struct Team {
    members: Vec<Member>,
}

struct Member {
    name: String,
}

impl Validate for Team {
    fn validate_into(&self, at: &Location<'_>, found: &mut Report) {
        let members = at.field("members");
        for (index, member) in self.members.iter().enumerate() {
            member.validate_into(&members.index(index), found);
        }
    }
}

impl Validate for Member {
    fn validate_into(&self, at: &Location<'_>, found: &mut Report) {
        if self.name.is_empty() {
            found.push(&at.field("name"), Problem::custom("empty_name", "is empty"));
        }
    }
}

#[test]
fn validating_a_valid_value_by_hand_allocates_nothing() {
    let team = Team {
        members: vec![
            Member {
                name: "Renée".to_owned(),
            },
            Member { name: "Sam".to_owned() },
        ],
    };

    let allocations = allocation_counter::measure(|| {
        let _ = black_box(team.validate());
    });

    assert_eq!(allocations.count_total, 0);
}

#[derive(Validate)]
struct NewMember {
    #[validate(required, length(max = 50))]
    full_name: String,
    #[validate(length(min = 2))]
    nickname: Option<String>,
    #[validate(url)]
    website: String,
}

#[test]
fn validating_a_valid_value_by_its_derived_rules_allocates_nothing() {
    let member = NewMember {
        full_name: "Alex Morgan".to_owned(),
        nickname: Some("Al".to_owned()),
        website: "https://example.com/alex".to_owned(),
    };

    let allocations = allocation_counter::measure(|| {
        let _ = black_box(member.validate());
    });

    assert_eq!(allocations.count_total, 0);
}

#[test]
fn validating_a_valid_value_failing_fast_allocates_nothing() {
    let member = NewMember {
        full_name: "Alex Morgan".to_owned(),
        nickname: Some("Al".to_owned()),
        website: "https://example.com/alex".to_owned(),
    };

    let allocations = allocation_counter::measure(|| {
        let _ = black_box(member.validate_fail_fast());
    });

    assert_eq!(allocations.count_total, 0);
}

#[cfg(feature = "regex")]
#[derive(Validate)]
struct Address {
    #[validate(pattern = r"^\d{5}$")]
    postal_code: String,
}

#[cfg(feature = "regex")]
#[test]
fn a_pattern_is_compiled_once_and_validating_by_it_again_allocates_nothing() {
    let address = Address {
        postal_code: "28013".to_owned(),
    };
    let _ = address.validate();

    let allocations = allocation_counter::measure(|| {
        let _ = black_box(address.validate());
    });

    assert_eq!(allocations.count_total, 0);
}

fn passwords_match(new_password: &NewPassword) -> Result<(), Problem> {
    if new_password.repeated_password != new_password.password {
        return Err(Problem::custom("passwords_differ", "must match the password"));
    }
    Ok(())
}

#[derive(Validate)]
#[validate(custom(passwords_match, field = repeated_password))]
struct NewPassword {
    #[validate(length(min = 8))]
    password: String,
    #[validate(required)]
    repeated_password: String,
}

#[test]
fn validating_a_valid_value_by_its_struct_level_rules_allocates_nothing() {
    let new_password = NewPassword {
        password: "correct horse battery staple".to_owned(),
        repeated_password: "correct horse battery staple".to_owned(),
    };

    let allocations = allocation_counter::measure(|| {
        let _ = black_box(new_password.validate());
    });

    assert_eq!(allocations.count_total, 0);
}

#[cfg(feature = "serde")]
#[derive(annotated::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct SignUp {
    #[validate(required, length(max = 50))]
    full_name: String,
    #[validate(length(min = 2))]
    nickname: Option<String>,
    #[validate(email)]
    email: String,
    #[validate(range(min = 18))]
    age: u8,
}

#[cfg(feature = "serde")]
#[test]
fn deserializing_a_valid_value_allocates_only_its_three_strings() {
    let body = r#"{ "fullName": "Alex Morgan", "nickname": "Al", "email": "alex@example.com", "age": 30 }"#;

    let allocations = allocation_counter::measure(|| {
        let mut deserializer = serde_json::Deserializer::from_str(black_box(body));
        let _ = black_box(annotated::from_deserializer::<SignUp, _>(&mut deserializer));
    });

    assert_eq!(allocations.count_total, 3);
}

#[cfg(feature = "serde")]
#[derive(annotated::Deserialize)]
#[serde(rename_all = "camelCase")]
#[annotated(collect_type_errors)]
#[allow(dead_code)]
struct SignUpCollectingTypeErrors {
    #[validate(required, length(max = 50))]
    full_name: String,
    #[validate(length(min = 2))]
    nickname: Option<String>,
    #[validate(email)]
    email: String,
    #[validate(range(min = 18))]
    age: u8,
    plan: Plan,
}

#[cfg(feature = "serde")]
#[derive(serde::Deserialize)]
enum Plan {
    Free,
    Family,
}

#[cfg(feature = "serde")]
#[test]
fn deserializing_a_valid_value_collecting_type_errors_borrows_what_it_buffers_and_allocates_only_its_three_strings() {
    let body =
        r#"{ "fullName": "Alex Morgan", "nickname": "Al", "email": "alex@example.com", "age": 30, "plan": "Family" }"#;

    let allocations = allocation_counter::measure(|| {
        let mut deserializer = serde_json::Deserializer::from_str(black_box(body));
        let _ = black_box(annotated::from_deserializer::<SignUpCollectingTypeErrors, _>(
            &mut deserializer,
        ));
    });

    assert_eq!(allocations.count_total, 3);
}

#[cfg(feature = "serde")]
#[derive(annotated::Deserialize)]
#[serde(rename_all = "camelCase")]
#[annotated(collect_type_errors)]
#[allow(dead_code)]
struct Delivery {
    #[validate(required)]
    reference: String,
    #[validate(nested)]
    address: DeliveryAddress,
}

#[cfg(feature = "serde")]
#[derive(annotated::Deserialize)]
#[serde(rename_all = "camelCase")]
#[annotated(collect_type_errors)]
#[allow(dead_code)]
struct DeliveryAddress {
    #[validate(required)]
    street: String,
    #[validate(length(min = 5))]
    postal_code: String,
    #[validate(length(max = 3))]
    notes: Vec<String>,
}

#[cfg(feature = "serde")]
#[test]
fn deserializing_a_valid_nested_value_collecting_type_errors_buffers_it_once() {
    let body = r#"{ "reference": "PX-1", "address": { "street": "Gran Vía 1", "postalCode": "28013",
        "notes": ["Ring twice", "Leave at the door"] } }"#;

    let allocations = allocation_counter::measure(|| {
        let mut deserializer = serde_json::Deserializer::from_str(black_box(body));
        let _ = black_box(annotated::from_deserializer::<Delivery, _>(&mut deserializer));
    });

    assert_eq!(allocations.count_total, 8);
}

#[cfg(feature = "serde")]
#[test]
fn finding_a_value_of_the_wrong_type_in_a_nested_value_collecting_type_errors_allocates_its_issue_once() {
    let body = r#"{ "reference": "PX-1", "address": { "street": 5, "postalCode": "28013", "notes": [] } }"#;

    let allocations = allocation_counter::measure(|| {
        let mut deserializer = serde_json::Deserializer::from_str(black_box(body));
        let _ = black_box(annotated::from_deserializer::<Delivery, _>(&mut deserializer));
    });

    assert_eq!(allocations.count_total, 5);
}

#[cfg(feature = "serde")]
#[derive(annotated::Deserialize)]
#[annotated(collect_type_errors)]
#[allow(dead_code)]
struct Node {
    #[validate(length(max = 100))]
    name: String,
    #[validate(each(nested))]
    children: Vec<Node>,
}

/// A chain of `depth` nodes, each the only child of the one before, whose last node has `leaves` children of its own.
#[cfg(feature = "serde")]
fn nodes_nested(depth: usize, leaves: usize) -> String {
    let leaves = vec![r#"{ "name": "leaf", "children": [] }"#; leaves].join(", ");
    let opening = r#"{ "name": "branch", "children": ["#.repeat(depth);
    let closing = "] }".repeat(depth);
    format!("{opening}{leaves}{closing}")
}

#[cfg(feature = "serde")]
#[test]
fn deserializing_structs_collecting_type_errors_nested_sixty_deep_allocates_about_as_much_as_one_deep() {
    let one_deep = nodes_nested(1, 1000);
    let sixty_deep = nodes_nested(60, 1000);

    let allocated_one_deep = allocation_counter::measure(|| {
        let mut deserializer = serde_json::Deserializer::from_str(black_box(&one_deep));
        let _ = black_box(annotated::from_deserializer::<Node, _>(&mut deserializer));
    });
    let allocated_sixty_deep = allocation_counter::measure(|| {
        let mut deserializer = serde_json::Deserializer::from_str(black_box(&sixty_deep));
        let _ = black_box(annotated::from_deserializer::<Node, _>(&mut deserializer));
    });

    assert_eq!(
        (allocated_one_deep.count_total, allocated_sixty_deep.count_total),
        (2011, 2247)
    );
}

#[cfg(feature = "serde")]
#[derive(annotated::Deserialize)]
#[serde(rename_all = "camelCase")]
#[annotated(collect_type_errors)]
#[allow(dead_code)]
enum Expression {
    Literal(#[validate(range(max = 100))] u32),
    Sum(#[validate(each(nested))] Vec<Expression>),
}

/// A chain of `depth` sums, each the only term of the one before, whose last sum has `literals` terms of its own.
#[cfg(feature = "serde")]
fn sums_nested(depth: usize, literals: usize) -> String {
    let literals = vec![r#"{ "literal": 1 }"#; literals].join(", ");
    let opening = r#"{ "sum": ["#.repeat(depth);
    let closing = "] }".repeat(depth);
    format!("{opening}{literals}{closing}")
}

#[cfg(feature = "serde")]
#[test]
fn deserializing_enums_collecting_type_errors_nested_sixty_deep_allocates_about_as_much_as_one_deep() {
    let one_deep = sums_nested(1, 1000);
    let sixty_deep = sums_nested(60, 1000);

    let allocated_one_deep = allocation_counter::measure(|| {
        let mut deserializer = serde_json::Deserializer::from_str(black_box(&one_deep));
        let _ = black_box(annotated::from_deserializer::<Expression, _>(&mut deserializer));
    });
    let allocated_sixty_deep = allocation_counter::measure(|| {
        let mut deserializer = serde_json::Deserializer::from_str(black_box(&sixty_deep));
        let _ = black_box(annotated::from_deserializer::<Expression, _>(&mut deserializer));
    });

    assert_eq!(
        (allocated_one_deep.count_total, allocated_sixty_deep.count_total),
        (1011, 1188)
    );
}

#[cfg(feature = "serde")]
#[derive(annotated::Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
enum Contact {
    Email {
        #[validate(email)]
        address: String,
    },
    Phone(#[validate(length(min = 9))] String),
    Unlisted,
}

#[cfg(feature = "serde")]
#[test]
fn deserializing_a_valid_externally_tagged_enum_buffers_nothing_and_allocates_only_its_string() {
    let body = r#"{ "email": { "address": "alex@example.com" } }"#;

    let allocations = allocation_counter::measure(|| {
        let mut deserializer = serde_json::Deserializer::from_str(black_box(body));
        let _ = black_box(annotated::from_deserializer::<Contact, _>(&mut deserializer));
    });

    assert_eq!(allocations.count_total, 1);
}

#[cfg(feature = "serde")]
#[derive(annotated::Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
#[allow(dead_code)]
enum Event {
    SignedUp {
        #[validate(email)]
        email: String,
    },
    LoggedOut,
}

#[cfg(feature = "serde")]
#[test]
fn deserializing_a_valid_adjacently_tagged_enum_with_its_tag_first_buffers_nothing_and_allocates_only_its_string() {
    let body = r#"{ "type": "signedUp", "data": { "email": "alex@example.com" } }"#;

    let allocations = allocation_counter::measure(|| {
        let mut deserializer = serde_json::Deserializer::from_str(black_box(body));
        let _ = black_box(annotated::from_deserializer::<Event, _>(&mut deserializer));
    });

    assert_eq!(allocations.count_total, 1);
}

#[cfg(feature = "serde")]
#[derive(annotated::Deserialize)]
#[serde(untagged)]
#[allow(dead_code)]
enum Recipient {
    ByEmail {
        #[validate(email)]
        email: String,
    },
    ByPhone {
        #[validate(length(min = 9))]
        phone: String,
    },
}

#[cfg(feature = "serde")]
#[test]
fn deserializing_a_valid_untagged_enum_buffers_it_once_and_allocates_only_the_buffer_and_its_string() {
    let body = r#"{ "email": "alex@example.com" }"#;

    let allocations = allocation_counter::measure(|| {
        let mut deserializer = serde_json::Deserializer::from_str(black_box(body));
        let _ = black_box(annotated::from_deserializer::<Recipient, _>(&mut deserializer));
    });

    assert_eq!(allocations.count_total, 2);
}

#[cfg(feature = "serde")]
#[derive(annotated::Deserialize)]
#[allow(dead_code)]
struct Crew {
    #[validate(nested)]
    captain: CrewMember,
    #[validate(each(nested))]
    sailors: Vec<CrewMember>,
}

#[cfg(feature = "serde")]
#[derive(annotated::Deserialize)]
#[allow(dead_code)]
struct CrewMember {
    #[validate(required)]
    name: String,
}

#[cfg(feature = "serde")]
#[test]
fn deserializing_a_valid_nested_value_allocates_only_its_three_strings_and_its_list() {
    let body = r#"{ "captain": { "name": "Alex" }, "sailors": [{ "name": "Sam" }, { "name": "Lu" }] }"#;

    let allocations = allocation_counter::measure(|| {
        let mut deserializer = serde_json::Deserializer::from_str(black_box(body));
        let _ = black_box(annotated::from_deserializer::<Crew, _>(&mut deserializer));
    });

    assert_eq!(allocations.count_total, 4);
}

#[cfg(feature = "serde")]
#[test]
fn a_map_read_into_a_finished_report_allocates_only_its_keys_and_room_for_the_values_it_could_not_build() {
    let body = r#"{ "first": {}, "second": {} }"#;
    let mut found = Report::failing_fast();
    found.push(&Location::root(), Problem::custom("taken", "is taken"));

    let allocations = allocation_counter::measure(|| {
        let mut deserializer = serde_json::Deserializer::from_str(black_box(body));
        let _ = black_box(
            <std::collections::BTreeMap<String, CrewMember> as annotated::Deserialize>::deserialize_at(
                &mut deserializer,
                &Location::root(),
                &mut found,
            ),
        );
    });

    assert_eq!(allocations.count_total, 3);
}

#[test]
fn issues_appended_to_an_empty_report_are_moved_without_allocating() {
    let root = Location::root();
    let mut found_in_a_part = Report::new();
    found_in_a_part.push(&root.field("name"), Problem::custom("empty_name", "is empty"));
    let mut found = Report::new();

    let allocations = allocation_counter::measure(|| {
        found.append(black_box(&mut found_in_a_part));
    });

    assert_eq!(allocations.count_total, 0);
}

#[test]
fn an_issue_three_segments_deep_allocates_nothing_but_the_report() {
    let team = Team {
        members: vec![Member { name: String::new() }],
    };

    let allocations = allocation_counter::measure(|| {
        let _ = black_box(team.validate());
    });

    assert_eq!(allocations.count_total, 1);
}

#[derive(Validate)]
struct Shipment {
    #[validate(each(each(range(min = 1))))]
    quantities: BTreeMap<String, Vec<u32>>,
}

#[test]
fn a_thousand_issues_under_a_100_kb_key_hold_only_its_first_256_chars() {
    let shipment = Shipment {
        quantities: BTreeMap::from([("k".repeat(100_000), vec![0; 1_000])]),
    };

    let allocations = allocation_counter::measure(|| {
        let _ = black_box(shipment.validate());
    });

    assert_eq!(allocations.bytes_max, 692_480);
}
