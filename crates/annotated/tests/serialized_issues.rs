//! With the `serde` feature, issues serialize as data a program can send: a path as its segments, a problem as its
//! code and params, and an issue as both with its message.

#![cfg(feature = "serde")]

use annotated::{Issue, ParamValue, Path, PathSegment, Problem, Validate};

fn json_of(value: &impl serde::Serialize) -> String {
    serde_json::to_string(value).unwrap()
}

fn age_out_of_range() -> Problem {
    annotated::Rule::<u8>::problem(&annotated::rules::range().min(18_u8).max(120)).unwrap()
}

#[test]
fn a_path_serializes_as_its_segments_a_field_as_text_and_an_index_as_a_number() {
    let path = Path::root().field("members").index(1).field("name");

    assert_eq!(json_of(&path), r#"["members",1,"name"]"#);
}

#[test]
fn the_root_path_serializes_as_no_segments() {
    assert_eq!(json_of(&Path::root()), "[]");
}

#[test]
fn a_map_s_key_serializes_as_its_text() {
    let path = Path::root().field("labels").key(42);

    assert_eq!(json_of(&path), r#"["labels","42"]"#);
}

#[test]
fn a_key_serializes_as_it_is_without_the_escapes_it_is_printed_with() {
    let path = Path::root().key("a\nb");

    assert_eq!(json_of(&path), r#"["a\nb"]"#);
}

#[test]
fn a_path_segment_serializes_as_its_bare_value() {
    assert_eq!(
        (json_of(&PathSegment::Field("name")), json_of(&PathSegment::Index(3))),
        (r#""name""#.to_owned(), "3".to_owned())
    );
}

#[test]
fn an_integer_param_serializes_as_a_number() {
    assert_eq!(json_of(&ParamValue::Integer(-50)), "-50");
}

#[test]
fn an_integer_param_beyond_an_i64_serializes_as_the_number() {
    assert_eq!(json_of(&ParamValue::from(u64::MAX)), "18446744073709551615");
}

#[test]
fn an_integer_param_beyond_a_u64_serializes_as_the_number() {
    assert_eq!(
        json_of(&ParamValue::Integer(
            -170_141_183_460_469_231_731_687_303_715_884_105_728
        )),
        "-170141183460469231731687303715884105728"
    );
}

#[test]
fn an_integer_param_an_i64_holds_is_written_as_an_i64_for_formats_without_bigger_integers() {
    serde_test::assert_ser_tokens(&ParamValue::Integer(-50), &[serde_test::Token::I64(-50)]);
}

#[test]
fn an_integer_param_only_a_u64_holds_is_written_as_a_u64() {
    serde_test::assert_ser_tokens(&ParamValue::from(u64::MAX), &[serde_test::Token::U64(u64::MAX)]);
}

#[test]
fn a_float_param_serializes_as_a_number() {
    assert_eq!(json_of(&ParamValue::Float(0.5)), "0.5");
}

#[test]
fn a_text_param_serializes_as_a_string() {
    assert_eq!(json_of(&ParamValue::Text("admin".into())), r#""admin""#);
}

#[test]
fn a_problem_serializes_as_its_code_and_its_params_in_their_order() {
    assert_eq!(
        json_of(&age_out_of_range()),
        r#"{"code":"range","params":{"min":18,"max":120}}"#
    );
}

#[test]
fn a_problem_without_params_serializes_them_as_an_empty_object() {
    assert_eq!(json_of(&Problem::Required), r#"{"code":"required","params":{}}"#);
}

#[test]
fn a_custom_problem_serializes_its_params_as_numbers_and_text() {
    let problem = Problem::custom("reserved_name", "is reserved")
        .with_param("name", "admin")
        .with_param("max_ratio", 0.5);

    assert_eq!(
        json_of(&problem),
        r#"{"code":"reserved_name","params":{"name":"admin","max_ratio":0.5}}"#
    );
}

#[test]
fn an_issue_serializes_as_its_path_code_params_and_message() {
    let issue = Issue::new(Path::root().field("age"), age_out_of_range());

    assert_eq!(
        json_of(&issue),
        r#"{"path":["age"],"code":"range","params":{"min":18,"max":120},"message":"must be between 18 and 120"}"#
    );
}

#[test]
fn an_issue_with_a_message_of_its_own_serializes_that_message_filled_in() {
    let issue = Issue::new(Path::root().field("age"), age_out_of_range()).with_message("{min} or older");

    assert_eq!(
        json_of(&issue),
        r#"{"path":["age"],"code":"range","params":{"min":18,"max":120},"message":"18 or older"}"#
    );
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
struct NewMember {
    #[validate(required)]
    full_name: String,
    #[validate(range(min = 18, max = 120))]
    age: u8,
}

#[test]
fn issues_serialize_as_the_sequence_of_their_issues() {
    let new_member = NewMember {
        full_name: String::new(),
        age: 12,
    };

    let issues = new_member.validate().unwrap_err();

    assert_eq!(
        json_of(&issues),
        concat!(
            r#"[{"path":["fullName"],"code":"required","params":{},"message":"is required"},"#,
            r#"{"path":["age"],"code":"range","params":{"min":18,"max":120},"message":"must be between 18 and 120"}]"#
        )
    );
}
