//! Validating a valid value allocates nothing, deserializing one allocates only what its fields hold, even when it
//! collects type errors, and finding an issue allocates little.

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
