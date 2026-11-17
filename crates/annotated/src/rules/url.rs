use crate::{Problem, Rule};

/// The rule that a string is an absolute URL with a host, whose scheme is `http`, `https` or `ftp`, or one of those
/// [`Url::schemes`] sets.
///
/// A URL here is an absolute URI with an authority, as [RFC 3986](https://www.rfc-editor.org/rfc/rfc3986#appendix-A)
/// defines it in section 3 and appendix A: `scheme "://" authority path-abempty [ "?" query ] [ "#" fragment ]`.
///
/// - [RFC 3987](https://www.rfc-editor.org/rfc/rfc3987#section-2.2) adds international characters: `ucschar` in the
///   host, path, query and fragment, and `iprivate` in the query. So `https://bücher.de/straße` is a URL.
/// - The scheme is compared with the accepted ones ignoring ASCII case.
/// - The host is an IPv6 address in brackets, an IPv4 address, or a registered name. The port can be empty.
/// - A `%` must be followed by two hexadecimal digits.
/// - Anything the grammar doesn't allow breaks the rule: whitespace anywhere, including around the URL, a backslash, a
///   control character.
///
/// The rule departs from that grammar where it accepts what no one means by a URL, or refuses what browsers send:
///
/// - The host can't be empty, as [RFC 9110 section 4.2.1](https://www.rfc-editor.org/rfc/rfc9110#section-4.2.1)
///   requires of `http`.
/// - A URL with userinfo, which [RFC 9110 section 4.2.4](https://www.rfc-editor.org/rfc/rfc9110#section-4.2.4)
///   deprecates, breaks the rule whatever its scheme.
/// - The port is at most 65535: `http://example.com:99999` breaks the rule.
/// - The host is never an `IPvFuture` literal, such as `[v1.x]`.
/// - A host that ends in a number is an IPv4 address: four decimal numbers from 0 to 255, without leading zeros. So
///   `http://256.0.0.1`, `http://2130706433`, `http://1.2.3` and `http://0x7f.0.0.1` break the rule, where a browser
///   would read some of them as another address. A host ends in a number as the
///   [URL Standard](https://url.spec.whatwg.org/#ends-in-a-number-checker) says: its last label, after one trailing dot
///   is set aside, is all digits, or `0x` followed by hexadecimal digits. So `http://192.0.2.16.` follows the rule and
///   `http://256.0.0.1.` breaks it.
/// - The host has no percent-encoding: `http://%31%32%37.0.0.1` breaks the rule.
/// - Whitespace and invisible characters break the rule anywhere, even those RFC 3987 allows: Unicode's
///   `White_Space`, such as U+00A0 and U+2028, and its `Default_Ignorable_Code_Point`, such as U+00AD, U+200B and the
///   bidirectional controls U+202A to U+202E and U+2066 to U+2069.
/// - A query and a fragment also take `[`, `]` and `|`, which browsers send as they are:
///   `https://api.example/items?filter[x]=1` follows the rule.
///
/// It checks anything that is `AsRef<str>`, exactly as written, and allocates nothing.
///
/// ```
/// use annotated::{Rule, rules};
///
/// assert_eq!(rules::url().check("ftp://example.com/report.pdf"), Ok(()));
///
/// let not_https = rules::url().schemes(&["https"]).check("http://example.com").unwrap_err();
/// assert_eq!(not_https.to_string(), "must be a URL using https");
/// ```
#[must_use]
pub const fn url() -> Url {
    Url {
        schemes: &["http", "https", "ftp"],
    }
}

/// Checks that a string is a URL with an accepted scheme. Made by [`url()`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Url {
    schemes: &'static [&'static str],
}

impl Url {
    /// The rule accepting URLs with one of these `schemes`, in place of `http`, `https` and `ftp`:
    /// `schemes = &["https"]`.
    #[must_use]
    pub const fn schemes(self, schemes: &'static [&'static str]) -> Self {
        Self { schemes }
    }

    fn accepts(self, scheme: &str) -> bool {
        self.schemes
            .iter()
            .any(|accepted| accepted.eq_ignore_ascii_case(scheme))
    }
}

