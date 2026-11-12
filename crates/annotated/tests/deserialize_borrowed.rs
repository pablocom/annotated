//! `#[derive(Deserialize)]` reads a struct with lifetime parameters, whose fields borrow from the input as they do
//! with serde: a `&str` or `&[u8]` field always, any other field with `#[serde(borrow)]`.

#![cfg(feature = "serde")]

use std::borrow::Cow;

use annotated::{Deserialize, DeserializeError, Issue, Validate};

fn deserialize<'de, T: Deserialize<'de>>(json: &'de str) -> Result<T, DeserializeError<serde_json::Error>> {
    annotated::from_deserializer(&mut serde_json::Deserializer::from_str(json))
}

fn issues_found_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<Issue> {
    match deserialize::<T>(json) {
        Ok(_) => Vec::new(),
        Err(DeserializeError::Issues(issues)) => issues.into_iter().collect(),
        Err(error) => panic!("expected issues, got the format's error: {error}"),
    }
}

fn issues_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<String> {
    issues_found_reading::<T>(json).iter().map(described).collect()
}

fn described(issue: &Issue) -> String {
    let params = issue.problem().params();
    let params = params
        .iter()
        .map(|(name, value)| format!("{name}: {value}"))
        .collect::<Vec<_>>();
    let code_and_params = if params.is_empty() {
        issue.problem().code().to_owned()
    } else {
        format!("{}({})", issue.problem().code(), params.join(", "))
    };
    format!("{}: {code_and_params}", issue.path())
}

fn format_error_reading<'de, T: Deserialize<'de>>(json: &'de str) -> String {
    match deserialize::<T>(json) {
        Err(error @ DeserializeError::Format(_)) => error.to_string(),
        Ok(_) => panic!("expected the format's error, got a value"),
        Err(error) => panic!("expected the format's error, got {error:?}"),
    }
}

fn issues_validating_after_serde<'de, T: serde::Deserialize<'de> + Validate>(json: &'de str) -> Vec<Issue> {
    let value = serde_json::from_str::<T>(json).unwrap();
    value
        .validate()
        .map_or_else(|issues| issues.into_iter().collect(), |()| Vec::new())
}

#[derive(Deserialize, Debug, PartialEq)]
struct LogIn<'a> {
    #[validate(email)]
    email: &'a str,
    #[validate(length(min = 8))]
    password: &'a [u8],
}

#[test]
fn a_str_and_a_byte_slice_borrow_from_the_input() {
    let body = r#"{ "email": "ada@example.com", "password": "correct horse" }"#;

    let log_in = deserialize::<LogIn>(body).unwrap();

    assert_eq!(
        log_in,
        LogIn {
            email: "ada@example.com",
            password: b"correct horse",
        }
    );
}

#[test]
fn a_borrowed_value_is_checked_by_its_field_s_rules() {
    let body = r#"{ "email": "ada", "password": "horse" }"#;

    assert_eq!(
        issues_reading::<LogIn>(body),
        ["email: email", "password: length(min: 8, unit: elements)"]
    );
}

