//! `#[derive(Deserialize)]` reads a struct however a format presents it: by the name serde gives the struct, and
//! with its fields named in text, in bytes or by their index.

#![cfg(feature = "serde")]

use annotated::Deserialize;
use serde_test::{Token, assert_de_tokens, assert_de_tokens_error};

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename = "Member")]
struct NewMember {
    #[serde(rename = "e-mail", alias = "mail")]
    #[validate(email)]
    email: String,
    #[validate(range(min = 18))]
    age: u8,
}

fn alex() -> NewMember {
    NewMember {
        email: "alex@example.com".to_owned(),
        age: 30,
    }
}

#[test]
fn a_struct_is_read_by_the_name_serde_gives_it() {
    assert_de_tokens(
        &alex(),
        &[
            Token::Struct { name: "Member", len: 2 },
            Token::Str("e-mail"),
            Token::Str("alex@example.com"),
            Token::Str("age"),
            Token::U8(30),
            Token::StructEnd,
        ],
    );
}

#[test]
fn a_field_named_in_bytes_is_read_by_every_name_serde_gives_it() {
    assert_de_tokens(
        &alex(),
        &[
            Token::Struct { name: "Member", len: 2 },
            Token::Bytes(b"mail"),
            Token::Str("alex@example.com"),
            Token::Bytes(b"age"),
            Token::U8(30),
            Token::StructEnd,
        ],
    );
}

#[test]
fn a_field_named_by_its_index_is_read() {
    assert_de_tokens(
        &alex(),
        &[
            Token::Struct { name: "Member", len: 2 },
            Token::U64(0),
            Token::Str("alex@example.com"),
            Token::U64(1),
            Token::U8(30),
            Token::StructEnd,
        ],
    );
}

#[test]
fn an_unknown_field_named_in_bytes_or_by_index_is_ignored() {
    assert_de_tokens(
        &alex(),
        &[
            Token::Struct { name: "Member", len: 4 },
            Token::Bytes(b"nickname"),
            Token::Str("Al"),
            Token::Str("e-mail"),
            Token::Str("alex@example.com"),
            Token::U64(7),
            Token::Bool(true),
            Token::Str("age"),
            Token::U8(30),
            Token::StructEnd,
        ],
    );
}

#[test]
fn issues_are_found_whatever_way_the_fields_are_named() {
    assert_de_tokens_error::<NewMember>(
        &[
            Token::Struct { name: "Member", len: 2 },
            Token::Bytes(b"mail"),
            Token::Str("alex"),
            Token::U64(1),
            Token::U8(12),
            Token::StructEnd,
        ],
        "e-mail: must be an email address\nage: must be at least 18",
    );
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
struct LogIn {
    #[validate(email)]
    email: String,
}

#[test]
fn an_unknown_field_named_in_bytes_is_the_format_s_error_when_the_struct_denies_unknown_fields() {
    assert_de_tokens_error::<LogIn>(
        &[Token::Struct { name: "LogIn", len: 1 }, Token::Bytes(b"password")],
        "unknown field `password`, expected `email`",
    );
}

#[test]
fn an_unknown_field_index_is_the_format_s_error_when_the_struct_denies_unknown_fields() {
    assert_de_tokens_error::<LogIn>(
        &[Token::Struct { name: "LogIn", len: 1 }, Token::U64(1)],
        "invalid value: integer `1`, expected field index 0 <= i < 1",
    );
}
