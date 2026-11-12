//! Rules on a struct's fields, checked by `#[derive(Validate)]`.

mod support;

use annotated::Validate;
use support::issues_of;

#[derive(Validate)]
struct NewMember {
    #[validate(required, length(max = 50))]
    full_name: String,
    #[validate(required, length(min = 2, max = 20))]
    nickname: String,
}

fn new_member() -> NewMember {
    NewMember {
        full_name: "Alex Morgan".to_owned(),
        nickname: "Al".to_owned(),
    }
}

#[test]
fn a_value_following_every_rule_has_no_issues() {
    assert_eq!(new_member().validate(), Ok(()));
}

#[test]
fn a_field_breaking_a_rule_is_reported_at_its_name() {
    let member = NewMember {
        nickname: "L".to_owned(),
        ..new_member()
    };

    assert_eq!(issues_of(&member), ["nickname: length"]);
}

#[test]
fn every_field_is_checked_in_the_order_declared() {
    let member = NewMember {
        full_name: String::new(),
        nickname: "L".to_owned(),
    };

    assert_eq!(issues_of(&member), ["full_name: required", "nickname: length"]);
}

#[test]
fn a_rule_is_set_up_by_the_builder_methods_its_arguments_name() {
    let member = NewMember {
        full_name: "L".repeat(51),
        ..new_member()
    };

    assert_eq!(
        member.validate().unwrap_err().to_string(),
        "full_name: must be at most 50 characters long"
    );
}

#[test]
fn the_first_rule_a_field_breaks_stops_its_other_rules() {
    let member = NewMember {
        nickname: String::new(),
        ..new_member()
    };

    assert_eq!(issues_of(&member), ["nickname: required"]);
}

#[derive(Validate)]
#[validate(report_all)]
struct NewNickname {
    #[validate(required, length(min = 2))]
    nickname: String,
}

#[test]
fn a_struct_reporting_all_reports_every_rule_a_field_breaks() {
    let new_nickname = NewNickname {
        nickname: String::new(),
    };

    assert_eq!(issues_of(&new_nickname), ["nickname: required", "nickname: length"]);
}

#[derive(Validate)]
struct NewProject {
    #[validate(
        required(message = "give the project a name"),
        length(max = 30, message = "keep it under {max} characters")
    )]
    name: String,
}

#[test]
fn a_message_replaces_the_problem_s_own() {
    let project = NewProject { name: String::new() };

    assert_eq!(
        project.validate().unwrap_err().to_string(),
        "name: give the project a name"
    );
}

#[test]
fn a_message_quotes_the_problem_s_params() {
    let project = NewProject { name: "L".repeat(31) };

    assert_eq!(
        project.validate().unwrap_err().to_string(),
        "name: keep it under 30 characters"
    );
}

#[test]
fn a_message_keeps_the_problem_s_code() {
    let project = NewProject { name: String::new() };

    assert_eq!(issues_of(&project), ["name: required"]);
}

#[derive(Validate)]
struct Tag {
    #[validate(length(max = 5, unit = "bytes"))]
    label: String,
    #[validate(length(max = 4, unit = "chars"))]
    short_label: String,
}

fn tag() -> Tag {
    Tag {
        label: "Rent".to_owned(),
        short_label: "Rent".to_owned(),
    }
}

#[test]
fn a_string_s_length_can_be_counted_in_bytes() {
    let tag = Tag {
        label: "Cafés".to_owned(),
        ..tag()
    };

    assert_eq!(
        tag.validate().unwrap_err().to_string(),
        "label: must be at most 5 bytes long"
    );
}

#[test]
fn a_string_s_length_can_be_counted_in_chars() {
    let tag = Tag {
        short_label: "Cafés".to_owned(),
        ..tag()
    };

    assert_eq!(
        tag.validate().unwrap_err().to_string(),
        "short_label: must be at most 4 characters long"
    );
}

#[cfg(feature = "unicode")]
#[derive(Validate)]
struct Flag {
    #[validate(length(max = 1, unit = "graphemes"))]
    emoji: String,
}

#[cfg(feature = "unicode")]
#[test]
fn a_string_s_length_can_be_counted_in_graphemes() {
    let flag_of_spain_of_two_chars = Flag {
        emoji: "🇪🇸".to_owned()
    };

    assert_eq!(flag_of_spain_of_two_chars.validate(), Ok(()));
}

#[derive(Validate)]
struct Page<T> {
    #[validate(length(max = 2))]
    items: Vec<T>,
}

#[test]
fn a_generic_struct_is_checked_like_any_other() {
    let page = Page { items: vec![1, 2, 3] };

    assert_eq!(issues_of(&page), ["items: length"]);
}

#[derive(Validate)]
struct Note {
    #[allow(dead_code)]
    text: String,
}

#[test]
fn a_struct_without_rules_is_always_valid() {
    let note = Note { text: String::new() };

    assert_eq!(note.validate(), Ok(()));
}
