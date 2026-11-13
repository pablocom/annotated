use crate::{Problem, Rule};

/// The rule that a string is an absolute URL with a host, whose scheme is `http`, `https` or `ftp`, or one of those
/// [`Url::schemes`] sets.
///
/// A URL here is an absolute URI with an authority, as [RFC 3986](https://www.rfc-editor.org/rfc/rfc3986#appendix-A)
/// defines it in section 3 and appendix A: `scheme "://" authority path-abempty [ "?" query ] [ "#" fragment ]`.
///
/// - [RFC 3987](https://www.rfc-editor.org/rfc/rfc3987#section-2.2) adds international characters: `ucschar` in the
///   host, path, query and fragment, and `iprivate` in the query. So `https://bücher.de/straße` is a URL.
/// - The rule departs from that grammar twice. The host can't be empty, as
///   [RFC 9110 section 4.2.1](https://www.rfc-editor.org/rfc/rfc9110#section-4.2.1) requires of `http`. And a URL with
///   userinfo, which [RFC 9110 section 4.2.4](https://www.rfc-editor.org/rfc/rfc9110#section-4.2.4) deprecates, breaks
///   the rule whatever its scheme.
/// - The scheme is compared with the accepted ones ignoring ASCII case.
/// - The host is an IP literal in brackets (an IPv6 address or an `IPvFuture`), an IPv4 address, or a registered name.
///   The port is any run of digits, and can be empty.
/// - A `%` must be followed by two hexadecimal digits.
/// - Anything the grammar doesn't allow breaks the rule: whitespace anywhere, including around the URL, a backslash, a
///   control character.
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
    Some(after_host.strip_prefix(':').map_or(after_host, port))
}

/// `ihost = IP-literal / IPv4address / ireg-name`, not empty.
fn host(text: &str) -> Option<&str> {
    if let Some(after_bracket) = text.strip_prefix('[') {
        return ip_literal(after_bracket);
    }
    // Every `IPv4address` is an `ireg-name` too, so the `ireg-name` alone tells which hosts are accepted.
    let after_name = ireg_name(text);
    (after_name.len() < text.len()).then_some(after_name)
}

/// `IP-literal = "[" ( IPv6address / IPvFuture ) "]"`, after its `[`.
fn ip_literal(text: &str) -> Option<&str> {
    let (address, rest) = text.split_once(']')?;
    (ipv6_address(address) || ipv_future(address)).then_some(rest)
}

/// `IPvFuture = "v" 1*HEXDIG "." 1*( unreserved / sub-delims / ":" )`.
fn ipv_future(text: &str) -> bool {
    let Some((version, address)) = text.strip_prefix(['v', 'V']).and_then(|rest| rest.split_once('.')) else {
        return false;
    };
    let is_version = !version.is_empty() && version.chars().all(|character| character.is_ascii_hexdigit());
    let is_address = !address.is_empty()
        && address
            .chars()
            .all(|character| is_unreserved(character) || is_sub_delim(character) || character == ':');
    is_version && is_address
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

/// `ireg-name = *( iunreserved / pct-encoded / sub-delims )`, and what follows it.
fn ireg_name(text: &str) -> &str {
    after_run_of(text, |character| is_iunreserved(character) || is_sub_delim(character))
}

/// `port = *DIGIT`, and what follows it.
fn port(text: &str) -> &str {
    text.trim_start_matches(|character: char| character.is_ascii_digit())
}

/// `ipath-abempty = *( "/" isegment )`, and what follows it.
fn path_abempty(text: &str) -> &str {
    if !text.starts_with('/') {
        return text;
    }
    after_run_of(text, |character| character == '/' || is_ipchar(character))
}

/// `iquery = *( ipchar / iprivate / "/" / "?" )`, and what follows it.
fn iquery(text: &str) -> &str {
    after_run_of(text, |character| {
        is_ipchar(character) || is_iprivate(character) || matches!(character, '/' | '?')
    })
}

/// `ifragment = *( ipchar / "/" / "?" )`, and what follows it.
fn ifragment(text: &str) -> &str {
    after_run_of(text, |character| is_ipchar(character) || matches!(character, '/' | '?'))
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

/// `iunreserved = ALPHA / DIGIT / "-" / "." / "_" / "~" / ucschar`.
fn is_iunreserved(character: char) -> bool {
    is_unreserved(character) || is_ucschar(character)
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
                    "http://r%C3%A9sum%C3%A9.example.org",
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
    fn a_host_can_be_an_ip_address_or_a_future_ip_literal() {
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
                    "http://[v1.fe80::a+en1]",
                    "http://[V7.x]",
                ]
            ),
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
                    "http://256.0.0.1",
                    "http://1.2.3",
                    "http://my_host.example",
                    "http://example.com.",
                    "http://ex%41mple.com",
                    "http://a!$&'()*+,;=b",
                    "http://xn--a.example",
                ]
            ),
            NONE
        );
    }

    #[test]
    fn a_port_is_any_run_of_digits() {
        assert_eq!(
            refused_by(
                url(),
                &[
                    "http://example.com:",
                    "http://example.com:0",
                    "http://example.com:99999/a"
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
                    "https://example.com/a|b",
                    "https://example.com/a^b",
                    "https://example.com/a`b",
                    "https://example.com/[a]",
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
