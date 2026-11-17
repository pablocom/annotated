//! Rules on `Option` fields: every rule but `required` checks the value inside. `required` asks for the field's key,
//! which only deserializing can tell, so it accepts every value that already exists.

mod support;

use annotated::Validate;
use support::issues_of;

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
fn required_on_an_option_accepts_none() {
    let profile = Profile {
        nickname: None,
        ..profile()
    };

    assert_eq!(profile.validate(), Ok(()));
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

    assert_eq!(issues_of(&profile), ["bio: length"]);
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

    assert_eq!(issues_of(&profile), ["city: length"]);
}

#[test]
fn an_option_written_with_its_full_path_is_an_option_too() {
    let profile = Profile {
        website: Some("ab".to_owned()),
        ..profile()
    };

    assert_eq!(issues_of(&profile), ["website: length"]);
}

#[derive(Validate)]
struct Badge {
    #[validate(required, length(min = 1))]
    caption: Option<String>,
}

#[test]
fn an_empty_string_in_a_required_option_passes_required_and_breaks_the_rule_beside_it() {
    let badge = Badge {
        caption: Some(String::new()),
    };

    assert_eq!(issues_of(&badge), ["caption: length"]);
}

#[derive(Validate)]
enum Delivery {
    Courier {
        #[validate(required)]
        tracking: Option<String>,
    },
}

#[test]
fn required_on_an_option_in_a_struct_variant_accepts_none() {
    let delivery = Delivery::Courier { tracking: None };

    assert_eq!(delivery.validate(), Ok(()));
}

macro_rules! with_a_field_of_type {
    ($name:ident, $field_type:ty) => {
        #[derive(Validate)]
        struct $name {
            #[validate(length(min = 3))]
            nickname: $field_type,
        }
    };
}

with_a_field_of_type!(Alias, Option<String>);

#[test]
fn a_rule_on_an_option_passed_to_a_macro_as_a_type_checks_the_value_inside() {
    let alias = Alias {
        nickname: Some("Al".to_owned()),
    };

    assert_eq!(issues_of(&alias), ["nickname: length"]);
}
