//! With the `json-schema` feature, `#[derive(Deserialize)]` describes a struct's rules as data, as `#[derive(Validate)]`
//! does.

#![cfg(all(feature = "json-schema", feature = "serde"))]
// The types here are only described, never read.
#![allow(dead_code)]

use annotated::rules::HasLength;
use annotated::{Deserialize, FieldRules, Location, Problem, Report, Rules, Validate, ValueRules};
use serde::Deserializer;

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

fn trimmed<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let text: String = serde::Deserialize::deserialize(deserializer)?;
    Ok(text.trim().to_owned())
}

#[test]
fn a_struct_read_collecting_its_issues_describes_its_rules() {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Member {
        #[validate(required, length(max = 50))]
        full_name: String,
        #[validate(each(email))]
        contacts: Option<Vec<String>>,
    }

    let rules = <Member as Deserialize>::rules();

    assert_eq!(
        (messages_by_field(&rules), rules.fields().get(1).map(FieldRules::rules)),
        (
            vec![
                (
                    "fullName",
                    vec![
                        "is required".to_owned(),
                        "must be at most 50 characters long".to_owned()
                    ]
                ),
                ("contacts", Vec::new())
            ],
            Some(&ValueRules::optional().with_elements(ValueRules::new().with_problem(Problem::Email)))
        )
    );
}

#[test]
fn a_field_that_takes_a_default_is_described_as_having_one() {
    #[derive(Deserialize)]
    struct Member {
        #[validate(required)]
        #[serde(default)]
        nickname: String,
        #[validate(required)]
        full_name: String,
    }

    assert_eq!(
        <Member as Deserialize>::rules(),
        Rules::of_fields([
            FieldRules::new("nickname", ValueRules::new().with_problem(Problem::Required)).with_default(),
            FieldRules::new("full_name", ValueRules::new().with_problem(Problem::Required)),
        ])
    );
}

#[test]
fn every_field_of_a_struct_that_takes_its_default_is_described_as_having_one() {
    #[derive(Deserialize, Default)]
    #[serde(default)]
    struct Member {
        #[validate(required)]
        full_name: String,
    }

    assert_eq!(
        <Member as Deserialize>::rules(),
        Rules::of_fields([
            FieldRules::new("full_name", ValueRules::new().with_problem(Problem::Required)).with_default()
        ])
    );
}

#[test]
fn a_field_that_is_not_read_from_the_input_as_written_is_not_described() {
    #[derive(Deserialize, Validate)]
    struct Address {
        #[validate(required)]
        street: String,
    }

    #[derive(Deserialize)]
    struct Contact {
        #[validate(email)]
        #[serde(skip)]
        skipped: String,
        #[validate(email)]
        #[serde(deserialize_with = "trimmed")]
        read_by_a_function: String,
        #[validate(nested)]
        #[serde(flatten)]
        flattened: Address,
        #[validate(email)]
        read_as_written: String,
    }

    assert_eq!(
        <Contact as Deserialize>::rules(),
        Rules::of_fields([FieldRules::new(
            "read_as_written",
            ValueRules::new().with_problem(Problem::Email)
        )])
    );
}

#[test]
fn a_struct_deriving_both_traits_describes_the_same_rules_through_each() {
    #[derive(Validate, Deserialize)]
    struct Member {
        #[validate(required, email)]
        email: Option<String>,
    }

    let email_rules = ValueRules::optional()
        .with_problem(Problem::Required)
        .with_problem(Problem::Email);

    assert_eq!(
        (<Member as Validate>::rules(), <Member as Deserialize>::rules()),
        (
            Rules::of_fields([FieldRules::new("email", email_rules.clone())]),
            Rules::of_fields([FieldRules::new("email", email_rules)])
        )
    );
}

#[test]
fn a_generic_struct_describes_its_rules_for_the_type_it_is_used_with() {
    #[derive(Deserialize)]
    struct Parcel<T: HasLength> {
        #[validate(length(max = 3))]
        content: T,
    }

    let (of_a_string, of_a_list) = (
        <Parcel<String> as Deserialize>::rules(),
        <Parcel<Vec<u8>> as Deserialize>::rules(),
    );

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
    #[derive(Deserialize)]
    struct LogIn<'a> {
        #[validate(email)]
        email: &'a str,
    }

    assert_eq!(
        <LogIn as Deserialize>::rules(),
        Rules::of_fields([FieldRules::new("email", ValueRules::new().with_problem(Problem::Email))])
    );
}

#[test]
fn an_enum_has_no_rules_as_data_yet() {
    #[derive(Deserialize)]
    enum Contact {
        Email {
            #[validate(email)]
            address: String,
        },
    }

    assert_eq!(<Contact as Deserialize>::rules(), Rules::default());
}

#[test]
fn a_type_read_by_hand_has_no_rules_as_data() {
    struct Handle(String);

    impl<'de> Deserialize<'de> for Handle {
        fn deserialize_at<D: Deserializer<'de>>(
            deserializer: D,
            _: &Location<'_>,
            _: &mut Report,
        ) -> Result<Option<Self>, D::Error> {
            let handle: String = serde::Deserialize::deserialize(deserializer)?;
            Ok(Some(Self(handle)))
        }
    }

    assert_eq!(<Handle as Deserialize>::rules(), Rules::default());
}
