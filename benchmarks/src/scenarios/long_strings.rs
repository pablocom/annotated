//! Scenario 5: `length` on a 1 KB and a 64 KB text, the longest length within bounds, so the whole text is
//! counted. The text is Spanish prose: mostly ASCII, with some two-byte letters, as real text is.
//!
//! The rule is length at most 100000 in every library, counted the way each can: chars (Unicode scalar
//! values), bytes, or grapheme clusters. `serde_valid` counts only graphemes; `validator` counts only chars.

use super::count;
use super::flat::parsed;
use crate::{Case, Library};

const PROSE: &str = "La niña pidió una paella en la playa de Valencia; ¡qué rica estaba, y qué calor hacía! ";

/// A text of exactly `bytes` bytes of Spanish prose.
#[must_use]
pub fn prose_of(bytes: usize) -> String {
    let mut text = PROSE.repeat(bytes / PROSE.len() + 1);
    let mut end = bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    let padding = "x".repeat(bytes - end);
    text.push_str(&padding);
    text
}

fn article_body(text: &str) -> String {
    format!(r#"{{"text":"{text}"}}"#)
}

pub mod with_annotated {
    use serde::Deserialize;

    #[derive(Deserialize, annotated::Validate)]
    pub struct Article {
        #[validate(length(max = 100_000))]
        pub text: String,
    }

    #[derive(Deserialize, annotated::Validate)]
    pub struct ArticleInBytes {
        #[validate(length(max = 100_000, unit = "bytes"))]
        pub text: String,
    }

    #[derive(Deserialize, annotated::Validate)]
    pub struct ArticleInGraphemes {
        #[validate(length(max = 100_000, unit = "graphemes"))]
        pub text: String,
    }
}

pub mod with_validator {
    use serde::Deserialize;

    #[derive(Deserialize, validator::Validate)]
    pub struct Article {
        #[validate(length(max = 100_000))]
        pub text: String,
    }
}

pub mod with_garde {
    use serde::Deserialize;

    #[derive(Deserialize, garde::Validate)]
    pub struct Article {
        #[garde(length(chars, max = 100_000))]
        pub text: String,
    }

    #[derive(Deserialize, garde::Validate)]
    pub struct ArticleInBytes {
        #[garde(length(bytes, max = 100_000))]
        pub text: String,
    }

    #[derive(Deserialize, garde::Validate)]
    pub struct ArticleInGraphemes {
        #[garde(length(graphemes, max = 100_000))]
        pub text: String,
    }
}

pub mod with_wary {
    use serde::Deserialize;

    #[derive(Deserialize, wary::Wary)]
    pub struct Article {
        #[validate(length(chars, ..=100_000))]
        pub text: String,
    }

    #[derive(Deserialize, wary::Wary)]
    pub struct ArticleInBytes {
        #[validate(length(bytes, ..=100_000))]
        pub text: String,
    }

    #[derive(Deserialize, wary::Wary)]
    pub struct ArticleInGraphemes {
        #[validate(length(graphemes, ..=100_000))]
        pub text: String,
    }
}

pub mod with_serde_valid {
    use serde::Deserialize;

    #[derive(Deserialize, serde_valid::Validate)]
    pub struct Article {
        #[validate(max_length = 100_000)]
        pub text: String,
    }
}

#[must_use]
pub fn cases() -> Vec<Case> {
    [("long_string_1kb", 1024), ("long_string_64kb", 64 * 1024)]
        .into_iter()
        .flat_map(|(scenario, bytes)| cases_of(scenario, bytes))
        .collect()
}

fn cases_of(scenario: &'static str, bytes: usize) -> Vec<Case> {
    use wary::Validate as _;

    let body = article_body(&prose_of(bytes));
    let body = body.as_str();
    vec![
        Case::new(
            scenario,
            Library::Annotated,
            parsed::<with_annotated::Article>(body),
            annotated::Validate::validate,
            count::annotated,
        )
        .variant("chars"),
        Case::new(
            scenario,
            Library::Annotated,
            parsed::<with_annotated::ArticleInBytes>(body),
            annotated::Validate::validate,
            count::annotated,
        )
        .variant("bytes"),
        Case::new(
            scenario,
            Library::Annotated,
            parsed::<with_annotated::ArticleInGraphemes>(body),
            annotated::Validate::validate,
            count::annotated,
        )
        .variant("graphemes"),
        Case::new(
            scenario,
            Library::Validator,
            parsed::<with_validator::Article>(body),
            validator::Validate::validate,
            count::validator,
        )
        .variant("chars"),
        Case::new(
            scenario,
            Library::Garde,
            parsed::<with_garde::Article>(body),
            garde::Validate::validate,
            count::garde,
        )
        .variant("chars"),
        Case::new(
            scenario,
            Library::Garde,
            parsed::<with_garde::ArticleInBytes>(body),
            garde::Validate::validate,
            count::garde,
        )
        .variant("bytes"),
        Case::new(
            scenario,
            Library::Garde,
            parsed::<with_garde::ArticleInGraphemes>(body),
            garde::Validate::validate,
            count::garde,
        )
        .variant("graphemes"),
        Case::new(
            scenario,
            Library::Wary,
            parsed::<with_wary::Article>(body),
            |article| article.validate(&()),
            count::wary,
        )
        .variant("chars"),
        Case::new(
            scenario,
            Library::Wary,
            parsed::<with_wary::ArticleInBytes>(body),
            |article| article.validate(&()),
            count::wary,
        )
        .variant("bytes"),
        Case::new(
            scenario,
            Library::Wary,
            parsed::<with_wary::ArticleInGraphemes>(body),
            |article| article.validate(&()),
            count::wary,
        )
        .variant("graphemes"),
        Case::new(
            scenario,
            Library::SerdeValid,
            parsed::<with_serde_valid::Article>(body),
            serde_valid::Validate::validate,
            count::serde_valid,
        )
        .variant("graphemes"),
    ]
    .into_iter()
    .map(|case| case.throughput_bytes(bytes))
    .collect()
}
