//! Scenarios 1 to 3: a flat sign-up body of eight fields, valid, with every field invalid, and with one field
//! invalid.
//!
//! The rules, the same in every library:
//!
//! | Field | Type | Rules |
//! |---|---|---|
//! | `username` | `String` | length 3 to 30 chars, pattern `^[a-z0-9_]+$` |
//! | `fullName` | `Option<String>` | required (is `Some`), length at most 50 chars |
//! | `email` | `String` | email |
//! | `website` | `Option<String>` | url, when present |
//! | `age` | `u8` | range 18 to 120 |
//! | `rating` | `f64` | range 0.0 to 5.0 |
//! | `postalCode` | `String` | pattern `^\d{5}$` |
//! | `bio` | `Option<String>` | length at most 280 chars, when present |

use serde::de::DeserializeOwned;

use super::count;
use crate::{Case, Library};

pub const VALID_BODY: &str = r#"{
    "username": "alex_morgan",
    "fullName": "Alex Morgan",
    "email": "alex.morgan@example.com",
    "website": "https://example.com/alex",
    "age": 34,
    "rating": 4.5,
    "postalCode": "28013",
    "bio": "Backend developer in Madrid. Likes long walks and short functions."
}"#;

/// The valid body with an email address missing its `@`, the most common failure: one field wrong.
pub const ONE_FIELD_INVALID_BODY: &str = r#"{
    "username": "alex_morgan",
    "fullName": "Alex Morgan",
    "email": "alex.morgan.example.com",
    "website": "https://example.com/alex",
    "age": 34,
    "rating": 4.5,
    "postalCode": "28013",
    "bio": "Backend developer in Madrid. Likes long walks and short functions."
}"#;

/// Every field breaks a rule, and `username` breaks two: too short and not matching its pattern.
#[must_use]
pub fn every_field_invalid_body() -> String {
    let too_long_bio = "a".repeat(281);
    format!(
        r#"{{
    "username": "A!",
    "fullName": null,
    "email": "alex.morgan.example.com",
    "website": "example dot com",
    "age": 12,
    "rating": 7.5,
    "postalCode": "2801",
    "bio": "{too_long_bio}"
}}"#
    )
}

pub mod with_annotated {
    use serde::Deserialize;

    #[derive(Deserialize, annotated::Validate)]
    #[serde(rename_all = "camelCase")]
    pub struct SignUp {
        #[validate(length(min = 3, max = 30), pattern = r"^[a-z0-9_]+$")]
        pub username: String,
        #[validate(required, length(max = 50))]
        pub full_name: Option<String>,
        #[validate(email)]
        pub email: String,
        #[validate(url)]
        pub website: Option<String>,
        #[validate(range(min = 18, max = 120))]
        pub age: u8,
        #[validate(range(min = 0.0, max = 5.0))]
        pub rating: f64,
        #[validate(pattern = r"^\d{5}$")]
        pub postal_code: String,
        #[validate(length(max = 280))]
        pub bio: Option<String>,
    }

    #[derive(Deserialize, annotated::Validate)]
    #[serde(rename_all = "camelCase")]
    #[validate(report_all)]
    pub struct SignUpReportingAll {
        #[validate(length(min = 3, max = 30), pattern = r"^[a-z0-9_]+$")]
        pub username: String,
        #[validate(required, length(max = 50))]
        pub full_name: Option<String>,
        #[validate(email)]
        pub email: String,
        #[validate(url)]
        pub website: Option<String>,
        #[validate(range(min = 18, max = 120))]
        pub age: u8,
        #[validate(range(min = 0.0, max = 5.0))]
        pub rating: f64,
        #[validate(pattern = r"^\d{5}$")]
        pub postal_code: String,
        #[validate(length(max = 280))]
        pub bio: Option<String>,
    }
}

/// The same rules read with annotated's collecting deserialization: `annotated::Deserialize` in place of serde's
/// derive, so the body is checked as it is read and a missing field is one more issue. Its serde `Deserialize`
/// validates too, so these types are only ever read with `annotated::json`.
pub mod with_annotated_deserialize {
    #[derive(annotated::Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SignUp {
        #[validate(length(min = 3, max = 30), pattern = r"^[a-z0-9_]+$")]
        pub username: String,
        #[validate(required, length(max = 50))]
        pub full_name: Option<String>,
        #[validate(email)]
        pub email: String,
        #[validate(url)]
        pub website: Option<String>,
        #[validate(range(min = 18, max = 120))]
        pub age: u8,
        #[validate(range(min = 0.0, max = 5.0))]
        pub rating: f64,
        #[validate(pattern = r"^\d{5}$")]
        pub postal_code: String,
        #[validate(length(max = 280))]
        pub bio: Option<String>,
    }

    #[derive(annotated::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[validate(report_all)]
    pub struct SignUpReportingAll {
        #[validate(length(min = 3, max = 30), pattern = r"^[a-z0-9_]+$")]
        pub username: String,
        #[validate(required, length(max = 50))]
        pub full_name: Option<String>,
        #[validate(email)]
        pub email: String,
        #[validate(url)]
        pub website: Option<String>,
        #[validate(range(min = 18, max = 120))]
        pub age: u8,
        #[validate(range(min = 0.0, max = 5.0))]
        pub rating: f64,
        #[validate(pattern = r"^\d{5}$")]
        pub postal_code: String,
        #[validate(length(max = 280))]
        pub bio: Option<String>,
    }

    /// Reads each field into a buffer before its type, so a value of the wrong type is one more issue too.
    #[derive(annotated::Deserialize)]
    #[serde(rename_all = "camelCase")]
    #[annotated(collect_type_errors)]
    pub struct SignUpCollectingTypeErrors {
        #[validate(length(min = 3, max = 30), pattern = r"^[a-z0-9_]+$")]
        pub username: String,
        #[validate(required, length(max = 50))]
        pub full_name: Option<String>,
        #[validate(email)]
        pub email: String,
        #[validate(url)]
        pub website: Option<String>,
        #[validate(range(min = 18, max = 120))]
        pub age: u8,
        #[validate(range(min = 0.0, max = 5.0))]
        pub rating: f64,
        #[validate(pattern = r"^\d{5}$")]
        pub postal_code: String,
        #[validate(length(max = 280))]
        pub bio: Option<String>,
    }
}

