//! `each(...)` applies its rules to every element of a collection, or every value of a map.

mod support;

use std::collections::{BTreeMap, HashMap, VecDeque};

use annotated::Validate;
use support::violations_of;

#[derive(Validate)]
struct Author {
    #[validate(required)]
    name: String,
}

#[derive(Validate)]
struct Post {
    #[validate(each(required, length(min = 2, max = 10)))]
    tags: Vec<String>,
    #[validate(each(nested))]
    authors: Vec<Author>,
    #[validate(each(length(max = 8)))]
    titles: BTreeMap<String, String>,
    #[validate(each(required, length(min = 2)))]
    reviewers: Vec<Option<String>>,
    #[validate(each(length(min = 2)))]
    keywords: Option<Vec<String>>,
    #[validate(each(each(length(max = 3))))]
    grid: Vec<Vec<String>>,
    #[validate(length(max = 2), each(length(max = 3)))]
    codes: Vec<String>,
}

fn post() -> Post {
    Post {
        tags: vec!["rust".to_owned()],
        authors: vec![Author {
            name: "Alex".to_owned(),
        }],
        titles: BTreeMap::from([("en".to_owned(), "Settings".to_owned())]),
        reviewers: vec![Some("Sam".to_owned())],
        keywords: None,
        grid: vec![vec!["ab".to_owned()]],
        codes: vec!["a1".to_owned()],
    }
}

#[test]
fn a_value_whose_elements_follow_their_rules_has_no_violations() {
    assert_eq!(post().validate(), Ok(()));
}

#[test]
fn each_reports_an_element_at_its_index() {
    let post = Post {
        tags: vec!["rust".to_owned(), "validation-rules".to_owned()],
        ..post()
    };

    assert_eq!(violations_of(&post), ["tags[1]: length"]);
}

#[test]
fn each_checks_every_element() {
    let post = Post {
        tags: vec!["validation-rules".to_owned(), String::new()],
        ..post()
    };

    assert_eq!(violations_of(&post), ["tags[0]: length", "tags[1]: required"]);
}

#[test]
fn the_first_rule_an_element_breaks_stops_its_other_rules() {
    let post = Post {
        tags: vec![String::new()],
        ..post()
    };

    assert_eq!(violations_of(&post), ["tags[0]: required"]);
}

#[test]
fn a_rule_after_required_on_optional_elements_checks_the_value_inside() {
    let post = Post {
        reviewers: vec![Some(String::new())],
        ..post()
    };

    assert_eq!(violations_of(&post), ["reviewers[0]: length"]);
}

#[test]
fn each_reports_a_map_value_at_its_key() {
    let post = Post {
        titles: BTreeMap::from([
            ("en".to_owned(), "Settings".to_owned()),
            ("fr".to_owned(), "Paramètres".to_owned()),
        ]),
        ..post()
    };

    assert_eq!(violations_of(&post), ["titles[fr]: length"]);
}

#[test]
fn each_nested_validates_every_element_as_a_struct() {
    let post = Post {
        authors: vec![
            Author {
                name: "Alex".to_owned(),
            },
            Author { name: String::new() },
        ],
        ..post()
    };

    assert_eq!(violations_of(&post), ["authors[1].name: required"]);
}

#[test]
fn required_on_optional_elements_checks_each_option() {
    let post = Post {
        reviewers: vec![Some("Sam".to_owned()), None],
        ..post()
    };

    assert_eq!(violations_of(&post), ["reviewers[1]: required"]);
}

#[test]
fn each_on_an_optional_collection_checks_the_elements_inside() {
    let post = Post {
        keywords: Some(vec!["a".to_owned()]),
        ..post()
    };

    assert_eq!(violations_of(&post), ["keywords[0]: length"]);
}

#[test]
fn each_inside_each_checks_the_elements_of_every_element() {
    let post = Post {
        grid: vec![vec!["ab".to_owned(), "abcd".to_owned()]],
        ..post()
    };

    assert_eq!(violations_of(&post), ["grid[0][1]: length"]);
}

#[test]
fn a_collection_breaking_a_rule_before_each_has_its_elements_skipped() {
    let post = Post {
        codes: vec!["abcd".to_owned(), "abcd".to_owned(), "abcd".to_owned()],
        ..post()
    };

    assert_eq!(violations_of(&post), ["codes: length"]);
}

#[derive(Validate)]
struct Shelf {
    #[validate(each(length(max = 3)))]
    slots: [Option<String>; 2],
    #[validate(each(length(max = 3)))]
    queue: VecDeque<String>,
    #[validate(each(length(max = 3)))]
    labels: HashMap<String, String>,
}

fn shelf() -> Shelf {
    Shelf {
        slots: [None, None],
        queue: VecDeque::new(),
        labels: HashMap::new(),
    }
}

#[test]
fn each_checks_the_value_inside_every_optional_element_of_an_array() {
    let shelf = Shelf {
        slots: [None, Some("jars".to_owned())],
        ..shelf()
    };

    assert_eq!(violations_of(&shelf), ["slots[1]: length"]);
}

#[test]
fn each_checks_every_element_of_a_queue() {
    let shelf = Shelf {
        queue: VecDeque::from(["tin".to_owned(), "jars".to_owned()]),
        ..shelf()
    };

    assert_eq!(violations_of(&shelf), ["queue[1]: length"]);
}

#[test]
fn each_checks_every_value_of_a_hash_map() {
    let shelf = Shelf {
        labels: HashMap::from([("top".to_owned(), "jars".to_owned())]),
        ..shelf()
    };

    assert_eq!(violations_of(&shelf), ["labels[top]: length"]);
}