#[diagnostic::do_not_recommend]
impl<T: AsRef<str> + ?Sized> Rule<T> for Url {
    fn check(&self, value: &T) -> Result<(), Problem> {
        let has_accepted_scheme = scheme_of_url(value.as_ref()).is_some_and(|scheme| self.accepts(scheme));
        if has_accepted_scheme {
            return Ok(());
        }
        Err(Problem::Url { schemes: self.schemes })
    }

    fn problem(&self) -> Option<Problem> {
        Some(Problem::Url { schemes: self.schemes })
    }
}

/// The scheme of `text` when the whole of it is `scheme "://" authority path-abempty [ "?" iquery ] [ "#"
/// ifragment ]`.
fn scheme_of_url(text: &str) -> Option<&str> {
    let (scheme, after_scheme) = scheme(text)?;
    let after_authority = authority(after_scheme.strip_prefix("://")?)?;
    let after_path = path_abempty(after_authority);
    let after_query = after_path.strip_prefix('?').map_or(after_path, iquery);
    let after_fragment = after_query.strip_prefix('#').map_or(after_query, ifragment);
    after_fragment.is_empty().then_some(scheme)
}

/// `scheme = ALPHA *( ALPHA / DIGIT / "+" / "-" / "." )`, and what follows it.
fn scheme(text: &str) -> Option<(&str, &str)> {
    let length = text
        .find(|character: char| !(character.is_ascii_alphanumeric() || matches!(character, '+' | '-' | '.')))
        .unwrap_or(text.len());
    let (scheme, rest) = text.split_at(length);
    scheme
        .starts_with(|character: char| character.is_ascii_alphabetic())
        .then_some((scheme, rest))
}

/// `authority = host [ ":" port ]`: RFC 3986's without `userinfo "@"`, and with a host that isn't empty.
fn authority(text: &str) -> Option<&str> {
    let after_host = host(text)?;
    match after_host.strip_prefix(':') {
        Some(after_colon) => port(after_colon),
        None => Some(after_host),
    }
}

/// `ihost = IP-literal / IPv4address / ireg-name`, not empty, and an `IPv4address` when it ends in a number.
fn host(text: &str) -> Option<&str> {
    if let Some(after_bracket) = text.strip_prefix('[') {
        return ip_literal(after_bracket);
    }
    let (name, rest) = text.split_at(text.len() - ireg_name(text).len());
    let is_host = if ends_in_a_number(name) {
        ipv4_address(name.strip_suffix('.').unwrap_or(name))
    } else {
        !name.is_empty()
    };
    is_host.then_some(rest)
}

/// Whether a browser reads `name` as an IPv4 address, by the URL Standard's ends-in-a-number checker: its last label,
/// after one trailing dot is set aside, is all digits, or `0x` or `0X` followed by hexadecimal digits.
fn ends_in_a_number(name: &str) -> bool {
    let without_trailing_dot = name.strip_suffix('.').unwrap_or(name);
    let last_label = without_trailing_dot
        .rsplit_once('.')
        .map_or(without_trailing_dot, |(_, last_label)| last_label);
    let is_decimal = !last_label.is_empty() && last_label.bytes().all(|byte| byte.is_ascii_digit());
    let is_hexadecimal = last_label
        .strip_prefix("0x")
        .or_else(|| last_label.strip_prefix("0X"))
        .is_some_and(|digits| digits.bytes().all(|byte| byte.is_ascii_hexdigit()));
    is_decimal || is_hexadecimal
}

/// `IP-literal = "[" IPv6address "]"`, after its `[`: RFC 3986's without `IPvFuture`.
fn ip_literal(text: &str) -> Option<&str> {
    let (address, rest) = text.split_once(']')?;
    ipv6_address(address).then_some(rest)
}

/// `IPv6address`: eight 16-bit pieces separated by `:`, or fewer around one `::` standing for at least one
/// piece left out.
fn ipv6_address(text: &str) -> bool {
    let Some((before, after)) = text.split_once("::") else {
        return pieces(text) == Some(8);
    };
    let pieces_before = if before.is_empty() { Some(0) } else { h16s(before) };
    let pieces_after = if after.is_empty() { Some(0) } else { pieces(after) };
    matches!((pieces_before, pieces_after), (Some(before), Some(after)) if before + after <= 7)
}