pub mod with_validator {
    use std::sync::LazyLock;

    use regex::Regex;
    use serde::Deserialize;

    static USERNAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-z0-9_]+$").unwrap());
    static POSTAL_CODE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\d{5}$").unwrap());

    #[derive(Deserialize, validator::Validate)]
    #[serde(rename_all = "camelCase")]
    pub struct SignUp {
        #[validate(length(min = 3, max = 30), regex(path = *USERNAME))]
        pub username: String,
        #[validate(required, length(max = 50))]
        pub full_name: Option<String>,
        #[validate(email)]
        pub email: String,
        #[validate(url)]
        pub website: Option<String>,
        #[validate(range(min = 18, max = 120))]
        pub age: u8,
        #[validate(range(min = 0.0, max = 5.0))]
        pub rating: f64,
        #[validate(regex(path = *POSTAL_CODE))]
        pub postal_code: String,
        #[validate(length(max = 280))]
        pub bio: Option<String>,
    }
}

pub mod with_garde {
    use serde::Deserialize;

    #[derive(Deserialize, garde::Validate)]
    #[serde(rename_all = "camelCase")]
    pub struct SignUp {
        #[garde(length(chars, min = 3, max = 30), pattern(r"^[a-z0-9_]+$"))]
        pub username: String,
        #[garde(required, length(chars, max = 50))]
        pub full_name: Option<String>,
        #[garde(email)]
        pub email: String,
        #[garde(url)]
        pub website: Option<String>,
        #[garde(range(min = 18, max = 120))]
        pub age: u8,
        #[garde(range(min = 0.0, max = 5.0))]
        pub rating: f64,
        #[garde(pattern(r"^\d{5}$"))]
        pub postal_code: String,
        #[garde(length(chars, max = 280))]
        pub bio: Option<String>,
    }
}

pub mod with_wary {
    use serde::Deserialize;

    #[derive(Deserialize, wary::Wary)]
    #[serde(rename_all = "camelCase")]
    pub struct SignUp {
        #[validate(length(chars, 3..=30), regex(pat = r"^[a-z0-9_]+$"))]
        pub username: String,
        #[validate(required, length(chars, ..=50))]
        pub full_name: Option<String>,
        #[validate(email)]
        pub email: String,
        #[validate(url)]
        pub website: Option<String>,
        #[validate(range(18..=120))]
        pub age: u8,
        #[validate(range(0.0..=5.0))]
        pub rating: f64,
        #[validate(regex(pat = r"^\d{5}$"))]
        pub postal_code: String,
        #[validate(length(chars, ..=280))]
        pub bio: Option<String>,
    }
}

pub(crate) fn parsed<T: DeserializeOwned>(body: &str) -> T {
    serde_json::from_str(body).expect("the scenario's body deserializes")
}

#[must_use]
pub fn cases() -> Vec<Case> {
    let every_field_invalid_body = every_field_invalid_body();
    [
        ("flat_valid", VALID_BODY),
        ("flat_every_field_invalid", every_field_invalid_body.as_str()),
        ("flat_one_field_invalid", ONE_FIELD_INVALID_BODY),
    ]
    .into_iter()
    .flat_map(|(scenario, body)| cases_of(scenario, body))
    .collect()
}

fn cases_of(scenario: &'static str, body: &str) -> Vec<Case> {
    use wary::Validate as _;

    vec![
        Case::new(
            scenario,
            Library::Annotated,
            parsed::<with_annotated::SignUp>(body),
            annotated::Validate::validate,
            count::annotated,
        ),
        Case::new(
            scenario,
            Library::Annotated,
            parsed::<with_annotated::SignUpReportingAll>(body),
            annotated::Validate::validate,
            count::annotated,
        )
        .variant("report_all"),
        Case::new(
            scenario,
            Library::Validator,
            parsed::<with_validator::SignUp>(body),
            validator::Validate::validate,
            count::validator,
        ),
        Case::new(
            scenario,
            Library::Garde,
            parsed::<with_garde::SignUp>(body),
            garde::Validate::validate,
            count::garde,
        ),
        Case::new(
            scenario,
            Library::Wary,
            parsed::<with_wary::SignUp>(body),
            |sign_up| sign_up.validate(&()),
            count::wary,
        ),
    ]
}
