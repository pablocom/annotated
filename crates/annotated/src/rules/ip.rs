use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::{Problem, Rule};

/// The rule that a string is an IP address of either version, v4 or v6: `192.0.2.1`, `2001:db8::1`.
///
/// It checks anything that is `AsRef<str>`, in the forms `std::net` parses. [`ipv4`] and [`ipv6`] accept only
/// one version.
///
/// ```
/// use annotated::{Rule, rules};
///
/// assert_eq!(rules::ip().check("2001:db8::1"), Ok(()));
/// assert_eq!(rules::ip().check("192.0.2").unwrap_err().to_string(), "must be an IP address");
/// ```
#[must_use]
pub const fn ip() -> Ip {
    Ip {
        version: IpVersion::Either,
    }
}

/// The rule that a string is an IPv4 address: `192.0.2.1`.
///
/// It checks anything that is `AsRef<str>`, in the form `std::net` parses.
///
/// ```
/// use annotated::{Rule, rules};
///
/// assert_eq!(rules::ipv4().check("192.0.2.1"), Ok(()));
/// assert_eq!(rules::ipv4().check("2001:db8::1").unwrap_err().to_string(), "must be an IPv4 address");
/// ```
#[must_use]
pub const fn ipv4() -> Ip {
    Ip { version: IpVersion::V4 }
}

/// The rule that a string is an IPv6 address: `2001:db8::1`.
///
/// It checks anything that is `AsRef<str>`, in the forms `std::net` parses.
///
/// ```
/// use annotated::{Rule, rules};
///
/// assert_eq!(rules::ipv6().check("2001:db8::1"), Ok(()));
/// assert_eq!(rules::ipv6().check("192.0.2.1").unwrap_err().to_string(), "must be an IPv6 address");
/// ```
#[must_use]
pub const fn ipv6() -> Ip {
    Ip { version: IpVersion::V6 }
}

/// Checks that a string is an IP address; made by [`ip`], [`ipv4`] or [`ipv6`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ip {
    version: IpVersion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IpVersion {
    Either,
    V4,
    V6,
}

impl IpVersion {
    fn is_address(self, text: &str) -> bool {
        match self {
            Self::Either => text.parse::<IpAddr>().is_ok(),
            Self::V4 => text.parse::<Ipv4Addr>().is_ok(),
            Self::V6 => text.parse::<Ipv6Addr>().is_ok(),
        }
    }

    fn problem(self) -> Problem {
        match self {
            Self::Either => Problem::Ip,
            Self::V4 => Problem::Ipv4,
            Self::V6 => Problem::Ipv6,
        }
    }
}

#[diagnostic::do_not_recommend]
impl<T: AsRef<str> + ?Sized> Rule<T> for Ip {
    fn check(&self, value: &T) -> Result<(), Problem> {
        if self.version.is_address(value.as_ref()) {
            return Ok(());
        }
        Err(self.version.problem())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_ipv4_address_follows_the_rule() {
        assert_eq!(ip().check("192.0.2.1"), Ok(()));
    }

    #[test]
    fn an_ipv6_address_follows_the_rule() {
        assert_eq!(ip().check("2001:db8::1"), Ok(()));
    }

    #[test]
    fn a_string_that_is_not_an_ip_address_breaks_the_rule() {
        assert_eq!(ip().check("192.0.2"), Err(Problem::Ip));
    }

    #[test]
    fn an_ipv4_address_follows_the_ipv4_rule() {
        assert_eq!(ipv4().check("192.0.2.1"), Ok(()));
    }

    #[test]
    fn an_ipv6_address_breaks_the_ipv4_rule() {
        assert_eq!(ipv4().check("2001:db8::1"), Err(Problem::Ipv4));
    }

    #[test]
    fn an_ipv6_address_follows_the_ipv6_rule() {
        assert_eq!(ipv6().check("2001:db8::1"), Ok(()));
    }

    #[test]
    fn an_ipv4_address_breaks_the_ipv6_rule() {
        assert_eq!(ipv6().check("192.0.2.1"), Err(Problem::Ipv6));
    }
}
