//! The `url` rule agrees with a strict RFC 3986 and RFC 3987 validator, `iri-string`, on generated text.

use annotated::{Rule, rules};
use iri_string::types::IriStr;
use proptest::prelude::*;
use proptest::sample::select;

/// What the rule promises, told by `iri-string`: an absolute IRI with an authority whose host isn't empty,
/// without userinfo, of the scheme `http`, `https` or `ftp` in any case.
fn is_url_by_the_rfcs(text: &str) -> bool {
    let Ok(iri) = IriStr::new(text) else {
        return false;
    };
    let Some(authority) = iri.authority_components() else {
        return false;
    };
    let has_accepted_scheme = ["http", "https", "ftp"]
        .iter()
        .any(|scheme| scheme.eq_ignore_ascii_case(iri.scheme_str()));
    has_accepted_scheme && authority.userinfo().is_none() && !authority.host().is_empty()
}

fn is_url_by_the_rule(text: &str) -> bool {
    rules::url().check(text).is_ok()
}

/// Characters on both sides of every line the grammar draws: delimiters, characters it never allows, and the
/// edges of `ucschar` and `iprivate`.
fn character() -> impl Strategy<Value = char> {
    select(vec![
        'a',
        'z',
        'A',
        'Z',
        '0',
        '9',
        'f',
        'F',
        'g',
        'v',
        'V',
        '-',
        '.',
        '_',
        '~',
        '!',
        '$',
        '&',
        '\'',
        '(',
        ')',
        '*',
        '+',
        ',',
        ';',
        '=',
        ':',
        '@',
        '/',
        '?',
        '#',
        '[',
        ']',
        '%',
        ' ',
        '\t',
        '\n',
        '\\',
        '"',
        '<',
        '>',
        '^',
        '`',
        '{',
        '|',
        '}',
        '\u{7F}',
        '\u{80}',
        '\u{9F}',
        '\u{A0}',
        'ü',
        'ß',
        '納',
        '\u{D7FF}',
        '\u{E000}',
        '\u{F8FF}',
        '\u{F900}',
        '\u{FDCF}',
        '\u{FDD0}',
        '\u{FDEF}',
        '\u{FDF0}',
        '\u{FFEF}',
        '\u{FFF0}',
        '\u{FFFD}',
        '\u{FFFE}',
        '\u{10000}',
        '\u{1FFFD}',
        '\u{1FFFE}',
        '\u{E0001}',
        '\u{E0FFF}',
        '\u{E1000}',
        '\u{EFFFD}',
        '\u{EFFFE}',
        '\u{F0000}',
        '\u{FFFFD}',
        '\u{100000}',
        '\u{10FFFD}',
        '\u{10FFFF}',
    ])
}

fn text(most_characters: usize) -> impl Strategy<Value = String> {
    prop::collection::vec(character(), 0..=most_characters).prop_map(String::from_iter)
}

fn scheme() -> impl Strategy<Value = String> {
    prop_oneof![
        select(vec![
            "http", "https", "ftp", "HTTP", "hTtPs", "FTP", "sftp", "mailto", "h", "1http", ""
        ])
        .prop_map(str::to_owned),
        "[a-zA-Z][a-zA-Z0-9+.-]{0,5}",
        text(3),
    ]
}

fn separator() -> impl Strategy<Value = &'static str> {
    prop_oneof![
        4 => Just("://"),
        1 => select(vec![":", ":/", ":///", "//", "", ":\\\\"]),
    ]
}

fn userinfo() -> impl Strategy<Value = String> {
    prop_oneof![
        6 => Just(String::new()),
        1 => select(vec!["alex@", "alex:secret@", "@", ":@"]).prop_map(str::to_owned),
        1 => text(3).prop_map(|userinfo| userinfo + "@"),
    ]
}

