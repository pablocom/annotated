//! Rules on `Option` fields: `required` checks the `Option`, every other rule the value inside.

mod support;

use annotated::Validate;
use support::violations_of;

#[derive(Validate)]
struct Profile {
    #[validate(required)]
    nickname: Option<String>,
    #[validate(length(min = 3))]
    bio: Option<String>,
    #[validate(required, length(max = 10))]
    city: Option<String>,
    #[validate(length(min = 3))]
    website: std::option::Option<String>,
}

fn profile() -> Profile {
    Profile {
        nickname: Some("Al".to_owned()),
        bio: None,
        city: Some("Lisbon".to_owned()),
        website: None,
    }
}

#[test]
fn required_on_an_option_is_broken_by_none() {
    let profile = Profile {
        nickname: None,
        ..profile()
    };

    assert_eq!(violations_of(&profile), ["nickname: required"]);
}

#[test]
fn required_on_an_option_accepts_some_whatever_it_holds() {
    let profile = Profile {
        nickname: Some(String::new()),
        ..profile()
    };

    assert_eq!(profile.validate(), Ok(()));
}

#[test]
fn a_rule_on_an_option_checks_the_value_inside() {
    let profile = Profile {
        bio: Some("Hi".to_owned()),
        ..profile()
    };

    assert_eq!(violations_of(&profile), ["bio: length"]);
}

#[test]
fn a_rule_on_an_option_is_skipped_when_there_is_none() {
    let profile = Profile { bio: None, ..profile() };

    assert_eq!(profile.validate(), Ok(()));
}

#[test]
fn a_rule_after_required_on_an_option_checks_the_value_inside() {
    let profile = Profile {
        city: Some("Rio de Janeiro".to_owned()),
        ..profile()
    };

    assert_eq!(violations_of(&profile), ["city: length"]);
}

#[test]
fn an_option_written_with_its_full_path_is_an_option_too() {
    let profile = Profile {
        website: Some("ab".to_owned()),
        ..profile()
    };

    assert_eq!(violations_of(&profile), ["website: length"]);
}
