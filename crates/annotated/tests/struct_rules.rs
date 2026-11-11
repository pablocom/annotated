//! Rules on a whole struct, checked after its fields.

mod support;

use annotated::{Problem, Validate};
use support::violations_of;

fn passwords_match(new_password: &NewPassword) -> Result<(), Problem> {
    if new_password.repeated_password != new_password.password {
        return Err(Problem::custom("passwords_differ", "must match the password"));
    }
    Ok(())
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
#[validate(custom(passwords_match, field = repeated_password))]
struct NewPassword {
    #[validate(length(min = 8))]
    password: String,
    repeated_password: String,
}

fn new_password() -> NewPassword {
    NewPassword {
        password: "correct horse battery staple".to_owned(),
        repeated_password: "correct horse battery staple".to_owned(),
    }
}

#[test]
fn a_struct_following_its_struct_level_rule_has_no_violations() {
    assert_eq!(new_password().validate(), Ok(()));
}

#[test]
fn a_struct_level_rule_naming_a_field_reports_at_it_by_its_serde_name() {
    let new_password = NewPassword {
        repeated_password: "correct horse".to_owned(),
        ..new_password()
    };

    assert_eq!(violations_of(&new_password), ["repeatedPassword: passwords_differ"]);
}

#[test]
fn struct_level_rules_are_checked_after_every_field() {
    let new_password = NewPassword {
        password: "horse".to_owned(),
        repeated_password: "battery".to_owned(),
    };

    assert_eq!(
        violations_of(&new_password),
        ["password: length", "repeatedPassword: passwords_differ"]
    );
}

fn ends_after_it_starts(stay: &Stay) -> Result<(), Problem> {
    if stay.last_night < stay.first_night {
        return Err(Problem::custom("ends_before_it_starts", "must end after it starts"));
    }
    Ok(())
}

#[derive(Validate)]
#[validate(custom(ends_after_it_starts))]
struct Stay {
    first_night: u16,
    last_night: u16,
}

fn stay() -> Stay {
    Stay {
        first_night: 120,
        last_night: 122,
    }
}

fn stay_ending_before_it_starts() -> Stay {
    Stay {
        first_night: 122,
        last_night: 120,
    }
}

#[test]
fn a_struct_level_rule_reports_at_the_struct_itself() {
    let violations = stay_ending_before_it_starts().validate().unwrap_err();

    assert_eq!(violations.to_string(), "must end after it starts");
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
struct Booking {
    #[validate(nested)]
    stay: Stay,
    #[validate(each(nested))]
    extra_stays: Vec<Stay>,
    #[validate(nested)]
    guest_password: NewPassword,
}

fn booking() -> Booking {
    Booking {
        stay: stay(),
        extra_stays: Vec::new(),
        guest_password: new_password(),
    }
}

#[test]
fn a_struct_level_rule_of_a_nested_struct_reports_at_its_field() {
    let booking = Booking {
        stay: stay_ending_before_it_starts(),
        ..booking()
    };

    assert_eq!(violations_of(&booking), ["stay: ends_before_it_starts"]);
}

#[test]
fn a_struct_level_rule_of_every_element_reports_at_its_index() {
    let booking = Booking {
        extra_stays: vec![stay(), stay_ending_before_it_starts()],
        ..booking()
    };

    assert_eq!(violations_of(&booking), ["extraStays[1]: ends_before_it_starts"]);
}

#[test]
fn a_struct_level_rule_of_a_nested_struct_naming_a_field_reports_under_the_nested_field() {
    let booking = Booking {
        guest_password: NewPassword {
            repeated_password: "correct horse".to_owned(),
            ..new_password()
        },
        ..booking()
    };

    assert_eq!(
        violations_of(&booking),
        ["guestPassword.repeatedPassword: passwords_differ"]
    );
}

fn ends_after_it_starts_playing(concert: &Concert) -> Result<(), Problem> {
    if concert.ends_at <= concert.starts_at {
        return Err(Problem::custom("ends_before_it_starts", "must end after it starts"));
    }
    Ok(())
}

fn doors_open_before_it_starts(concert: &Concert) -> Result<(), Problem> {
    if concert.doors_open_at > concert.starts_at {
        return Err(Problem::custom("doors_open_too_late", "must open by the start"));
    }
    Ok(())
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
#[validate(report_all, custom(ends_after_it_starts_playing, field = ends_at))]
#[validate(custom(
    doors_open_before_it_starts,
    field = doors_open_at,
    message = "doors must open by the start"
))]
struct Concert {
    #[validate(required, length(min = 3))]
    title: String,
    #[serde(rename = "doors")]
    doors_open_at: u16,
    starts_at: u16,
    ends_at: u16,
}

fn concert() -> Concert {
    Concert {
        title: "Night of strings".to_owned(),
        doors_open_at: 1900,
        starts_at: 2000,
        ends_at: 2200,
    }
}

#[test]
fn every_struct_level_rule_is_checked_in_the_order_written() {
    let concert = Concert {
        doors_open_at: 2030,
        ends_at: 1930,
        ..concert()
    };

    assert_eq!(
        violations_of(&concert),
        ["endsAt: ends_before_it_starts", "doors: doors_open_too_late"]
    );
}

#[test]
fn a_message_replaces_the_problem_s_own_on_a_struct_level_rule() {
    let concert = Concert {
        doors_open_at: 2030,
        ..concert()
    };

    assert_eq!(
        concert.validate().unwrap_err().to_string(),
        "doors: doors must open by the start"
    );
}

#[test]
fn report_all_beside_a_struct_level_rule_still_reports_every_rule_a_field_breaks() {
    let concert = Concert {
        title: String::new(),
        ..concert()
    };

    assert_eq!(violations_of(&concert), ["title: required", "title: length"]);
}

#[derive(Validate)]
#[validate(custom(|limits| {
    if limits.youngest > limits.oldest {
        return Err(Problem::custom("youngest_above_oldest", "must not have its youngest age above its oldest"));
    }
    Ok(())
}))]
struct AgeLimits {
    youngest: u8,
    oldest: u8,
}

#[test]
fn a_struct_level_rule_takes_a_closure() {
    let limits = AgeLimits {
        youngest: 65,
        oldest: 18,
    };

    assert_eq!(violations_of(&limits), ["youngest_above_oldest"]);
}
