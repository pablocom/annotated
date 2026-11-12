use std::borrow::Cow;
use std::fmt;

use serde::Deserializer;
use serde::de::{Error, Unexpected, Visitor};

/// Reads a `Cow<str>` field with `#[serde(borrow)]`: borrowed when the format lends the string, owned otherwise, as
/// serde's derive reads it.
///
/// # Errors
///
/// Returns the format's error when the value isn't a string, or is bytes that aren't UTF-8.
pub fn borrow_cow_str<'de: 'a, 'a, D: Deserializer<'de>>(deserializer: D) -> Result<Cow<'a, str>, D::Error> {
    deserializer.deserialize_str(CowStrVisitor)
}

/// Reads a `Cow<[u8]>` field with `#[serde(borrow)]`: borrowed when the format lends the bytes, owned otherwise, as
/// serde's derive reads it.
///
/// # Errors
///
/// Returns the format's error when the value is neither bytes nor a string.
pub fn borrow_cow_bytes<'de: 'a, 'a, D: Deserializer<'de>>(deserializer: D) -> Result<Cow<'a, [u8]>, D::Error> {
    deserializer.deserialize_bytes(CowBytesVisitor)
}

struct CowStrVisitor;

impl<'a> Visitor<'a> for CowStrVisitor {
    type Value = Cow<'a, str>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a string")
    }

    fn visit_borrowed_str<E: Error>(self, string: &'a str) -> Result<Self::Value, E> {
        Ok(Cow::Borrowed(string))
    }

    fn visit_str<E: Error>(self, string: &str) -> Result<Self::Value, E> {
        Ok(Cow::Owned(string.to_owned()))
    }

    fn visit_string<E: Error>(self, string: String) -> Result<Self::Value, E> {
        Ok(Cow::Owned(string))
    }

    fn visit_borrowed_bytes<E: Error>(self, bytes: &'a [u8]) -> Result<Self::Value, E> {
        std::str::from_utf8(bytes)
            .map(Cow::Borrowed)
            .map_err(|_| E::invalid_value(Unexpected::Bytes(bytes), &self))
    }

    fn visit_bytes<E: Error>(self, bytes: &[u8]) -> Result<Self::Value, E> {
        std::str::from_utf8(bytes)
            .map(|string| Cow::Owned(string.to_owned()))
            .map_err(|_| E::invalid_value(Unexpected::Bytes(bytes), &self))
    }

    fn visit_byte_buf<E: Error>(self, bytes: Vec<u8>) -> Result<Self::Value, E> {
        String::from_utf8(bytes)
            .map(Cow::Owned)
            .map_err(|error| E::invalid_value(Unexpected::Bytes(error.as_bytes()), &self))
    }
}

struct CowBytesVisitor;

impl<'a> Visitor<'a> for CowBytesVisitor {
    type Value = Cow<'a, [u8]>;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a byte array")
    }

    fn visit_borrowed_str<E: Error>(self, string: &'a str) -> Result<Self::Value, E> {
        Ok(Cow::Borrowed(string.as_bytes()))
    }

    fn visit_str<E: Error>(self, string: &str) -> Result<Self::Value, E> {
        Ok(Cow::Owned(string.as_bytes().to_vec()))
    }

    fn visit_string<E: Error>(self, string: String) -> Result<Self::Value, E> {
        Ok(Cow::Owned(string.into_bytes()))
    }

    fn visit_borrowed_bytes<E: Error>(self, bytes: &'a [u8]) -> Result<Self::Value, E> {
        Ok(Cow::Borrowed(bytes))
    }

    fn visit_bytes<E: Error>(self, bytes: &[u8]) -> Result<Self::Value, E> {
        Ok(Cow::Owned(bytes.to_vec()))
    }

    fn visit_byte_buf<E: Error>(self, bytes: Vec<u8>) -> Result<Self::Value, E> {
        Ok(Cow::Owned(bytes))
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde_test::{Token, assert_de_tokens, assert_de_tokens_error};

    use super::*;

    #[derive(Debug, PartialEq)]
    enum Text<'a> {
        Borrowed(&'a str),
        Owned(String),
    }

    impl<'de: 'a, 'a> Deserialize<'de> for Text<'a> {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            Ok(match borrow_cow_str(deserializer)? {
                Cow::Borrowed(text) => Self::Borrowed(text),
                Cow::Owned(text) => Self::Owned(text),
            })
        }
    }

    #[derive(Debug, PartialEq)]
    enum Bytes<'a> {
        Borrowed(&'a [u8]),
        Owned(Vec<u8>),
    }

    impl<'de: 'a, 'a> Deserialize<'de> for Bytes<'a> {
        fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            Ok(match borrow_cow_bytes(deserializer)? {
                Cow::Borrowed(bytes) => Self::Borrowed(bytes),
                Cow::Owned(bytes) => Self::Owned(bytes),
            })
        }
    }

    #[test]
    fn a_string_the_format_lends_is_borrowed() {
        assert_de_tokens(&Text::Borrowed("Hola"), &[Token::BorrowedStr("Hola")]);
    }

    #[test]
    fn a_string_the_format_does_not_lend_is_owned() {
        assert_de_tokens(&Text::Owned("Hola".to_owned()), &[Token::Str("Hola")]);
        assert_de_tokens(&Text::Owned("Hola".to_owned()), &[Token::String("Hola")]);
    }

    #[test]
    fn utf_8_bytes_the_format_lends_are_a_borrowed_string() {
        assert_de_tokens(&Text::Borrowed("Hola"), &[Token::BorrowedBytes(b"Hola")]);
    }

    #[test]
    fn utf_8_bytes_the_format_does_not_lend_are_an_owned_string() {
        assert_de_tokens(&Text::Owned("Hola".to_owned()), &[Token::Bytes(b"Hola")]);
        assert_de_tokens(&Text::Owned("Hola".to_owned()), &[Token::ByteBuf(b"Hola")]);
    }

    #[test]
    fn bytes_that_are_not_utf_8_are_not_a_string() {
        let not_utf_8 = &[0xff, 0xfe];

        assert_de_tokens_error::<Text>(
            &[Token::BorrowedBytes(not_utf_8)],
            "invalid value: byte array, expected a string",
        );
        assert_de_tokens_error::<Text>(
            &[Token::Bytes(not_utf_8)],
            "invalid value: byte array, expected a string",
        );
        assert_de_tokens_error::<Text>(
            &[Token::ByteBuf(not_utf_8)],
            "invalid value: byte array, expected a string",
        );
    }

    #[test]
    fn bytes_the_format_lends_are_borrowed() {
        assert_de_tokens(&Bytes::Borrowed(b"PDF"), &[Token::BorrowedBytes(b"PDF")]);
        assert_de_tokens(&Bytes::Borrowed(b"PDF"), &[Token::BorrowedStr("PDF")]);
    }

    #[test]
    fn bytes_the_format_does_not_lend_are_owned() {
        assert_de_tokens(&Bytes::Owned(b"PDF".to_vec()), &[Token::Bytes(b"PDF")]);
        assert_de_tokens(&Bytes::Owned(b"PDF".to_vec()), &[Token::ByteBuf(b"PDF")]);
        assert_de_tokens(&Bytes::Owned(b"PDF".to_vec()), &[Token::Str("PDF")]);
        assert_de_tokens(&Bytes::Owned(b"PDF".to_vec()), &[Token::String("PDF")]);
    }
}