/// How many 16-bit pieces `text` stands for: `h16`s separated by `:`, the last of which can be an `IPv4address`,
/// worth two (`ls32`).
fn pieces(text: &str) -> Option<usize> {
    match text.rsplit_once(':') {
        Some((leading, last)) if ipv4_address(last) => Some(h16s(leading)? + 2),
        None if ipv4_address(text) => Some(2),
        _ => h16s(text),
    }
}

/// How many `h16 = 1*4HEXDIG` `text` is, separated by `:`.
fn h16s(text: &str) -> Option<usize> {
    text.split(':')
        .try_fold(0, |count, piece| h16(piece).then_some(count + 1))
}

/// `h16 = 1*4HEXDIG`.
fn h16(text: &str) -> bool {
    (1..=4).contains(&text.len()) && text.chars().all(|character| character.is_ascii_hexdigit())
}

/// `IPv4address = dec-octet "." dec-octet "." dec-octet "." dec-octet`.
fn ipv4_address(text: &str) -> bool {
    text.split('.')
        .try_fold(0, |count, octet| dec_octet(octet).then_some(count + 1))
        == Some(4)
}

/// `dec-octet`: a number from 0 to 255, without leading zeros.
fn dec_octet(text: &str) -> bool {
    let is_digits = !text.is_empty() && text.chars().all(|character| character.is_ascii_digit());
    let has_leading_zero = text.len() > 1 && text.starts_with('0');
    is_digits && !has_leading_zero && text.parse::<u8>().is_ok()
}

/// `ireg-name = *( iunreserved / sub-delims )`: RFC 3987's without `pct-encoded`, and what follows it.
fn ireg_name(text: &str) -> &str {
    text.trim_start_matches(|character| is_iunreserved(character) || is_sub_delim(character))
}

/// `port = *DIGIT`, of a number up to 65535, and what follows it.
fn port(text: &str) -> Option<&str> {
    let rest = text.trim_start_matches(|character: char| character.is_ascii_digit());
    let digits = &text[..text.len() - rest.len()];
    (digits.is_empty() || digits.parse::<u16>().is_ok()).then_some(rest)
}

/// `ipath-abempty = *( "/" isegment )`, and what follows it.
fn path_abempty(text: &str) -> &str {
    if !text.starts_with('/') {
        return text;
    }
    after_run_of(text, |character| character == '/' || is_ipchar(character))
}

/// `iquery = *( ipchar / iprivate / "/" / "?" )`, with what browsers leave unencoded, and what follows it.
fn iquery(text: &str) -> &str {
    after_run_of(text, |character| {
        is_ipchar(character)
            || is_iprivate(character)
            || matches!(character, '/' | '?')
            || is_left_unencoded_by_browsers(character)
    })
}

/// `ifragment = *( ipchar / "/" / "?" )`, with what browsers leave unencoded, and what follows it.
fn ifragment(text: &str) -> &str {
    after_run_of(text, |character| {
        is_ipchar(character) || matches!(character, '/' | '?') || is_left_unencoded_by_browsers(character)
    })
}

/// `[`, `]` and `|`, which RFC 3986 doesn't allow in a query or a fragment, but browsers send there as they are:
/// `?filter[x]=1`, `?family=A|B`.
fn is_left_unencoded_by_browsers(character: char) -> bool {
    matches!(character, '[' | ']' | '|')
}

/// What follows the longest run at the start of `text` of percent-encodings and characters `is_allowed`.
fn after_run_of(text: &str, is_allowed: impl Fn(char) -> bool) -> &str {
    let mut rest = text;
    loop {
        if let Some(after) = percent_encoded(rest) {
            rest = after;
            continue;
        }
        let mut characters = rest.chars();
        match characters.next() {
            Some(character) if is_allowed(character) => rest = characters.as_str(),
            _ => return rest,
        }
    }
}

/// `pct-encoded = "%" HEXDIG HEXDIG`, and what follows it.
fn percent_encoded(text: &str) -> Option<&str> {
    let (digits, rest) = text.strip_prefix('%')?.split_at_checked(2)?;
    digits
        .chars()
        .all(|character| character.is_ascii_hexdigit())
        .then_some(rest)
}

