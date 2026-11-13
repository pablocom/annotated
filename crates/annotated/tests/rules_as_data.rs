//! With the `json-schema` feature, `#[derive(Validate)]` also describes a struct's rules as data: each field under the
//! name serde reads it by, with the problem a value breaking each rule has.

#![cfg(feature = "json-schema")]

use std::collections::BTreeMap;

use annotated::rules::HasLength;
use annotated::{FieldRules, Location, Problem, Report, Rules, Validate, ValueRules};

fn messages_by_field(rules: &Rules) -> Vec<(&str, Vec<String>)> {
    rules
        .fields()
        .iter()
        .map(|field| (field.name(), messages_of(field.rules())))
        .collect()
}

fn messages_of(rules: &ValueRules) -> Vec<String> {
    rules.problems().iter().map(ToString::to_string).collect()
}

fn not_reserved(handle: &str) -> Result<(), Problem> {
    if handle == "admin" {
        return Err(Problem::custom("reserved", "is reserved"));
    }
    Ok(())
}

mod my_rules {
    use annotated::{Problem, Rule};

    pub struct Even;

    pub fn even() -> Even {
        Even
    }

    impl Rule<u32> for Even {
        fn check(&self, value: &u32) -> Result<(), Problem> {
            if value.is_multiple_of(2) {
                return Ok(());
            }
            Err(Problem::custom("even", "must be even"))
        }

        fn problem(&self) -> Option<Problem> {
            Some(Problem::custom("even", "must be even"))
        }
    }
}

#[test]
fn each_field_with_rules_is_described_in_the_order_declared() {
    #[derive(Validate)]
    struct Member {
        #[validate(required)]
        full_name: String,
        #[allow(dead_code)]
        age: u8,
        #[validate(email)]
        email: String,
    }

    assert_eq!(
        Member::rules(),
        Rules::of_fields([
            FieldRules::new("full_name", ValueRules::new().with_problem(Problem::Required)),
            FieldRules::new("email", ValueRules::new().with_problem(Problem::Email)),
        ])
    );
}

#[test]
fn a_field_is_described_under_the_name_serde_reads_it_by() {
    #[derive(Validate)]
    #[serde(rename_all = "camelCase")]
    struct Member {
        #[validate(required)]
        full_name: String,
        #[validate(email)]
        #[serde(rename = "e-mail")]
        email_address: String,
    }

    assert_eq!(
        Member::rules(),
        Rules::of_fields([
            FieldRules::new("fullName", ValueRules::new().with_problem(Problem::Required)),
            FieldRules::new("e-mail", ValueRules::new().with_problem(Problem::Email)),
        ])
    );
}

#[test]
fn the_rules_of_a_field_are_described_in_the_order_written() {
    #[derive(Validate)]
    struct Subscriber {
        #[validate(required, email)]
        #[validate(ipv4)]
        contact: String,
    }

    let contact_rules = ValueRules::new()
        .with_problem(Problem::Required)
        .with_problem(Problem::Email)
        .with_problem(Problem::Ipv4);

    assert_eq!(
        Subscriber::rules(),
        Rules::of_fields([FieldRules::new("contact", contact_rules)])
    );
}

#[test]
fn a_rule_is_described_by_the_problem_a_value_breaking_it_has() {
    #[derive(Validate)]
    struct Member {
        #[validate(length(max = 50), contains = "@")]
        handle: String,
    }

    let rules = Member::rules();

    assert_eq!(
        messages_by_field(&rules),
        [(
            "handle",
            vec![
                "must be at most 50 characters long".to_owned(),
                "must contain \"@\"".to_owned()
            ]
        )]
    );
}

#[test]
fn a_rule_is_asked_about_the_type_of_the_field_it_checks() {
    #[derive(Validate)]
    struct Post {
        #[validate(length(max = 3))]
        tags: Vec<String>,
    }

    let rules = Post::rules();

    assert_eq!(
        messages_by_field(&rules),
        [("tags", vec!["must have at most 3 elements".to_owned()])]
    );
}

