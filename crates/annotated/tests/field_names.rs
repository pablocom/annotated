//! Fields are reported by the name serde reads them by.

mod support;

use annotated::Validate;
use support::issues_of;

#[derive(Validate)]
struct LogIn {
    #[validate(required)]
    email: String,
}

#[test]
fn a_field_without_serde_attributes_is_reported_by_its_rust_name() {
    let log_in = LogIn { email: String::new() };

    assert_eq!(issues_of(&log_in), ["email: required"]);
}

#[derive(Validate)]
struct Expense {
    #[validate(required)]
    r#type: String,
}

#[test]
fn a_raw_identifier_is_reported_without_its_prefix() {
    let expense = Expense { r#type: String::new() };

    assert_eq!(issues_of(&expense), ["type: required"]);
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
struct NewMember {
    #[validate(required)]
    full_name: String,
}

#[test]
fn a_field_is_reported_by_the_name_its_struct_s_rename_all_gives_it() {
    let member = NewMember {
        full_name: String::new(),
    };

    assert_eq!(issues_of(&member), ["fullName: required"]);
}

#[derive(Validate)]
struct Contact {
    #[serde(rename = "e-mail")]
    #[validate(required)]
    email: String,
}

#[test]
fn a_renamed_field_is_reported_by_its_new_name() {
    let contact = Contact { email: String::new() };

    assert_eq!(issues_of(&contact), ["e-mail: required"]);
}

#[derive(Validate)]
struct Subscription {
    #[serde(rename(serialize = "mail", deserialize = "e-mail"))]
    #[validate(required)]
    email: String,
}

#[test]
fn a_field_renamed_apart_for_deserializing_is_reported_by_the_name_it_is_read_by() {
    let subscription = Subscription { email: String::new() };

    assert_eq!(issues_of(&subscription), ["e-mail: required"]);
}

#[derive(Validate)]
struct Invite {
    #[serde(alias = "mail")]
    #[validate(required)]
    email: String,
}

#[test]
fn a_field_with_an_alias_is_reported_by_its_name() {
    let invite = Invite { email: String::new() };

    assert_eq!(issues_of(&invite), ["email: required"]);
}

#[derive(Validate)]
#[serde(an_attribute_of_a_later_serde, rename_all = "camelCase")]
struct Newsletter {
    #[serde(rename_all_in_a_later_serde, rename = "e-mail")]
    #[validate(required)]
    email: String,
    #[serde(rename = 7)]
    #[validate(required)]
    sent_to_list: String,
}

#[test]
fn a_serde_attribute_validate_can_t_read_is_left_out_and_those_beside_it_are_read() {
    let newsletter = Newsletter {
        email: String::new(),
        sent_to_list: String::new(),
    };

    assert_eq!(issues_of(&newsletter), ["e-mail: required", "sentToList: required"]);
}

#[derive(Validate)]
#[serde(transparent)]
struct EmailAddress {
    #[validate(required)]
    address: String,
    #[serde(skip)]
    #[validate(length(max = 0))]
    verified_by: Vec<String>,
}

#[derive(Validate)]
struct Recipient {
    #[validate(nested)]
    email: EmailAddress,
}

#[test]
fn the_field_of_a_transparent_struct_is_reported_where_the_struct_is() {
    let recipient = Recipient {
        email: EmailAddress {
            address: String::new(),
            verified_by: Vec::new(),
        },
    };

    assert_eq!(issues_of(&recipient), ["email: required"]);
}

#[test]
fn a_skipped_field_of_a_transparent_struct_is_reported_by_its_name() {
    let recipient = Recipient {
        email: EmailAddress {
            address: "lucia@example.com".to_owned(),
            verified_by: vec!["admin".to_owned()],
        },
    };

    assert_eq!(issues_of(&recipient), ["email.verified_by: length"]);
}