/// `ipchar = iunreserved / pct-encoded / sub-delims / ":" / "@"`, but for `pct-encoded`.
fn is_ipchar(character: char) -> bool {
    is_iunreserved(character) || is_sub_delim(character) || matches!(character, ':' | '@')
}

/// `iunreserved = ALPHA / DIGIT / "-" / "." / "_" / "~" / ucschar`, without whitespace or invisible characters.
fn is_iunreserved(character: char) -> bool {
    is_unreserved(character) || (is_ucschar(character) && !is_whitespace_or_invisible(character))
}

/// Unicode's `White_Space` and `Default_Ignorable_Code_Point` characters: those that show as blank space, or not at
/// all.
fn is_whitespace_or_invisible(character: char) -> bool {
    character.is_whitespace() || is_default_ignorable(character)
}

/// `Default_Ignorable_Code_Point`, as Unicode 18.0's `DerivedCoreProperties.txt` lists it.
fn is_default_ignorable(character: char) -> bool {
    matches!(
        character,
        '\u{AD}'
            | '\u{34F}'
            | '\u{61C}'
            | '\u{115F}'..='\u{1160}'
            | '\u{17B4}'..='\u{17B5}'
            | '\u{180B}'..='\u{180F}'
            | '\u{200B}'..='\u{200F}'
            | '\u{202A}'..='\u{202E}'
            | '\u{2060}'..='\u{206F}'
            | '\u{3164}'
            | '\u{FE00}'..='\u{FE0F}'
            | '\u{FEFF}'
            | '\u{FFA0}'
            | '\u{FFF0}'..='\u{FFF8}'
            | '\u{1BCA0}'..='\u{1BCA3}'
            | '\u{1D173}'..='\u{1D17A}'
            | '\u{E0000}'..='\u{E0FFF}'
    )
}

/// `unreserved = ALPHA / DIGIT / "-" / "." / "_" / "~"`.
fn is_unreserved(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '-' | '.' | '_' | '~')
}

/// `sub-delims = "!" / "$" / "&" / "'" / "(" / ")" / "*" / "+" / "," / ";" / "="`.
fn is_sub_delim(character: char) -> bool {
    matches!(
        character,
        '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '='
    )
}

/// `ucschar`: the characters beyond ASCII an IRI takes anywhere but in its scheme.
fn is_ucschar(character: char) -> bool {
    matches!(
        character,
        '\u{A0}'..='\u{D7FF}'
            | '\u{F900}'..='\u{FDCF}'
            | '\u{FDF0}'..='\u{FFEF}'
            | '\u{10000}'..='\u{1FFFD}'
            | '\u{20000}'..='\u{2FFFD}'
            | '\u{30000}'..='\u{3FFFD}'
            | '\u{40000}'..='\u{4FFFD}'
            | '\u{50000}'..='\u{5FFFD}'
            | '\u{60000}'..='\u{6FFFD}'
            | '\u{70000}'..='\u{7FFFD}'
            | '\u{80000}'..='\u{8FFFD}'
            | '\u{90000}'..='\u{9FFFD}'
            | '\u{A0000}'..='\u{AFFFD}'
            | '\u{B0000}'..='\u{BFFFD}'
            | '\u{C0000}'..='\u{CFFFD}'
            | '\u{D0000}'..='\u{DFFFD}'
            | '\u{E1000}'..='\u{EFFFD}'
    )
}