fn host() -> impl Strategy<Value = String> {
    prop_oneof![
        select(vec![
            "example.com",
            "bücher.de",
            "a",
            "",
            "xn--rsum-bpad.example.org",
            "ex%41mple.com"
        ])
        .prop_map(str::to_owned),
        "[a-z0-9._~!$&'()*+,;=%-]{0,8}",
        ipv4_like(),
        ipv6_like().prop_map(|address| format!("[{address}]")),
        ipv6_like().prop_map(|address| format!("[{address}%25eth0]")),
        "\\[[vV][0-9a-fA-F]{0,2}\\.[a-z0-9:~!+]{0,4}\\]",
        text(6),
    ]
}

fn dec_octet_like() -> impl Strategy<Value = &'static str> {
    select(vec![
        "0", "9", "10", "99", "100", "199", "200", "249", "250", "255", "256", "00", "01", "010", "1000", "+1",
    ])
}

fn ipv4_like() -> impl Strategy<Value = String> {
    prop::collection::vec(dec_octet_like(), 3..=5).prop_map(|octets| octets.join("."))
}

fn h16_like() -> impl Strategy<Value = String> {
    prop_oneof![
        8 => "[0-9a-fA-F]{1,4}",
        1 => "[0-9a-fA-F]{5}|g|",
    ]
}

/// Pieces separated by `:`, maybe with an IPv4 address last, and maybe with a `::` among them.
fn ipv6_like() -> impl Strategy<Value = String> {
    (
        prop::collection::vec(h16_like(), 0..=8),
        prop::option::of(ipv4_like()),
        prop::option::of(any::<prop::sample::Index>()),
    )
        .prop_map(|(mut pieces, last_ipv4_address, double_colon)| {
            pieces.extend(last_ipv4_address);
            let Some(double_colon) = double_colon else {
                return pieces.join(":");
            };
            let (before, after) = pieces.split_at(double_colon.index(pieces.len() + 1));
            format!("{}::{}", before.join(":"), after.join(":"))
        })
}

fn port() -> impl Strategy<Value = String> {
    prop_oneof![
        4 => Just(String::new()),
        2 => "(:[0-9]{0,6})",
        1 => text(3).prop_map(|port| format!(":{port}")),
    ]
}

fn path() -> impl Strategy<Value = String> {
    prop_oneof![
        3 => prop::collection::vec(text(4), 0..=3).prop_map(|segments| {
            segments.iter().flat_map(|segment| ["/", segment.as_str()]).collect()
        }),
        1 => text(4),
    ]
}

fn query_or_fragment(delimiter: char) -> impl Strategy<Value = String> {
    prop_oneof![
        2 => Just(String::new()),
        1 => text(6).prop_map(move |text| format!("{delimiter}{text}")),
    ]
}

fn url_shaped_text() -> impl Strategy<Value = String> {
    (
        scheme(),
        separator(),
        userinfo(),
        host(),
        port(),
        path(),
        query_or_fragment('?'),
        query_or_fragment('#'),
    )
        .prop_map(|(scheme, separator, userinfo, host, port, path, query, fragment)| {
            format!("{scheme}{separator}{userinfo}{host}{port}{path}{query}{fragment}")
        })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(20_000))]

    #[test]
    fn the_rule_accepts_what_the_rfcs_call_a_url_and_nothing_else(text in url_shaped_text()) {
        prop_assert_eq!(is_url_by_the_rule(&text), is_url_by_the_rfcs(&text), "{:?}", text);
    }

    #[test]
    fn the_rule_agrees_with_the_rfcs_on_any_host_and_port(host in host(), port in port()) {
        let text = format!("https://{host}{port}/");

        prop_assert_eq!(is_url_by_the_rule(&text), is_url_by_the_rfcs(&text), "{:?}", text);
    }

    #[test]
    fn the_rule_agrees_with_the_rfcs_on_any_text(text in any::<String>()) {
        prop_assert_eq!(is_url_by_the_rule(&text), is_url_by_the_rfcs(&text), "{:?}", text);
    }
}
