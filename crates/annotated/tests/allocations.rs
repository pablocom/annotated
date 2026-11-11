//! Validating a valid value allocates nothing.

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
}

#[test]
fn validating_a_valid_value_by_its_derived_rules_allocates_nothing() {
    let member = NewMember {
        full_name: "Alex Morgan".to_owned(),
        nickname: Some("Al".to_owned()),
    };

    let allocations = allocation_counter::measure(|| {
        let _ = black_box(member.validate());
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
