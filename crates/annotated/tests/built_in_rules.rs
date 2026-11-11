//! The built-in rules, written in `#[validate(...)]`.

mod support;

use annotated::Validate;
use support::violations_of;

#[derive(Validate)]
struct NewMember {
    #[validate(range(min = 18, max = 120))]
    age: u8,
    #[validate(range(min = 0.0))]
    balance: Option<f64>,
}

fn new_member() -> NewMember {
    NewMember {
        age: 34,
        balance: Some(10.5),
    }
}

#[test]
fn a_range_takes_its_bounds_type_from_the_field() {
    let member = NewMember {
        age: 12,
        ..new_member()
    };

    assert_eq!(
        member.validate().unwrap_err().to_string(),
        "age: must be between 18 and 120"
    );
}

#[test]
fn a_range_on_an_option_checks_the_number_inside() {
    let member = NewMember {
        balance: Some(-0.5),
        ..new_member()
    };

    assert_eq!(violations_of(&member), ["balance: range"]);
}

#[derive(Validate)]
struct Contact {
    #[validate(email)]
    email: String,
    #[validate(ip)]
    server: Option<String>,
}

fn contact() -> Contact {
    Contact {
        email: "alex@example.com".to_owned(),
        server: None,
    }
}

#[test]
fn email_checks_that_a_field_is_an_email_address() {
    let contact = Contact {
        email: "alex.example.com".to_owned(),
        ..contact()
    };

    assert_eq!(violations_of(&contact), ["email: email"]);
}

#[test]
fn ip_checks_that_a_field_is_an_ip_address() {
    let contact = Contact {
        server: Some("localhost".to_owned()),
        ..contact()
    };

    assert_eq!(violations_of(&contact), ["server: ip"]);
}

#[cfg(feature = "url")]
#[derive(Validate)]
struct Release {
    #[validate(url(schemes = &["ftp", "sftp"]))]
    download: String,
}

#[cfg(feature = "url")]
#[test]
fn url_takes_the_schemes_it_accepts_as_a_named_argument() {
    let release = Release {
        download: "https://example.com/release.tar.gz".to_owned(),
    };

    assert_eq!(
        release.validate().unwrap_err().to_string(),
        "download: must be a URL using ftp or sftp"
    );
}