#[test]
fn a_rule_that_does_not_say_what_it_requires_is_not_listed() {
    #[derive(Validate)]
    struct Member {
        #[validate(custom(not_reserved), email)]
        contact: String,
        #[validate(custom(not_reserved))]
        handle: String,
    }

    assert_eq!(
        Member::rules(),
        Rules::of_fields([
            FieldRules::new("contact", ValueRules::new().with_problem(Problem::Email)),
            FieldRules::new("handle", ValueRules::new()),
        ])
    );
}

#[test]
fn a_rule_of_your_own_is_described_by_the_problem_it_says() {
    #[derive(Validate)]
    struct Pairing {
        #[validate(my_rules::even)]
        guests: u32,
    }

    assert_eq!(
        Pairing::rules(),
        Rules::of_fields([FieldRules::new(
            "guests",
            ValueRules::new().with_problem(Problem::custom("even", "must be even"))
        )])
    );
}

#[test]
fn on_an_optional_field_required_asks_for_the_option_and_other_rules_look_inside() {
    #[derive(Validate)]
    struct Profile {
        #[validate(required, length(max = 3))]
        tags: Option<Vec<String>>,
    }

    let rules = Profile::rules();

    assert_eq!(
        messages_by_field(&rules),
        [(
            "tags",
            vec!["is required".to_owned(), "must have at most 3 elements".to_owned()]
        )]
    );
}

#[test]
fn each_describes_what_it_requires_of_an_element() {
    #[derive(Validate)]
    struct Member {
        #[validate(required, each(email))]
        contacts: Vec<String>,
    }

    let contact_rules = ValueRules::new()
        .with_problem(Problem::Required)
        .with_elements(ValueRules::new().with_problem(Problem::Email));

    assert_eq!(
        Member::rules(),
        Rules::of_fields([FieldRules::new("contacts", contact_rules)])
    );
}

#[test]
fn each_on_a_map_describes_what_it_requires_of_a_value() {
    #[derive(Validate)]
    struct Member {
        #[validate(each(email))]
        contacts_by_kind: BTreeMap<String, String>,
    }

    let contact_rules = ValueRules::new().with_elements(ValueRules::new().with_problem(Problem::Email));

    assert_eq!(
        Member::rules(),
        Rules::of_fields([FieldRules::new("contacts_by_kind", contact_rules)])
    );
}

#[test]
fn several_each_are_described_together() {
    #[derive(Validate)]
    struct Member {
        #[validate(each(required), each(email))]
        contacts: Vec<String>,
    }

    let rules_of_a_contact = ValueRules::new()
        .with_problem(Problem::Required)
        .with_problem(Problem::Email);

    assert_eq!(
        Member::rules(),
        Rules::of_fields([FieldRules::new(
            "contacts",
            ValueRules::new().with_elements(rules_of_a_contact)
        )])
    );
}

#[test]
fn optional_elements_and_an_optional_collection_are_described_as_optional() {
    #[derive(Validate)]
    struct Member {
        #[validate(each(required, email))]
        contacts: Option<Vec<Option<String>>>,
    }

    let rules_of_a_contact = ValueRules::optional()
        .with_problem(Problem::Required)
        .with_problem(Problem::Email);

    assert_eq!(
        Member::rules(),
        Rules::of_fields([FieldRules::new(
            "contacts",
            ValueRules::optional().with_elements(rules_of_a_contact)
        )])
    );
}

#[test]
fn a_field_with_a_default_is_described_as_having_one() {
    #[derive(Validate)]
    struct Member {
        #[validate(required)]
        #[serde(default)]
        nickname: String,
        #[validate(required)]
        full_name: String,
    }

    assert_eq!(
        Member::rules(),
        Rules::of_fields([
            FieldRules::new("nickname", ValueRules::new().with_problem(Problem::Required)).with_default(),
            FieldRules::new("full_name", ValueRules::new().with_problem(Problem::Required)),
        ])
    );
}

