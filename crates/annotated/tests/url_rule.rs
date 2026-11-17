//! The `url` rule agrees on generated text with a strict RFC 3986 and RFC 3987 validator, `iri-string`, and with the
//! departures from the RFCs its documentation lists.

use std::net::Ipv4Addr;
use std::ops::RangeInclusive;

use annotated::{Rule, rules};
use iri_string::types::IriStr;
use proptest::prelude::*;
use proptest::sample::select;

/// What the rule promises: an absolute IRI with an authority, of the scheme `http`, `https` or `ftp` in any case, as
/// `iri-string` tells once the `[`, `]` and `|` that browsers send in a query or a fragment are percent-encoded. Its
/// host isn't empty, isn't an `IPvFuture`, has no percent-encoding, and is an IPv4 address when it ends in a number.
/// It has no userinfo, a port up to 65535, and no whitespace or invisible character.
fn is_url_by_the_rfcs_and_the_departures(text: &str) -> bool {
    let text = with_browser_characters_encoded(text);
    let Ok(iri) = IriStr::new(&text) else {
        return false;
    };
    let Some(authority) = iri.authority_components() else {
        return false;
    };
    let has_accepted_scheme = ["http", "https", "ftp"]
        .iter()
        .any(|scheme| scheme.eq_ignore_ascii_case(iri.scheme_str()));
    let host = authority.host();
    let is_accepted_host = !host.is_empty()
        && !host.starts_with("[v")
        && !host.starts_with("[V")
        && !host.contains('%')
        && (!ends_in_a_number(host) || is_ipv4_address(host));
    let is_accepted_port = authority
        .port()
        .is_none_or(|port| port.is_empty() || port.parse::<u16>().is_ok());
    has_accepted_scheme
        && authority.userinfo().is_none()
        && is_accepted_host
        && is_accepted_port
        && !text.chars().any(is_whitespace_or_invisible)
}

/// `text` with `[`, `]` and `|` percent-encoded after the first `?` or `#`, which starts the query or the fragment of
/// any URL.
fn with_browser_characters_encoded(text: &str) -> String {
    let (before_query, from_query) = text.split_at(text.find(['?', '#']).unwrap_or(text.len()));
    let encoded = from_query.replace('[', "%5B").replace(']', "%5D").replace('|', "%7C");
    format!("{before_query}{encoded}")
}

/// The URL Standard's ends-in-a-number checker.
fn ends_in_a_number(host: &str) -> bool {
    let mut labels: Vec<&str> = host.split('.').collect();
    if labels.len() > 1 && labels.last() == Some(&"") {
        labels.pop();
    }
    let last = labels.last().copied().unwrap_or_default();
    let is_decimal = !last.is_empty() && last.chars().all(|character| character.is_ascii_digit());
    let hexadecimal_digits = last.strip_prefix("0x").or_else(|| last.strip_prefix("0X"));
    let is_hexadecimal =
        hexadecimal_digits.is_some_and(|digits| digits.chars().all(|character| character.is_ascii_hexdigit()));
    is_decimal || is_hexadecimal
}

/// An IPv4 address of four decimal numbers without leading zeros, maybe followed by a dot.
fn is_ipv4_address(host: &str) -> bool {
    host.strip_suffix('.').unwrap_or(host).parse::<Ipv4Addr>().is_ok()
}

fn is_whitespace_or_invisible(character: char) -> bool {
    character.is_whitespace() || DEFAULT_IGNORABLE.iter().any(|range| range.contains(&character))
}

/// `Default_Ignorable_Code_Point` in Unicode 18.0's `DerivedCoreProperties.txt`.
const DEFAULT_IGNORABLE: [RangeInclusive<char>; 17] = [
    '\u{AD}'..='\u{AD}',
    '\u{34F}'..='\u{34F}',
    '\u{61C}'..='\u{61C}',
    '\u{115F}'..='\u{1160}',
    '\u{17B4}'..='\u{17B5}',
    '\u{180B}'..='\u{180F}',
    '\u{200B}'..='\u{200F}',
    '\u{202A}'..='\u{202E}',
    '\u{2060}'..='\u{206F}',
    '\u{3164}'..='\u{3164}',
    '\u{FE00}'..='\u{FE0F}',
    '\u{FEFF}'..='\u{FEFF}',
    '\u{FFA0}'..='\u{FFA0}',
    '\u{FFF0}'..='\u{FFF8}',
    '\u{1BCA0}'..='\u{1BCA3}',
    '\u{1D173}'..='\u{1D17A}',
    '\u{E0000}'..='\u{E0FFF}',
];

fn is_url_by_the_rule(text: &str) -> bool {
    rules::url().check(text).is_ok()
}

/// Characters on both sides of every line the grammar draws: delimiters, characters it never allows, the edges of
/// `ucschar` and `iprivate`, and whitespace and invisible characters among `ucschar`.
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
        '\u{A1}',
        '\u{AD}',
        '\u{AE}',
        '\u{2028}',
        '\u{200B}',
        '\u{206F}',
        '\u{2070}',
        '\u{3000}',
        '\u{3164}',
        '\u{FE0F}',
        '\u{FEFF}',
        '\u{1D173}',
        '\u{1D17B}',
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
            "ex%41mple.com",
            "%31%32%37.0.0.1",
            "2130706433",
            "example.0x1f",
            "example.0x1g",
            "192.0.2.16.",
            "256.0.0.1..",
        ])
        .prop_map(str::to_owned),
        "[a-z0-9._~!$&'()*+,;=%-]{0,8}",
        ipv4_like(),
        ipv4_like().prop_map(|address| address + "."),
        ipv6_like().prop_map(|address| format!("[{address}]")),
        ipv6_like().prop_map(|address| format!("[{address}%25eth0]")),
        "\\[[vV][0-9a-fA-F]{0,2}\\.[a-z0-9:~!+]{0,4}\\]",
        text(6),
    ]
}

fn dec_octet_like() -> impl Strategy<Value = &'static str> {
    select(vec![
        "0", "9", "10", "99", "100", "199", "200", "249", "250", "255", "256", "00", "01", "010", "1000", "+1", "0x7f",
        "0x",
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
        1 => select(vec![":65535", ":65536", ":00080"]).prop_map(str::to_owned),
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
    fn the_rule_accepts_what_the_rfcs_call_a_url_but_for_its_departures(text in url_shaped_text()) {
        prop_assert_eq!(is_url_by_the_rule(&text), is_url_by_the_rfcs_and_the_departures(&text), "{:?}", text);
    }

    #[test]
    fn the_rule_agrees_with_the_rfcs_and_its_departures_on_any_host_and_port(host in host(), port in port()) {
        let text = format!("https://{host}{port}/");

        prop_assert_eq!(is_url_by_the_rule(&text), is_url_by_the_rfcs_and_the_departures(&text), "{:?}", text);
    }

    #[test]
    fn the_rule_agrees_with_the_rfcs_and_its_departures_on_any_text(text in any::<String>()) {
        prop_assert_eq!(is_url_by_the_rule(&text), is_url_by_the_rfcs_and_the_departures(&text), "{:?}", text);
    }
}
