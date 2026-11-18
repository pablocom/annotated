//! `#[derive(Deserialize)]` checks the elements of a `HashSet` or a `BTreeSet` with `each(...)` as it reads them: each
//! at its index in the input, in the order of the input, before it builds the set. A set it doesn't read, a default or
//! what a function returned, is checked in the set's own order, as `validate()` checks it.

#![cfg(feature = "json")]

use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet};
use std::hash::{Hash, Hasher};

use annotated::rules::HasElements;
use annotated::{Deserialize, DeserializeError, Issue, Problem, Validate};
use serde::Deserializer;

fn deserialize<'de, T: Deserialize<'de>>(json: &'de str) -> Result<T, DeserializeError<serde_json::Error>> {
    annotated::json::from_str(json)
}

fn issues_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<String> {
    paths_and_codes(deserialize::<T>(json))
}

fn paths_and_codes<T>(outcome: Result<T, DeserializeError<serde_json::Error>>) -> Vec<String> {
    match outcome {
        Ok(_) => Vec::new(),
        Err(DeserializeError::Issues(issues)) => issues.iter().map(path_and_code).collect(),
        Err(error) => panic!("expected issues, got the format's error: {error}"),
    }
}

fn path_and_code(issue: &Issue) -> String {
    let code = issue.problem().code();
    if issue.path().is_root() {
        return code.to_owned();
    }
    format!("{}: {code}", issue.path())
}

fn messages_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<String> {
    match deserialize::<T>(json) {
        Ok(_) => Vec::new(),
        Err(DeserializeError::Issues(issues)) => issues.iter().map(ToString::to_string).collect(),
        Err(error) => panic!("expected issues, got the format's error: {error}"),
    }
}

#[derive(Deserialize, Debug, PartialEq)]
struct Article {
    #[validate(each(length(max = 4)))]
    tags: BTreeSet<String>,
    #[validate(each(length(max = 4)))]
    topics: HashSet<String>,
}

#[test]
fn an_element_of_a_b_tree_set_is_reported_at_its_index_in_the_input() {
    let body = r#"{ "tags": ["rust", "serialization", "api"], "topics": [] }"#;

    assert_eq!(issues_reading::<Article>(body), ["tags[1]: length"]);
}

#[test]
fn an_element_of_a_hash_set_is_reported_at_its_index_in_the_input() {
    let body = r#"{ "tags": [], "topics": ["rust", "serialization", "api"] }"#;

    assert_eq!(issues_reading::<Article>(body), ["topics[1]: length"]);
}

#[test]
fn the_elements_of_a_set_are_reported_in_the_order_of_the_input() {
    let body = r#"{ "tags": ["serialization", "api", "deserialization"], "topics": [] }"#;

    assert_eq!(issues_reading::<Article>(body), ["tags[0]: length", "tags[2]: length"]);
}

#[test]
fn two_equal_elements_in_the_input_are_each_reported_at_their_index() {
    let body = r#"{ "tags": ["serialization", "api", "serialization"], "topics": [] }"#;

    assert_eq!(issues_reading::<Article>(body), ["tags[0]: length", "tags[2]: length"]);
}

#[test]
fn a_body_breaking_no_rule_builds_the_sets_of_its_elements() {
    let body = r#"{ "tags": ["rust", "api", "rust"], "topics": ["web"] }"#;

    assert_eq!(
        deserialize::<Article>(body).unwrap(),
        Article {
            tags: BTreeSet::from(["api".to_owned(), "rust".to_owned()]),
            topics: HashSet::from(["web".to_owned()]),
        }
    );
}

#[test]
fn a_struct_written_as_a_sequence_reports_an_element_of_a_set_at_its_index_in_the_input() {
    let body = r#"[["serialization", "api"], []]"#;

    assert_eq!(issues_reading::<Article>(body), ["tags[0]: length"]);
}

#[test]
fn failing_fast_answers_the_first_element_of_the_input_breaking_a_rule() {
    let body = r#"{ "tags": ["serialization", "api", "deserialization"], "topics": [] }"#;

    let outcome = annotated::json::from_str_fail_fast::<Article>(body);

    assert_eq!(paths_and_codes(outcome), ["tags[0]: length"]);
}

#[test]
fn a_limited_report_keeps_the_first_elements_of_the_input_breaking_a_rule() {
    let body = r#"{ "tags": ["serialization", "api", "deserialization", "web-services"], "topics": [] }"#;

    let outcome = annotated::json::from_str_limited_to::<Article>(body, 2);

    assert_eq!(paths_and_codes(outcome), ["tags[0]: length", "tags[2]: length"]);
}

