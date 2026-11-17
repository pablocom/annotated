//! A map's key comes from the input, so a path prints it with every char that could break a line or hide text
//! escaped, and a key can't forge an issue line. Its JSON Pointer keeps the key as it is.

use std::collections::BTreeMap;

use annotated::{Path, PathSegment, Validate};

#[derive(Validate)]
struct Catalog {
    #[validate(each(required))]
    titles: BTreeMap<String, String>,
}

#[test]
fn a_newline_in_a_key_is_printed_escaped() {
    let path = Path::root().field("labels").key("a\nadmin: must be true");

    assert_eq!(path.to_string(), r"labels[a\nadmin: must be true]");
}

#[test]
fn line_and_paragraph_separators_in_a_key_are_printed_escaped() {
    let path = Path::root().key("a\r\u{2028}b\u{2029}c");

    assert_eq!(path.to_string(), r"[a\r\u{2028}b\u{2029}c]");
}

#[test]
fn chars_that_hide_or_reorder_text_in_a_key_are_printed_escaped() {
    let path = Path::root().key("a\u{202e}b\u{200b}c\u{1b}[2J");

    assert_eq!(path.to_string(), r"[a\u{202e}b\u{200b}c\u{1b}[2J]");
}

#[test]
fn a_backslash_in_a_key_is_printed_escaped() {
    let path = Path::root().key(r"a\nb");

    assert_eq!(path.to_string(), r"[a\\nb]");
}

#[test]
fn quotes_and_printable_non_ascii_chars_in_a_key_are_printed_as_they_are() {
    let path = Path::root().key(r#"Grüße "en" 'de' café"#);

    assert_eq!(path.to_string(), r#"[Grüße "en" 'de' café]"#);
}

#[test]
fn a_key_segment_is_printed_escaped() {
    let segment = PathSegment::Key("a\nb".to_owned());

    assert_eq!(segment.to_string(), r"a\nb");
}

#[test]
fn a_newline_in_a_key_cannot_forge_a_line_of_the_issues() {
    let catalog = Catalog {
        titles: BTreeMap::from([("a\nadmin: must be true".to_owned(), String::new())]),
    };

    let issues = catalog.validate().unwrap_err();

    assert_eq!(issues.to_string(), r"titles[a\nadmin: must be true]: is required");
}

#[test]
fn a_json_pointer_keeps_a_newline_in_a_key_as_it_is() {
    let path = Path::root().field("labels").key("a\nb");

    assert_eq!(path.json_pointer(), "/labels/a\nb");
}

#[cfg(feature = "serde")]
mod deserializing {
    use std::collections::BTreeMap;

    use annotated::Deserialize;

    #[derive(Deserialize, Debug)]
    #[allow(dead_code)]
    struct Catalog {
        #[validate(each(required))]
        titles: BTreeMap<String, String>,
    }

    #[test]
    fn a_newline_in_a_key_cannot_forge_a_line_of_the_message_of_a_serde_error() {
        let body = r#"{ "titles": { "a\nadmin: must be true": "" } }"#;

        let error = serde_json::from_str::<Catalog>(body).unwrap_err();

        assert_eq!(error.to_string(), r"titles[a\nadmin: must be true]: is required");
    }
}