#[test]
fn an_absent_borrowed_field_is_missing() {
    assert_eq!(
        issues_reading::<LogIn>(r#"{ "password": "correct horse" }"#),
        ["email: missing"]
    );
}

#[test]
fn a_str_the_input_can_t_lend_is_the_format_s_error() {
    let body = r#"{ "email": "ada\u0040example.com", "password": "correct horse" }"#;

    assert_eq!(
        format_error_reading::<LogIn>(body),
        r#"invalid type: string "ada@example.com", expected a borrowed string at line 1 column 33"#
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[annotated(collect_type_errors)]
struct LogInCollectingTypeErrors<'a> {
    #[validate(email)]
    email: &'a str,
    #[validate(required)]
    device: &'a str,
}

#[test]
fn under_collect_type_errors_a_str_borrows_from_the_input() {
    let log_in =
        deserialize::<LogInCollectingTypeErrors>(r#"{ "email": "ada@example.com", "device": "phone" }"#).unwrap();

    assert_eq!(
        log_in,
        LogInCollectingTypeErrors {
            email: "ada@example.com",
            device: "phone",
        }
    );
}

#[test]
fn under_collect_type_errors_a_str_the_input_can_t_lend_is_an_issue_at_its_field() {
    let body = r#"{ "email": "ada\u0040example.com", "device": "" }"#;

    assert_eq!(
        issues_reading::<LogInCollectingTypeErrors>(body),
        [
            "email: invalid_type(expected: a borrowed string, found: string)",
            "device: required"
        ]
    );
}

#[derive(Deserialize, Debug)]
struct Greeting<'a> {
    #[serde(borrow)]
    #[validate(length(max = 10))]
    text: Cow<'a, str>,
}

#[test]
fn a_cow_with_borrow_borrows_from_the_input() {
    let greeting = deserialize::<Greeting>(r#"{ "text": "Hola" }"#).unwrap();

    let Cow::Borrowed(text) = greeting.text else {
        panic!("expected a borrowed text, got {:?}", greeting.text);
    };
    assert_eq!(text, "Hola");
}

#[test]
fn a_cow_with_borrow_owns_what_the_input_can_t_lend() {
    let greeting = deserialize::<Greeting>(r#"{ "text": "\u00a1Hola!" }"#).unwrap();

    let Cow::Owned(text) = greeting.text else {
        panic!("expected an owned text, got {:?}", greeting.text);
    };
    assert_eq!(text, "¡Hola!");
}

#[test]
fn a_cow_with_borrow_is_checked_by_its_field_s_rules() {
    assert_eq!(
        issues_reading::<Greeting>(r#"{ "text": "Buenos días a todos" }"#),
        ["text: length(max: 10, unit: chars)"]
    );
}

#[derive(Deserialize, Debug)]
struct Attachment<'a> {
    #[serde(borrow)]
    content: Cow<'a, [u8]>,
}

#[test]
fn a_cow_of_bytes_with_borrow_borrows_from_the_input() {
    let attachment = deserialize::<Attachment>(r#"{ "content": "PDF" }"#).unwrap();

    let Cow::Borrowed(content) = attachment.content else {
        panic!("expected borrowed content, got {:?}", attachment.content);
    };
    assert_eq!(content, b"PDF");
}

#[derive(Deserialize, Debug)]
struct Labels<'a, 'b> {
    #[serde(borrow = "'a")]
    title: Cow<'a, str>,
    footer: Cow<'b, str>,
}

#[test]
fn borrow_with_named_lifetimes_borrows_only_through_them() {
    let labels = deserialize::<Labels>(r#"{ "title": "Notes", "footer": "Page 1" }"#).unwrap();

    let (Cow::Borrowed(title), Cow::Owned(footer)) = (&labels.title, &labels.footer) else {
        panic!("expected a borrowed title and an owned footer, got {labels:?}");
    };
    assert_eq!((*title, footer.as_str()), ("Notes", "Page 1"));
}

#[derive(Deserialize, Validate, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
struct Author<'a> {
    #[validate(required)]
    name: &'a str,
    #[validate(length(max = 5))]
    initials: &'a str,
}

#[derive(Deserialize, Debug, PartialEq)]
struct Post<'a> {
    #[validate(required)]
    title: &'a str,
    #[serde(borrow)]
    #[validate(nested)]
    author: Author<'a>,
}

#[test]
fn a_nested_struct_with_borrow_borrows_from_the_input() {
    let body = r#"{ "title": "Notes", "author": { "name": "Ada Lovelace", "initials": "AL" } }"#;

    let post = deserialize::<Post>(body).unwrap();

    assert_eq!(
        post,
        Post {
            title: "Notes",
            author: Author {
                name: "Ada Lovelace",
                initials: "AL",
            },
        }
    );
}

#[test]
fn a_nested_struct_with_borrow_reports_the_issues_inside_it_at_their_paths() {
    let body = r#"{ "title": "", "author": { "initials": "ADALOV" } }"#;

    assert_eq!(
        issues_reading::<Post>(body),
        [
            "title: required",
            "author.name: missing",
            "author.initials: length(max: 5, unit: chars)"
        ]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Page<'a, T> {
    #[validate(required)]
    cursor: &'a str,
    #[validate(length(max = 2))]
    items: Vec<T>,
}

#[test]
fn a_struct_with_a_lifetime_and_a_type_parameter_is_read_with_both() {
    let page = deserialize::<Page<u8>>(r#"{ "cursor": "abc", "items": [1, 2] }"#).unwrap();

    assert_eq!(
        page,
        Page {
            cursor: "abc",
            items: vec![1, 2],
        }
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Country {
    code: &'static str,
}

#[test]
fn a_static_str_borrows_from_static_input() {
    let country = deserialize::<Country>(r#"{ "code": "ES" }"#).unwrap();

    assert_eq!(country, Country { code: "ES" });
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct Comment<'a> {
    #[validate(required)]
    body: &'a str,
    #[serde(borrow)]
    #[validate(length(max = 3))]
    tag: Cow<'a, str>,
    #[serde(borrow)]
    #[validate(nested)]
    author: Author<'a>,
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct AuthorThenValidated<'a> {
    #[validate(required)]
    name: &'a str,
    #[validate(length(max = 5))]
    initials: &'a str,
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct CommentThenValidated<'a> {
    #[validate(required)]
    body: &'a str,
    #[serde(borrow)]
    #[validate(length(max = 3))]
    tag: Cow<'a, str>,
    #[serde(borrow)]
    #[validate(nested)]
    author: AuthorThenValidated<'a>,
}

#[test]
fn a_complete_borrowed_body_is_reported_as_serde_then_validate_report_it() {
    let body = r#"{ "body": "", "tag": "question", "author": { "name": "", "initials": "ADALOV" } }"#;

    assert_eq!(
        issues_found_reading::<Comment>(body),
        issues_validating_after_serde::<CommentThenValidated>(body)
    );
}