// Equal, ordered and hashed whatever its case, so that a set keeps one of two spellings.
#[derive(serde::Deserialize, Debug)]
struct Term(String);

impl PartialEq for Term {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq_ignore_ascii_case(&other.0)
    }
}

impl Eq for Term {}

impl PartialOrd for Term {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Term {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.to_ascii_lowercase().cmp(&other.0.to_ascii_lowercase())
    }
}

impl Hash for Term {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_ascii_lowercase().hash(state);
    }
}

fn is_a_word(term: &Term) -> Result<(), Problem> {
    if term.0.contains(' ') {
        return Err(Problem::custom("not_a_word", "must be a single word"));
    }
    Ok(())
}

fn spellings<'a>(terms: impl IntoIterator<Item = &'a Term>) -> Vec<&'a str> {
    terms.into_iter().map(|term| term.0.as_str()).collect()
}

#[derive(Deserialize, Debug)]
struct Glossary {
    #[validate(each(custom(is_a_word)))]
    terms: BTreeSet<Term>,
    #[validate(each(custom(is_a_word)))]
    synonyms: HashSet<Term>,
}

#[test]
fn of_two_equal_elements_a_b_tree_set_keeps_the_first_as_serde_does() {
    let body = r#"{ "terms": ["Rust", "API", "rust"], "synonyms": [] }"#;

    let glossary = deserialize::<Glossary>(body).unwrap();

    assert_eq!(spellings(&glossary.terms), ["API", "Rust"]);
}

#[test]
fn of_two_equal_elements_a_hash_set_keeps_the_first_as_serde_does() {
    let body = r#"{ "terms": [], "synonyms": ["Serde", "serde"] }"#;

    let glossary = deserialize::<Glossary>(body).unwrap();

    assert_eq!(spellings(&glossary.synonyms), ["Serde"]);
}

#[derive(Deserialize, Debug)]
struct Shortlist {
    #[validate(length(max = 2), each(length(max = 4)))]
    #[allow(dead_code)]
    tags: BTreeSet<String>,
}

#[test]
fn length_on_a_set_counts_its_distinct_elements() {
    let body = r#"{ "tags": ["api", "web", "api"] }"#;

    assert_eq!(issues_reading::<Shortlist>(body), Vec::<String>::new());
}

#[test]
fn a_rule_before_each_that_the_set_breaks_skips_its_elements() {
    let body = r#"{ "tags": ["serialization", "api", "web"] }"#;

    assert_eq!(issues_reading::<Shortlist>(body), ["tags: length"]);
}

#[derive(Deserialize, Debug)]
#[validate(report_all)]
struct EveryIssueOfAShortlist {
    #[validate(length(max = 2), each(length(max = 4)))]
    #[allow(dead_code)]
    tags: BTreeSet<String>,
}

#[test]
fn report_all_reports_a_rule_before_each_then_the_elements_in_the_order_of_the_input() {
    let body = r#"{ "tags": ["serialization", "api", "web"] }"#;

    assert_eq!(
        issues_reading::<EveryIssueOfAShortlist>(body),
        ["tags: length", "tags[0]: length"]
    );
}

#[derive(Deserialize, Debug, PartialEq)]
struct Draft {
    #[validate(each(length(max = 4)))]
    tags: Option<BTreeSet<String>>,
}

#[test]
fn an_absent_optional_set_is_none() {
    assert_eq!(deserialize::<Draft>("{}").unwrap(), Draft { tags: None });
}

#[test]
fn a_null_optional_set_is_none() {
    assert_eq!(
        deserialize::<Draft>(r#"{ "tags": null }"#).unwrap(),
        Draft { tags: None }
    );
}

#[test]
fn an_element_of_an_optional_set_is_reported_at_its_index_in_the_input() {
    let body = r#"{ "tags": ["serialization", "api"] }"#;

    assert_eq!(issues_reading::<Draft>(body), ["tags[0]: length"]);
}

#[derive(Deserialize, Validate, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Author {
    name: String,
    #[validate(range(max = 120))]
    age: u8,
}

fn not_anonymous(author: &Author) -> Result<(), Problem> {
    if author.name == "Anonymous" {
        return Err(Problem::custom("anonymous", "must have a name"));
    }
    Ok(())
}