/// `iprivate`: the private-use characters an IRI takes only in its query.
fn is_iprivate(character: char) -> bool {
    matches!(
        character,
        '\u{E000}'..='\u{F8FF}' | '\u{F0000}'..='\u{FFFFD}' | '\u{100000}'..='\u{10FFFD}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_says_the_schemes_it_accepts() {
        assert_eq!(
            Rule::<str>::problem(&url().schemes(&["https"])),
            Some(Problem::Url { schemes: &["https"] })
        );
    }

    fn not_a_url() -> Result<(), Problem> {
        Err(Problem::Url {
            schemes: &["http", "https", "ftp"],
        })
    }

    fn not_a_url_of(schemes: &'static [&'static str]) -> Result<(), Problem> {
        Err(Problem::Url { schemes })
    }

    fn refused_by(rule: Url, texts: &[&'static str]) -> Vec<&'static str> {
        texts
            .iter()
            .copied()
            .filter(|text| rule.check(*text).is_err())
            .collect()
    }

    fn accepted_by(rule: Url, texts: &[&'static str]) -> Vec<&'static str> {
        texts.iter().copied().filter(|text| rule.check(*text).is_ok()).collect()
    }

    const NONE: [&str; 0] = [];

    #[test]
    fn an_https_url_follows_the_rule() {
        assert_eq!(url().check("https://example.com/projects/1?tab=members#top"), Ok(()));
    }

    #[test]
    fn an_http_url_follows_the_rule() {
        assert_eq!(url().check("http://example.com"), Ok(()));
    }

    #[test]
    fn an_ftp_url_follows_the_rule() {
        assert_eq!(url().check("ftp://example.com/report.pdf"), Ok(()));
    }

    #[test]
    fn a_url_of_another_scheme_breaks_the_rule() {
        assert_eq!(url().check("mailto:alex@example.com"), not_a_url());
    }

    #[test]
    fn a_url_of_a_scheme_set_follows_the_rule() {
        assert_eq!(url().schemes(&["ftp", "sftp"]).check("sftp://example.com"), Ok(()));
    }

    #[test]
    fn schemes_set_replace_the_default_ones() {
        assert_eq!(
            url().schemes(&["https"]).check("http://example.com"),
            not_a_url_of(&["https"])
        );
    }

    #[test]
    fn a_scheme_is_accepted_whatever_its_case() {
        assert_eq!(
            refused_by(
                url().schemes(&["HTTPS"]),
                &["https://example.com", "HTTPS://example.com", "hTtPs://example.com"]
            ),
            NONE
        );
    }

    #[test]
    fn a_relative_url_breaks_the_rule() {
        assert_eq!(url().check("/projects/1"), not_a_url());
    }

    #[test]
    fn a_scheme_alone_breaks_the_rule() {
        assert_eq!(url().check("http://"), not_a_url());
    }

    #[test]
    fn a_url_without_an_authority_breaks_the_rule() {
        assert_eq!(url().check("https:example.com"), not_a_url());
    }

    #[test]
    fn a_url_with_a_space_breaks_the_rule() {
        assert_eq!(url().check("https://example.com/a b"), not_a_url());
    }

    #[test]
    fn a_url_with_credentials_breaks_the_rule() {
        assert_eq!(url().check("https://alex:secret@example.com"), not_a_url());
    }

    #[test]
    fn surrounding_whitespace_breaks_the_rule() {
        assert_eq!(url().check(" https://example.com"), not_a_url());
    }

    #[test]
    fn the_rfc_3986_examples_with_an_authority_are_urls() {
        let rule = url().schemes(&["ftp", "http", "ldap", "telnet", "foo"]);

        assert_eq!(
            refused_by(
                rule,
                &[
                    "ftp://ftp.is.co.za/rfc/rfc1808.txt",
                    "http://www.ietf.org/rfc/rfc2396.txt",
                    "ldap://[2001:db8::7]/c=GB?objectClass?one",
                    "telnet://192.0.2.16:80/",
                    "foo://example.com:8042/over/there?name=ferret#nose",
                    "http://a/b/c/d;p?q",
                    "http://a/b/c/g;x?y#s",
                    "http://a/b/c/g?y/./x",
                    "http://a/b/c/g#s/../x",
                    "http://example.com",
                    "http://example.com/",
                    "http://example.com:/",
                    "http://example.com:80/",
                ]
            ),
            NONE
        );
    }

    #[test]
    fn the_rfc_3986_examples_without_an_authority_are_not_urls() {
        let rule = url().schemes(&["mailto", "news", "tel", "urn"]);

        assert_eq!(
            accepted_by(
                rule,
                &[
                    "mailto:John.Doe@example.com",
                    "news:comp.infosystems.www.servers.unix",
                    "tel:+1-816-555-1212",
                    "urn:oasis:names:specification:docbook:dtd:xml:4.1.2",
                ]
            ),
            NONE
        );
    }

    #[test]
    fn the_rfc_3987_examples_are_urls() {
        assert_eq!(
            refused_by(
                url(),
                &[
                    "http://résumé.example.org",
                    "http://xn--rsum-bpad.example.org",
                    "http://www.example.org/red%09ros%C3%A9#red",
                    "http://example.com/𐌀𐌁𐌂",
                    "http://www.example.org/Dürst",
                    "http://www.example.org/D%C3%BCrst",
                    "http://納豆.example.org/%E2%80%AE",
                    "http://ab.CDEFGH.ij/kl/mn/op.html",
                    "https://bücher.de/straße",
                ]
            ),
            NONE
        );
    }

    #[test]
    fn a_host_can_be_an_ip_address() {
        assert_eq!(
            refused_by(
                url(),
                &[
                    "http://192.0.2.16",
                    "http://[::]",
                    "http://[::1]",
                    "http://[2001:db8::]",
                    "http://[2001:db8:0:0:0:0:2:1]",
                    "http://[1:2:3:4:5:6:7::]",
                    "http://[::2:3:4:5:6:7:8]",
                    "http://[::ffff:192.0.2.16]",
                    "http://[1:2:3:4:5:6:192.0.2.16]",
                    "http://[FEDC:BA98::7654:3210]",
                ]
            ),
            NONE
        );
    }

    #[test]
    fn a_host_that_is_a_future_ip_literal_breaks_the_rule() {
        assert_eq!(
            accepted_by(url(), &["http://[v1.fe80::a+en1]", "http://[V7.x]", "http://[v1.x]"]),
            NONE
        );
    }

    #[test]
    fn a_host_in_brackets_that_is_no_ip_literal_breaks_the_rule() {
        assert_eq!(
            accepted_by(
                url(),
                &[
                    "http://[]",
                    "http://[1:2:3:4:5:6:7:8:9]",
                    "http://[1:2:3:4:5:6:7]",
                    "http://[1::2::3]",
                    "http://[:::]",
                    "http://[12345::]",
                    "http://[::g]",
                    "http://[::192.0.2.256]",
                    "http://[::192.0.2.016]",
                    "http://[192.0.2.16::]",
                    "http://[1:2:3:4:5:6:7:192.0.2.16]",
                    "http://[::1%25eth0]",
                    "http://[v1.]",
                    "http://[v.x]",
                    "http://[vg.x]",
                    "http://[::1",
                    "http://::1]",
                ]
            ),
            NONE
        );
    }

    #[test]
    fn a_registered_name_is_any_text_of_the_characters_it_allows() {
        assert_eq!(
            refused_by(
                url(),
                &[
                    "http://my_host.example",
                    "http://example.com.",
                    "http://a!$&'()*+,;=b",
                    "http://xn--a.example",
                    "http://1.2.3.example",
                    "http://example.123a",
                    "http://example.0x1g",
                ]
            ),
            NONE
        );
    }

    #[test]
    fn a_host_ending_in_a_number_must_be_an_ipv4_address() {
        assert_eq!(
            accepted_by(
                url(),
                &[
                    "http://256.0.0.1",
                    "http://2130706433",
                    "http://1.2.3",
                    "http://1.2.3.4.5",
                    "http://0x7f.0.0.1",
                    "http://010.0.0.1",
                    "http://example.123",
                    "http://0x7f000001",
                    "http://example.0X1f",
                    "http://example.0x",
                ]
            ),
            NONE
        );
    }

    #[test]
    fn a_host_ending_in_a_number_and_one_dot_must_be_an_ipv4_address() {
        assert_eq!(
            [url().check("http://192.0.2.16."), url().check("http://256.0.0.1.")],
            [Ok(()), not_a_url()]
        );
    }

    #[test]
    fn a_host_ending_in_a_number_and_two_dots_is_a_registered_name() {
        assert_eq!(url().check("http://256.0.0.1.."), Ok(()));
    }

    #[test]
    fn a_host_with_percent_encoding_breaks_the_rule() {
        assert_eq!(
            accepted_by(
                url(),
                &[
                    "http://ex%41mple.com",
                    "http://r%C3%A9sum%C3%A9.example.org",
                    "http://%31%32%37.0.0.1",
                ]
            ),
            NONE
        );
    }

    #[test]
    fn a_port_is_a_number_up_to_65535() {
        assert_eq!(
            refused_by(
                url(),
                &[
                    "http://example.com:",
                    "http://example.com:0",
                    "http://example.com:65535/a",
                    "http://example.com:00080",
                ]
            ),
            NONE
        );
    }

    #[test]
    fn a_port_above_65535_breaks_the_rule() {
        assert_eq!(
            accepted_by(
                url(),
                &[
                    "http://example.com:65536",
                    "http://example.com:99999/a",
                    "http://example.com:4294967376",
                ]
            ),
            NONE
        );
    }

    #[test]
    fn a_port_with_anything_but_digits_breaks_the_rule() {
        assert_eq!(
            accepted_by(
                url(),
                &[
                    "http://example.com:8a",
                    "http://example.com:-1",
                    "http://example.com:80:80"
                ]
            ),
            NONE
        );
    }

    #[test]
    fn a_url_without_a_host_breaks_the_rule() {
        assert_eq!(
            accepted_by(url(), &["http:///projects", "http://:80", "http://?q", "http://#top"]),
            NONE
        );
    }

    #[test]
    fn a_url_with_userinfo_breaks_the_rule() {
        assert_eq!(
            accepted_by(
                url(),
                &[
                    "https://alex@example.com",
                    "https://@example.com",
                    "https://:@example.com"
                ]
            ),
            NONE
        );
    }

    #[test]
    fn a_percent_sign_without_two_hexadecimal_digits_breaks_the_rule() {
        assert_eq!(
            accepted_by(
                url(),
                &[
                    "https://example.com/%",
                    "https://example.com/%4",
                    "https://example.com/%zz",
                    "https://example.com/?q=%g1",
                    "https://example.com/#%2",
                    "https://exa%mple.com",
                ]
            ),
            NONE
        );
    }

    #[test]
    fn a_character_outside_the_grammar_breaks_the_rule() {
        assert_eq!(
            accepted_by(
                url(),
                &[
                    "https://example.com ",
                    "https://example.com/a\tb",
                    "https://example.com/a\nb",
                    "https://example.com\\projects",
                    "https://example.com/\"a\"",
                    "https://example.com/<a>",
                    "https://example.com/{a}",
                    "https://example.com/a^b",
                    "https://example.com/a`b",
                    "https://example.com/#a#b",
                    "https://example.com/\u{7F}",
                    "https://example.com/\u{85}",
                    "https://example.com/\u{FDD0}",
                    "https://example.com/\u{FFFE}",
                    "https://example.com/\u{E0001}",
                    "https://exa mple.com",
                ]
            ),
            NONE
        );
    }

    #[test]
    fn a_whitespace_character_breaks_the_rule_even_where_rfc_3987_allows_it() {
        assert_eq!(
            accepted_by(
                url(),
                &[
                    "https://example.com/a\u{A0}b",
                    "https://example.com/a\u{1680}b",
                    "https://example.com/a\u{2000}b",
                    "https://example.com/a\u{200A}b",
                    "https://example.com/a\u{2028}b",
                    "https://example.com/a\u{2029}b",
                    "https://example.com/a\u{202F}b",
                    "https://example.com/a\u{205F}b",
                    "https://example.com/a\u{3000}b",
                ]
            ),
            NONE
        );
    }

    #[test]
    fn an_invisible_character_breaks_the_rule_even_where_rfc_3987_allows_it() {
        assert_eq!(
            accepted_by(
                url(),
                &[
                    "https://example.com/a\u{AD}b",
                    "https://example.com/a\u{34F}b",
                    "https://example.com/a\u{61C}b",
                    "https://example.com/a\u{115F}b",
                    "https://example.com/a\u{1160}b",
                    "https://example.com/a\u{17B4}b",
                    "https://example.com/a\u{17B5}b",
                    "https://example.com/a\u{180B}b",
                    "https://example.com/a\u{180E}b",
                    "https://example.com/a\u{180F}b",
                    "https://example.com/a\u{200B}b",
                    "https://example.com/a\u{200F}b",
                    "https://example.com/a\u{202A}b",
                    "https://example.com/a\u{202E}b",
                    "https://example.com/a\u{2060}b",
                    "https://example.com/a\u{2064}b",
                    "https://example.com/a\u{2065}b",
                    "https://example.com/a\u{2066}b",
                    "https://example.com/a\u{206F}b",
                    "https://example.com/a\u{3164}b",
                    "https://example.com/a\u{FE00}b",
                    "https://example.com/a\u{FE0F}b",
                    "https://example.com/a\u{FEFF}b",
                    "https://example.com/a\u{FFA0}b",
                    "https://example.com/a\u{1BCA0}b",
                    "https://example.com/a\u{1BCA3}b",
                    "https://example.com/a\u{1D173}b",
                    "https://example.com/a\u{1D17A}b",
                ]
            ),
            NONE
        );
    }

    #[test]
    fn a_whitespace_or_invisible_character_breaks_the_rule_anywhere() {
        assert_eq!(
            accepted_by(
                url(),
                &[
                    "https://exa\u{200B}mple.com",
                    "https://example.com\u{A0}",
                    "https://example.com/?q=\u{2028}",
                    "https://example.com/#\u{202E}",
                ]
            ),
            NONE
        );
    }

    #[test]
    fn a_character_beside_the_whitespace_and_invisible_ones_follows_the_rule() {
        assert_eq!(
            refused_by(
                url(),
                &[
                    "https://example.com/a\u{A1}\u{AC}\u{AE}b",
                    "https://example.com/a\u{34E}\u{350}b",
                    "https://example.com/a\u{61B}\u{61D}b",
                    "https://example.com/a\u{115E}\u{1161}b",
                    "https://example.com/a\u{167F}\u{1681}b",
                    "https://example.com/a\u{17B3}\u{17B6}b",
                    "https://example.com/a\u{180A}\u{1810}b",
                    "https://example.com/a\u{1FFF}\u{2010}b",
                    "https://example.com/a\u{2027}\u{2030}b",
                    "https://example.com/a\u{205E}\u{2070}b",
                    "https://example.com/a\u{2FFF}\u{3001}b",
                    "https://example.com/a\u{3163}\u{3165}b",
                    "https://example.com/a\u{FDFF}\u{FE10}b",
                    "https://example.com/a\u{FEFE}\u{FF00}b",
                    "https://example.com/a\u{FF9F}\u{FFA1}b",
                    "https://example.com/a\u{1BC9F}\u{1BCA4}b",
                    "https://example.com/a\u{1D172}\u{1D17B}b",
                ]
            ),
            NONE
        );
    }

    #[test]
    fn a_query_or_a_fragment_takes_square_brackets_and_a_vertical_bar() {
        assert_eq!(
            refused_by(
                url(),
                &[
                    "https://fonts.example/css?family=A|B",
                    "https://api.example/items?filter[x]=1",
                    "https://example.com/#a|b",
                    "https://example.com/#[x]",
                ]
            ),
            NONE
        );
    }

    #[test]
    fn square_brackets_or_a_vertical_bar_in_a_path_or_a_host_break_the_rule() {
        assert_eq!(
            accepted_by(
                url(),
                &[
                    "https://example.com/a|b",
                    "https://example.com/[a]",
                    "https://exa|mple.com",
                    "https://example.com]",
                ]
            ),
            NONE
        );
    }

    #[test]
    fn a_private_use_character_is_allowed_only_in_the_query() {
        assert_eq!(
            [
                url().check("https://example.com/?q=\u{E000}"),
                url().check("https://example.com/\u{E000}"),
                url().check("https://example.com/#\u{E000}"),
                url().check("https://\u{E000}.example"),
            ],
            [Ok(()), not_a_url(), not_a_url(), not_a_url()]
        );
    }

    #[test]
    fn a_scheme_not_starting_with_a_letter_breaks_the_rule() {
        assert_eq!(
            accepted_by(
                url().schemes(&["1http", "+http", "h"]),
                &["1http://example.com", "+http://example.com", "://example.com"]
            ),
            NONE
        );
    }

    #[test]
    fn a_scheme_of_letters_digits_plus_minus_and_dots_follows_the_rule() {
        assert_eq!(url().schemes(&["web+a.b-1"]).check("web+a.b-1://example.com"), Ok(()));
    }
}
