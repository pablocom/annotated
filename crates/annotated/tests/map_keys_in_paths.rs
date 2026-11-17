//! A map's key in a path is at most 256 chars: a longer key is cut to its first 256 chars followed by `…`.

use std::collections::BTreeMap;
use std::fmt::{self, Display};

use annotated::{Issue, Location, Path, PathSegment, Validate};

#[derive(Validate)]
struct Catalog {
    #[validate(each(length(min = 1)))]
    titles: BTreeMap<String, String>,
}

fn catalog_with_a_blank_title_at(key: &str) -> Catalog {
    Catalog {
        titles: BTreeMap::from([(key.to_owned(), String::new())]),
    }
}

fn the_one_issue_of(value: &impl Validate) -> Issue {
    let issues = value.validate().unwrap_err();
    let [issue] = issues.iter().collect::<Vec<_>>()[..] else {
        panic!("expected one issue, got {issues:?}");
    };
    issue.clone()
}

#[test]
fn a_key_longer_than_256_chars_is_cut_to_its_first_256_chars_followed_by_an_ellipsis() {
    let catalog = catalog_with_a_blank_title_at(&format!("{}and-the-rest-is-cut", "k".repeat(256)));

    let issue = the_one_issue_of(&catalog);

    assert_eq!(
        issue.path().segments(),
        [
            PathSegment::Field("titles"),
            PathSegment::Key(format!("{}…", "k".repeat(256)))
        ]
    );
}

#[test]
fn the_json_pointer_of_a_path_under_a_long_key_holds_the_cut_key() {
    let catalog = catalog_with_a_blank_title_at(&"k".repeat(300));

    let issue = the_one_issue_of(&catalog);

    assert_eq!(issue.path().json_pointer(), format!("/titles/{}…", "k".repeat(256)));
}

#[test]
fn a_key_of_exactly_256_chars_is_kept_whole() {
    let catalog = catalog_with_a_blank_title_at(&"k".repeat(256));

    let issue = the_one_issue_of(&catalog);

    assert_eq!(
        issue.path().segments(),
        [PathSegment::Field("titles"), PathSegment::Key("k".repeat(256))]
    );
}

#[test]
fn a_key_of_multibyte_chars_is_cut_after_256_chars() {
    let catalog = catalog_with_a_blank_title_at(&"ü".repeat(300));

    let issue = the_one_issue_of(&catalog);

    assert_eq!(
        issue.path().segments(),
        [
            PathSegment::Field("titles"),
            PathSegment::Key(format!("{}…", "ü".repeat(256)))
        ]
    );
}

#[test]
fn a_path_built_with_a_long_key_holds_it_cut_as_an_issue_does() {
    let path = Path::root().key("k".repeat(300));

    assert_eq!(path.segments(), [PathSegment::Key(format!("{}…", "k".repeat(256)))]);
}

struct EndlessKey;

impl Display for EndlessKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        loop {
            formatter.write_str("k")?;
        }
    }
}

#[test]
fn a_key_stops_being_rendered_once_it_is_known_to_be_cut() {
    let root = Location::root();

    let path = root.key(&EndlessKey).to_path();

    assert_eq!(path.segments(), [PathSegment::Key(format!("{}…", "k".repeat(256)))]);
}

#[cfg(feature = "serde")]
mod deserializing {
    use std::collections::BTreeMap;

    use annotated::{Deserialize, Path, Problem};

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    #[allow(dead_code)]
    struct Order {
        #[validate(each(custom(is_still_sold), nested))]
        lines: BTreeMap<String, Line>,
    }

    #[derive(Deserialize, Debug)]
    #[serde(rename_all = "camelCase")]
    #[allow(dead_code)]
    struct Line {
        #[validate(range(min = 1))]
        quantity: u32,
        is_discontinued: bool,
    }

    fn is_still_sold(line: &Line) -> Result<(), Problem> {
        if line.is_discontinued {
            return Err(Problem::custom("discontinued", "is no longer sold"));
        }
        Ok(())
    }

    #[test]
    fn a_value_under_a_long_key_breaking_a_rule_before_nested_has_the_issues_inside_it_dropped() {
        let body = format!(
            r#"{{ "lines": {{ "{}": {{ "quantity": 0, "isDiscontinued": true }} }} }}"#,
            "k".repeat(300)
        );
        let mut deserializer = serde_json::Deserializer::from_str(&body);

        let error = annotated::from_deserializer::<Order, _>(&mut deserializer).unwrap_err();

        let paths_and_codes: Vec<(&Path, &str)> = error
            .issues()
            .iter()
            .map(|issue| (issue.path(), issue.problem().code()))
            .collect();
        assert_eq!(
            paths_and_codes,
            [(
                &Path::root().field("lines").key(format!("{}…", "k".repeat(256))),
                "discontinued"
            )]
        );
    }
}