#[derive(Deserialize, Debug)]
struct Book {
    #[validate(each(nested))]
    #[allow(dead_code)]
    authors: BTreeSet<Author>,
    #[validate(each(nested))]
    #[allow(dead_code)]
    editors: HashSet<Author>,
}

#[test]
fn the_issues_inside_an_element_of_a_b_tree_set_are_reported_at_its_index_in_the_input() {
    let body = r#"{ "authors": [{ "name": "Zoe", "age": 130 }, { "name": "Ann", "age": 40 }], "editors": [] }"#;

    assert_eq!(issues_reading::<Book>(body), ["authors[0].age: range"]);
}

#[test]
fn the_issues_inside_an_element_of_a_hash_set_are_reported_at_its_index_in_the_input() {
    let body = r#"{ "authors": [], "editors": [{ "name": "Zoe", "age": 40 }, { "name": "Ann", "age": 130 }] }"#;

    assert_eq!(issues_reading::<Book>(body), ["editors[1].age: range"]);
}

#[derive(Deserialize, Debug)]
struct Anthology {
    #[validate(each(nested), length(max = 1))]
    #[allow(dead_code)]
    authors: BTreeSet<Author>,
}

#[test]
fn an_element_that_could_not_be_built_reports_its_issues_and_skips_the_other_rules_of_its_set() {
    let body = r#"{ "authors": [{ "name": "Zoe" }, { "age": 40 }] }"#;

    assert_eq!(
        issues_reading::<Anthology>(body),
        ["authors[0].age: missing", "authors[1].name: missing"]
    );
}

#[derive(Deserialize, Debug)]
struct Credits {
    #[validate(each(nested, custom(not_anonymous)))]
    #[allow(dead_code)]
    authors: BTreeSet<Author>,
    #[validate(each(custom(not_anonymous), nested))]
    #[allow(dead_code)]
    editors: BTreeSet<Author>,
}

#[test]
fn nested_before_another_rule_in_each_stops_an_element_s_rules_at_its_issues() {
    let body = r#"{
        "authors": [{ "name": "Zoe", "age": 130 }, { "name": "Anonymous", "age": 130 }],
        "editors": []
    }"#;

    assert_eq!(
        issues_reading::<Credits>(body),
        ["authors[0].age: range", "authors[1].age: range"]
    );
}

#[test]
fn a_rule_before_nested_in_each_that_an_element_breaks_skips_the_issues_inside_it() {
    let body = r#"{
        "authors": [],
        "editors": [{ "name": "Zoe", "age": 130 }, { "name": "Anonymous", "age": 130 }]
    }"#;

    assert_eq!(
        issues_reading::<Credits>(body),
        ["editors[0].age: range", "editors[1]: anonymous"]
    );
}

#[derive(Deserialize, Debug)]
struct Foreword {
    #[validate(each(nested))]
    #[allow(dead_code)]
    authors: Option<BTreeSet<Author>>,
}

#[test]
fn the_issues_inside_an_element_of_an_optional_set_are_reported_at_its_index_in_the_input() {
    let body = r#"{ "authors": [{ "name": "Zoe", "age": 130 }, { "name": "Ann", "age": 40 }] }"#;

    assert_eq!(issues_reading::<Foreword>(body), ["authors[0].age: range"]);
}

#[derive(Deserialize, Debug)]
#[annotated(collect_type_errors)]
struct CollectedArticle {
    #[validate(each(length(max = 4)))]
    #[allow(dead_code)]
    tags: BTreeSet<String>,
    #[validate(each(nested))]
    #[allow(dead_code)]
    authors: BTreeSet<Author>,
}

#[test]
fn collecting_type_errors_an_element_of_the_wrong_type_is_reported_at_its_set_as_for_a_list() {
    let body = r#"{ "tags": ["api", 3], "authors": [] }"#;

    assert_eq!(
        messages_reading::<CollectedArticle>(body),
        ["tags: must be a string, found integer"]
    );
}

#[test]
fn collecting_type_errors_a_nested_element_of_the_wrong_type_replaces_the_issues_of_its_set_as_for_a_list() {
    let body = r#"{ "tags": [], "authors": [{ "name": "Zoe", "age": 130 }, 5] }"#;

    assert_eq!(
        messages_reading::<CollectedArticle>(body),
        ["authors: must be struct Author, found integer"]
    );
}

fn sections() -> BTreeSet<String> {
    BTreeSet::from(["serialization".to_owned(), "api".to_owned()])
}

#[derive(Deserialize, Debug)]
struct Magazine {
    #[serde(default = "sections")]
    #[validate(each(length(max = 4)))]
    #[allow(dead_code)]
    sections: BTreeSet<String>,
}

