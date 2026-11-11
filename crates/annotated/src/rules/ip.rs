use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

use crate::{Problem, Rule};

/// The rule that a string is an IP address, v4 or v6, or only one of them with [`Ip::v4`] or [`Ip::v6`].
///
/// It checks anything that is `AsRef<str>`, in the forms `std::net` parses: `192.0.2.1`, `2001:db8::1`.
///
/// ```
/// use annotated::{Rule, rules};
///
/// assert_eq!(rules::ip().check("2001:db8::1"), Ok(()));
/// assert_eq!(rules::ip().v4().check("2001:db8::1").unwrap_err().to_string(), "must be an IPv4 address");
/// ```
#[must_use]
pub const fn ip() -> Ip {
    Ip { version: None }
}

/// Checks that a string is an IP address; made by [`ip`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ip {
    version: Option<u8>,
}

impl Ip {
    /// The rule accepting only IPv4 addresses.
    #[must_use]
    pub const fn v4(self) -> Self {
        Self { version: Some(4) }
    }

    /// The rule accepting only IPv6 addresses.
    #[must_use]
    pub const fn v6(self) -> Self {
        Self { version: Some(6) }
    }
}

#[diagnostic::do_not_recommend]
impl<T: AsRef<str> + ?Sized> Rule<T> for Ip {
    fn check(&self, value: &T) -> Result<(), Problem> {
        let text = value.as_ref();
        let is_ip_address = match self.version {
            Some(4) => text.parse::<Ipv4Addr>().is_ok(),
            Some(6) => text.parse::<Ipv6Addr>().is_ok(),
            _ => text.parse::<IpAddr>().is_ok(),
        };
        if is_ip_address {
            return Ok(());
        }
        Err(Problem::Ip { version: self.version })
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
        assert_eq!(ip().check("192.0.2"), Err(Problem::Ip { version: None }));
    }

    #[test]
    fn an_ipv4_address_follows_the_rule_for_v4() {
        assert_eq!(ip().v4().check("192.0.2.1"), Ok(()));
    }

    #[test]
    fn an_ipv6_address_breaks_the_rule_for_v4() {
        assert_eq!(ip().v4().check("2001:db8::1"), Err(Problem::Ip { version: Some(4) }));
    }

    #[test]
    fn an_ipv6_address_follows_the_rule_for_v6() {
        assert_eq!(ip().v6().check("2001:db8::1"), Ok(()));
    }

    #[test]
    fn an_ipv4_address_breaks_the_rule_for_v6() {
        assert_eq!(ip().v6().check("192.0.2.1"), Err(Problem::Ip { version: Some(6) }));
    }
}
