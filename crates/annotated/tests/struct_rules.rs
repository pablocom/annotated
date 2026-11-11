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
    #[validate(required)]
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

#[test]
fn a_struct_level_rule_at_a_field_is_skipped_when_the_field_breaks_its_own_rules() {
    let new_password = NewPassword {
        repeated_password: String::new(),
        ..new_password()
    };

    assert_eq!(violations_of(&new_password), ["repeatedPassword: required"]);
}

fn account_passwords_match(new_account: &NewAccount) -> Result<(), Problem> {
    if new_account.repeated_password != new_account.password {
        return Err(Problem::custom("passwords_differ", "must match the password"));
    }
    Ok(())
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
#[validate(custom(account_passwords_match, field = repeated_password))]
struct NewAccount {
    password: String,
    #[validate(required, length(min = 8))]
    repeated_password: String,
}

#[test]
fn a_struct_level_rule_at_a_field_is_skipped_when_the_field_breaks_a_later_rule_of_its_own() {
    let new_account = NewAccount {
        password: "correct horse battery staple".to_owned(),
        repeated_password: "horse".to_owned(),
    };

    assert_eq!(violations_of(&new_account), ["repeatedPassword: length"]);
}

fn last_night_follows_the_first(stay: &Stay) -> Result<(), Problem> {
    if stay.last_night < stay.first_night {
        return Err(Problem::custom("ends_before_it_starts", "must end after it starts"));
    }
    Ok(())
}

#[derive(Validate)]
#[validate(custom(last_night_follows_the_first))]
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

fn second_stay_follows_the_first(trip: &Trip) -> Result<(), Problem> {
    if trip.second_stay.first_night <= trip.first_stay.last_night {
        return Err(Problem::custom("stays_overlap", "must start after the first stay ends"));
    }
    Ok(())
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
#[validate(custom(second_stay_follows_the_first, field = second_stay))]
struct Trip {
    #[validate(nested)]
    first_stay: Stay,
    #[validate(nested)]
    second_stay: Stay,
}

#[test]
fn a_struct_level_rule_at_a_field_is_skipped_when_violations_are_found_under_it() {
    let trip = Trip {
        first_stay: stay(),
        second_stay: stay_ending_before_it_starts(),
    };

    assert_eq!(violations_of(&trip), ["secondStay: ends_before_it_starts"]);
}

fn ends_after_it_starts(starts_at: u16, ends_at: u16) -> Result<(), Problem> {
    if ends_at <= starts_at {
        return Err(Problem::custom("ends_before_it_starts", "must end after it starts"));
    }
    Ok(())
}

fn lasts_an_hour_at_least(starts_at: u16, ends_at: u16) -> Result<(), Problem> {
    if ends_at < starts_at + 100 {
        return Err(Problem::custom("too_short", "must last an hour at least"));
    }
    Ok(())
}

fn doors_open_before_it_starts(gig: &Gig) -> Result<(), Problem> {
    if gig.doors_open > gig.starts_at {
        return Err(Problem::custom("doors_open_too_late", "must open by the start"));
    }
    Ok(())
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
#[validate(custom(|gig| ends_after_it_starts(gig.starts_at, gig.ends_at), field = ends_at))]
#[validate(custom(|gig| lasts_an_hour_at_least(gig.starts_at, gig.ends_at), field = ends_at))]
#[validate(custom(
    doors_open_before_it_starts,
    field = doors_open,
    message = "doors must open by the start"
))]
struct Gig {
    #[serde(rename = "doors")]
    doors_open: u16,
    starts_at: u16,
    ends_at: u16,
}

fn gig() -> Gig {
    Gig {
        doors_open: 1900,
        starts_at: 2000,
        ends_at: 2200,
    }
}

#[test]
fn every_struct_level_rule_is_checked_in_the_order_written() {
    let gig = Gig {
        doors_open: 2030,
        ends_at: 2030,
        ..gig()
    };

    assert_eq!(violations_of(&gig), ["endsAt: too_short", "doors: doors_open_too_late"]);
}

#[test]
fn the_first_struct_level_rule_a_field_breaks_stops_its_later_ones() {
    let gig = Gig { ends_at: 1930, ..gig() };

    assert_eq!(violations_of(&gig), ["endsAt: ends_before_it_starts"]);
}

#[test]
fn a_message_replaces_the_problem_s_own_on_a_struct_level_rule() {
    let gig = Gig {
        doors_open: 2030,
        ..gig()
    };

    assert_eq!(
        gig.validate().unwrap_err().to_string(),
        "doors: doors must open by the start"
    );
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
#[validate(report_all, custom(|festival| ends_after_it_starts(festival.starts_at, festival.ends_at), field = ends_at))]
#[validate(custom(|festival| lasts_an_hour_at_least(festival.starts_at, festival.ends_at), field = ends_at))]
struct Festival {
    #[validate(required, length(min = 3))]
    title: String,
    starts_at: u16,
    #[validate(range(min = 1200))]
    ends_at: u16,
}

fn festival() -> Festival {
    Festival {
        title: "Summer strings".to_owned(),
        starts_at: 1400,
        ends_at: 2300,
    }
}

#[test]
fn report_all_beside_a_struct_level_rule_still_reports_every_rule_a_field_breaks() {
    let festival = Festival {
        title: String::new(),
        ..festival()
    };

    assert_eq!(violations_of(&festival), ["title: required", "title: length"]);
}

#[test]
fn report_all_checks_every_struct_level_rule_at_a_field_whatever_the_field_broke() {
    let festival = Festival {
        ends_at: 1130,
        ..festival()
    };

    assert_eq!(
        violations_of(&festival),
        ["endsAt: range", "endsAt: ends_before_it_starts", "endsAt: too_short"]
    );
}

#[derive(Validate)]
#[validate(custom(|limits| {
    if limits.youngest > limits.oldest {
        return Err(Problem::custom("youngest_above_oldest", "must not have its youngest age above its oldest"));
    }
    Ok(())
}))]
struct AgeLimits {
    #[validate(range(max = 120))]
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

#[test]
fn a_struct_level_rule_at_the_struct_itself_runs_whatever_the_fields_broke() {
    let limits = AgeLimits {
        youngest: 130,
        oldest: 18,
    };

    assert_eq!(violations_of(&limits), ["youngest: range", "youngest_above_oldest"]);
}