#[test]
fn the_default_of_an_absent_set_is_checked_in_the_set_s_order() {
    assert_eq!(issues_reading::<Magazine>("{}"), ["sections[1]: length"]);
}

#[test]
fn a_set_given_beside_its_default_is_checked_in_the_order_of_the_input() {
    let body = r#"{ "sections": ["serialization", "api"] }"#;

    assert_eq!(issues_reading::<Magazine>(body), ["sections[0]: length"]);
}

#[derive(Deserialize, Debug)]
#[serde(default)]
struct Journal {
    #[validate(each(length(max = 4)))]
    #[allow(dead_code)]
    sections: BTreeSet<String>,
}

impl Default for Journal {
    fn default() -> Self {
        Self { sections: sections() }
    }
}

#[test]
fn a_set_taken_from_the_struct_s_default_is_checked_in_the_set_s_order() {
    assert_eq!(issues_reading::<Journal>("{}"), ["sections[1]: length"]);
}

fn an_anonymous_author_and_a_late_one() -> BTreeSet<Author> {
    BTreeSet::from([
        Author {
            name: "Zoe".to_owned(),
            age: 130,
        },
        Author {
            name: "Anonymous".to_owned(),
            age: 30,
        },
    ])
}

#[derive(Deserialize, Debug)]
struct Preface {
    #[serde(default = "an_anonymous_author_and_a_late_one")]
    #[validate(each(custom(not_anonymous), nested))]
    #[allow(dead_code)]
    authors: BTreeSet<Author>,
}

#[test]
fn the_elements_of_a_default_are_checked_with_nested_beside_another_rule_in_the_set_s_order() {
    assert_eq!(
        issues_reading::<Preface>("{}"),
        ["authors[0]: anonymous", "authors[1].age: range"]
    );
}

fn sections_from_text<'de, D: Deserializer<'de>>(deserializer: D) -> Result<BTreeSet<String>, D::Error> {
    let text: String = serde::Deserialize::deserialize(deserializer)?;
    Ok(text.split(',').map(str::to_owned).collect())
}

#[derive(Deserialize, Debug)]
struct Index {
    #[serde(deserialize_with = "sections_from_text")]
    #[validate(each(length(max = 4)))]
    #[allow(dead_code)]
    sections: BTreeSet<String>,
}

#[test]
fn a_set_read_by_a_function_is_checked_in_the_set_s_order() {
    let body = r#"{ "sections": "serialization,api" }"#;

    assert_eq!(issues_reading::<Index>(body), ["sections[1]: length"]);
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum Publication {
    Article {
        #[validate(each(length(max = 4)))]
        #[allow(dead_code)]
        tags: BTreeSet<String>,
    },
    Tags(
        #[validate(each(length(max = 4)))]
        #[allow(dead_code)]
        BTreeSet<String>,
    ),
}

#[test]
fn an_element_of_a_set_in_a_struct_variant_is_reported_at_its_index_in_the_input() {
    let body = r#"{ "article": { "tags": ["serialization", "api"] } }"#;

    assert_eq!(issues_reading::<Publication>(body), ["article.tags[0]: length"]);
}

#[test]
fn an_element_of_a_set_in_a_newtype_variant_is_reported_at_its_index_in_the_input() {
    let body = r#"{ "tags": ["serialization", "api"] }"#;

    assert_eq!(issues_reading::<Publication>(body), ["tags[0]: length"]);
}

#[derive(Deserialize, Debug)]
struct Shelf<C: HasElements<Element = String>> {
    #[validate(each(length(max = 4)))]
    #[allow(dead_code)]
    items: C,
}

#[test]
fn a_set_a_type_parameter_stands_for_is_checked_in_the_set_s_order() {
    let body = r#"{ "items": ["serialization", "api"] }"#;

    assert_eq!(issues_reading::<Shelf<BTreeSet<String>>>(body), ["items[1]: length"]);
}

#[derive(Deserialize, Debug, PartialEq)]
struct Labels<'a> {
    #[serde(borrow)]
    #[validate(each(length(max = 4)))]
    tags: BTreeSet<&'a str>,
}

#[test]
fn a_set_borrowing_its_elements_from_the_input_is_built_from_them() {
    let body = r#"{ "tags": ["rust", "api"] }"#;

    assert_eq!(
        deserialize::<Labels>(body).unwrap(),
        Labels {
            tags: BTreeSet::from(["api", "rust"])
        }
    );
}
