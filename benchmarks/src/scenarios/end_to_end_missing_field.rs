//! Scenario 8: a request body missing a field, from JSON text: the sign-up body of scenario 1 without its
//! `postalCode`, its email and age broken.
//!
//! **The cases do different amounts of work, so their timings aren't like for like.** annotated's collecting
//! deserialization, `annotated::json::from_str`, reads the whole body, checks every field that came, and reports
//! the missing one beside the broken ones: three violations in one answer. serde's derive stops at the missing
//! field once it has read the body, so with `serde_json::from_str` no library gets to validate: every one,
//! annotated's `Validate` included, answers serde's error alone, and the client learns about the email and the age
//! only on its next requests. serde's error counts as one problem.

use super::count;
use super::end_to_end::{PlainSignUp, Received, received};
use super::flat::{with_annotated, with_annotated_deserialize, with_garde, with_validator, with_wary};
use crate::{Case, Library};

const SCENARIO: &str = "end_to_end_missing_field";

/// The valid sign-up body without `postalCode`, with an email address missing its `@` and an age of 12.
pub const MISSING_FIELD_BODY: &str = r#"{
    "username": "alex_morgan",
    "fullName": "Alex Morgan",
    "email": "alex.morgan.example.com",
    "website": "https://example.com/alex",
    "age": 12,
    "rating": 4.5,
    "bio": "Backend developer in Madrid. Likes long walks and short functions."
}"#;

/// The problems a client learns about from one answer: the violations validation found, or one, serde's error,
/// when serde refused the body before validation.
fn problems_received<Violations>(
    received: &Received<Violations>,
    count: impl Fn(&Result<(), Violations>) -> usize,
) -> usize {
    received.as_ref().map_or(1, count)
}

#[must_use]
pub fn cases() -> Vec<Case> {
    use annotated::Validate as _;
    use garde::Validate as _;
    use validator::Validate as _;
    use wary::Validate as _;

    let body = MISSING_FIELD_BODY.to_owned();
    vec![
        Case::new(
            SCENARIO,
            Library::SerdeJsonAlone,
            body.clone(),
            |body| serde_json::from_str::<PlainSignUp>(body).map(drop),
            |received| usize::from(received.is_err()),
        ),
        Case::new(
            SCENARIO,
            Library::Annotated,
            body.clone(),
            |body| annotated::json::from_str::<with_annotated_deserialize::SignUp>(body),
            count::annotated_deserialized,
        )
        .variant("json::from_str"),
        Case::new(
            SCENARIO,
            Library::Annotated,
            body.clone(),
            |body| received(body, with_annotated::SignUp::validate),
            |received| problems_received(received, count::annotated),
        )
        .variant("serde_json, then validate"),
        Case::new(
            SCENARIO,
            Library::Validator,
            body.clone(),
            |body| received(body, with_validator::SignUp::validate),
            |received| problems_received(received, count::validator),
        ),
        Case::new(
            SCENARIO,
            Library::Garde,
            body.clone(),
            |body| received(body, |sign_up: &with_garde::SignUp| sign_up.validate()),
            |received| problems_received(received, count::garde),
        ),
        Case::new(
            SCENARIO,
            Library::Wary,
            body,
            |body| received(body, |sign_up: &with_wary::SignUp| sign_up.validate(&())),
            |received| problems_received(received, count::wary),
        ),
    ]
}
