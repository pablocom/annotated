//! Validating a valid value allocates nothing.

use std::hint::black_box;

use annotated::{FoundViolations, Location, Problem, Validate};

struct Ledger {
    members: Vec<Member>,
}

struct Member {
    name: String,
}

impl Validate for Ledger {
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
    let ledger = Ledger {
        members: vec![
            Member {
                name: "Lucía".to_owned(),
            },
            Member {
                name: "Marcos".to_owned(),
            },
        ],
    };

    let allocations = allocation_counter::measure(|| {
        let _ = black_box(ledger.validate());
    });

    assert_eq!(allocations.count_total, 0);
}
