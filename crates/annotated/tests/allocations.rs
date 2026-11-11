//! Validating a valid value allocates nothing, and finding a violation allocates little.

use std::hint::black_box;

use annotated::{FoundViolations, Location, Problem, Validate};

struct Team {
    members: Vec<Member>,
}

struct Member {
    name: String,
}

impl Validate for Team {
    fn validate_into(&self, at: &Location<'_>, found: &mut FoundViolations) {
        let members = at.field("members");
        for (index, member) in self.members.iter().enumerate() {
            member.validate_into(&members.index(index), found);
        }
    }
}

impl Validate for Member {
    fn validate_into(&self, at: &Location<'_>, found: &mut FoundViolations) {
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

#[test]
fn a_violation_three_segments_deep_allocates_nothing_but_the_report() {
    let team = Team {
        members: vec![Member { name: String::new() }],
    };

    let allocations = allocation_counter::measure(|| {
        let _ = black_box(team.validate());
    });

    assert_eq!(allocations.count_total, 1);
}
