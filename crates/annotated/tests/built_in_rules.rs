//! The built-in rules, written in `#[validate(...)]`.

mod support;

use annotated::{Problem, Validate};
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

fn not_reserved(slug: &str) -> Result<(), Problem> {
    if slug == "admin" {
        return Err(Problem::custom("reserved_slug", "is reserved"));
    }
    Ok(())
}

#[derive(Validate)]
struct NewProduct {
    #[validate(prefix = "SKU-")]
    sku: String,
    #[validate(suffix(".pdf"))]
    manual: String,
    #[validate(contains("@", message = "must be an address"))]
    support: String,
    #[validate(custom(not_reserved))]
    slug: String,
}

fn new_product() -> NewProduct {
    NewProduct {
        sku: "SKU-1042".to_owned(),
        manual: "chair.pdf".to_owned(),
        support: "help@example.com".to_owned(),
        slug: "garden-chair".to_owned(),
    }
}

#[test]
fn prefix_checks_that_a_field_starts_with_its_value() {
    let product = NewProduct {
        sku: "1042".to_owned(),
        ..new_product()
    };

    assert_eq!(
        product.validate().unwrap_err().to_string(),
        r#"sku: must start with "SKU-""#
    );
}

#[test]
fn suffix_checks_that_a_field_ends_with_its_value() {
    let product = NewProduct {
        manual: "chair.doc".to_owned(),
        ..new_product()
    };

    assert_eq!(violations_of(&product), ["manual: suffix"]);
}

#[test]
fn contains_checks_that_a_field_contains_its_value() {
    let product = NewProduct {
        support: "help".to_owned(),
        ..new_product()
    };

    assert_eq!(
        product.validate().unwrap_err().to_string(),
        "support: must be an address"
    );
}

#[test]
fn custom_checks_a_field_with_your_function() {
    let product = NewProduct {
        slug: "admin".to_owned(),
        ..new_product()
    };

    assert_eq!(violations_of(&product), ["slug: reserved_slug"]);
}

#[cfg(feature = "regex")]
#[derive(Validate)]
struct Address {
    #[validate(pattern = r"^\d{5}$")]
    postal_code: String,
    #[validate(pattern(r"^[A-Z]{2}$", message = "must be a two-letter country code"))]
    country: String,
}

#[cfg(feature = "regex")]
fn address() -> Address {
    Address {
        postal_code: "28013".to_owned(),
        country: "ES".to_owned(),
    }
}

#[cfg(feature = "regex")]
#[test]
fn pattern_checks_that_a_field_matches_its_regular_expression() {
    let address = Address {
        postal_code: "2801".to_owned(),
        ..address()
    };

    assert_eq!(
        address.validate().unwrap_err().to_string(),
        r"postal_code: must match the pattern ^\d{5}$"
    );
}

#[cfg(feature = "regex")]
#[test]
fn pattern_takes_its_regular_expression_in_parentheses_too() {
    let address = Address {
        country: "Spain".to_owned(),
        ..address()
    };

    assert_eq!(
        address.validate().unwrap_err().to_string(),
        "country: must be a two-letter country code"
    );
}