#[test]
fn every_field_of_a_struct_with_a_default_is_described_as_having_one() {
    #[derive(Validate, Default)]
    #[serde(default)]
    struct Member {
        #[validate(required)]
        full_name: String,
    }

    assert_eq!(
        Member::rules(),
        Rules::of_fields([
            FieldRules::new("full_name", ValueRules::new().with_problem(Problem::Required)).with_default()
        ])
    );
}

#[test]
fn a_field_that_is_not_read_from_the_input_as_written_is_not_described() {
    #[derive(Validate)]
    struct Contact {
        #[validate(email)]
        #[serde(skip)]
        skipped: String,
        #[validate(email)]
        #[serde(deserialize_with = "read_by_a_function")]
        read_by_a_function: String,
        #[validate(email)]
        read_as_written: String,
    }

    assert_eq!(
        Contact::rules(),
        Rules::of_fields([FieldRules::new(
            "read_as_written",
            ValueRules::new().with_problem(Problem::Email)
        )])
    );
}

#[test]
fn nested_values_and_rules_across_fields_add_no_problem() {
    #[derive(Validate)]
    struct Address {
        #[validate(required)]
        street: String,
    }

    #[derive(Validate)]
    #[validate(custom(is_billed_where_they_live))]
    struct Member {
        #[validate(nested)]
        address: Address,
        #[validate(nested)]
        #[serde(flatten)]
        billing: Address,
    }

    fn is_billed_where_they_live(member: &Member) -> Result<(), Problem> {
        if member.billing.street != member.address.street {
            return Err(Problem::custom("billed_elsewhere", "must be billed where they live"));
        }
        Ok(())
    }

    assert_eq!(
        Member::rules(),
        Rules::of_fields([FieldRules::new("address", ValueRules::new())])
    );
}

#[test]
fn a_struct_without_rules_has_none_as_data() {
    #[derive(Validate)]
    struct Member {
        #[allow(dead_code)]
        full_name: String,
    }

    assert_eq!(Member::rules(), Rules::default());
}

#[test]
fn an_enum_has_no_rules_as_data_yet() {
    #[derive(Validate)]
    #[allow(dead_code)]
    enum Contact {
        Email {
            #[validate(email)]
            address: String,
        },
    }

    assert_eq!(Contact::rules(), Rules::default());
}

#[test]
fn a_type_validated_by_hand_has_no_rules_as_data() {
    struct Handle(String);

    impl Validate for Handle {
        fn validate_into(&self, at: &Location<'_>, found: &mut Report) {
            if let Err(reserved) = not_reserved(&self.0) {
                found.push(at, reserved);
            }
        }
    }

    assert_eq!(Handle::rules(), Rules::default());
}

#[test]
fn a_generic_struct_describes_its_rules_for_the_type_it_is_used_with() {
    #[derive(Validate)]
    struct Parcel<T: HasLength> {
        #[validate(length(max = 3))]
        content: T,
    }

    let (of_a_string, of_a_list) = (Parcel::<String>::rules(), Parcel::<Vec<u8>>::rules());

    assert_eq!(
        (messages_by_field(&of_a_string), messages_by_field(&of_a_list)),
        (
            vec![("content", vec!["must be at most 3 characters long".to_owned()])],
            vec![("content", vec!["must have at most 3 elements".to_owned()])]
        )
    );
}

#[test]
fn a_struct_that_borrows_describes_its_rules() {
    #[derive(Validate)]
    struct LogIn<'a> {
        #[validate(email)]
        email: &'a str,
    }

    assert_eq!(
        LogIn::rules(),
        Rules::of_fields([FieldRules::new("email", ValueRules::new().with_problem(Problem::Email))])
    );
}
