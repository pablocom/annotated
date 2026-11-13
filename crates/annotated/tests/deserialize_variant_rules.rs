//! `#[derive(Deserialize)]` checks a variant's rules once it has read the variant, as `validate()` checks them.

#![cfg(feature = "serde")]

use annotated::{Deserialize, DeserializeError, Issue, Problem, Validate};

fn deserialize<'de, T: Deserialize<'de>>(json: &'de str) -> Result<T, DeserializeError<serde_json::Error>> {
    annotated::from_deserializer(&mut serde_json::Deserializer::from_str(json))
}

fn issues_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<String> {
    paths_and_codes(deserialize::<T>(json))
}

fn fail_fast_issues_reading<'de, T: Deserialize<'de>>(json: &'de str) -> Vec<String> {
    paths_and_codes(annotated::from_deserializer_fail_fast::<T, _>(
        &mut serde_json::Deserializer::from_str(json),
    ))
}

fn paths_and_codes<T>(outcome: Result<T, DeserializeError<serde_json::Error>>) -> Vec<String> {
    match outcome {
        Ok(_) => Vec::new(),
        Err(DeserializeError::Issues(issues)) => issues.iter().map(path_and_code).collect(),
        Err(error) => vec![format!("the format's error: {error}")],
    }
}

fn path_and_code(issue: &Issue) -> String {
    let code = issue.problem().code();
    if issue.path().is_root() {
        return code.to_owned();
    }
    format!("{}: {code}", issue.path())
}

fn a_card_expires_after_it_starts(payment: &Payment) -> Result<(), Problem> {
    let Payment::Card { valid_from, expires_in } = payment else {
        panic!("checked for a variant the rule isn't written on")
    };
    if expires_in <= valid_from {
        return Err(Problem::custom("expires_too_soon", "must be after the card starts"));
    }
    Ok(())
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
enum Payment {
    #[serde(rename_all = "camelCase")]
    #[validate(custom(a_card_expires_after_it_starts, field = expires_in))]
    Card {
        #[validate(range(min = 2000))]
        valid_from: u16,
        #[validate(range(max = 2100))]
        expires_in: u16,
    },
    BankTransfer {
        #[validate(required)]
        iban: String,
    },
}

#[test]
fn a_variant_following_its_rule_is_read() {
    let body = r#"{ "card": { "validFrom": 2026, "expiresIn": 2030 } }"#;

    assert_eq!(
        deserialize::<Payment>(body).map_err(|error| error.to_string()),
        Ok(Payment::Card {
            valid_from: 2026,
            expires_in: 2030
        })
    );
}

#[test]
fn a_variant_s_rule_is_checked_once_the_variant_is_read() {
    let body = r#"{ "card": { "validFrom": 2030, "expiresIn": 2026 } }"#;

    assert_eq!(issues_reading::<Payment>(body), ["card.expiresIn: expires_too_soon"]);
}

#[test]
fn a_variant_s_rule_is_not_checked_when_another_variant_is_read() {
    let body = r#"{ "bankTransfer": { "iban": "ES9121000418450200051332" } }"#;

    assert_eq!(issues_reading::<Payment>(body), [] as [&str; 0]);
}

#[test]
fn a_variant_s_rules_are_checked_after_its_fields() {
    let body = r#"{ "card": { "validFrom": 1990, "expiresIn": 1980 } }"#;

    assert_eq!(
        issues_reading::<Payment>(body),
        ["card.validFrom: range", "card.expiresIn: expires_too_soon"]
    );
}

#[test]
fn a_variant_missing_a_field_is_not_checked_by_its_rule() {
    let body = r#"{ "card": { "validFrom": 2030 } }"#;

    assert_eq!(issues_reading::<Payment>(body), ["card.expiresIn: missing"]);
}

#[test]
fn a_variant_s_rule_at_a_field_is_skipped_when_the_field_breaks_its_own_rules() {
    let body = r#"{ "card": { "validFrom": 2300, "expiresIn": 2200 } }"#;

    assert_eq!(issues_reading::<Payment>(body), ["card.expiresIn: range"]);
}

#[test]
fn serde_s_deserialize_reports_the_issue_of_a_variant_s_rule() {
    let body = r#"{ "card": { "validFrom": 2030, "expiresIn": 2026 } }"#;

    let error = serde_json::from_str::<Payment>(body).unwrap_err();

    assert_eq!(error.to_string(), "card.expiresIn: must be after the card starts");
}

#[derive(serde::Deserialize, Validate)]
#[serde(rename_all = "camelCase")]
enum PaymentThenValidated {
    #[serde(rename_all = "camelCase")]
    #[validate(custom(
        |&PaymentThenValidated::Card { valid_from, expires_in }| {
            if expires_in <= valid_from {
                return Err(Problem::custom("expires_too_soon", "must be after the card starts"));
            }
            Ok(())
        },
        field = expires_in
    ))]
    Card {
        #[validate(range(min = 2000))]
        valid_from: u16,
        #[validate(range(max = 2100))]
        expires_in: u16,
    },
}

#[test]
fn a_variant_s_rules_are_reported_as_serde_then_validate_report_them() {
    let body = r#"{ "card": { "validFrom": 1990, "expiresIn": 1980 } }"#;
    let read_by_serde = serde_json::from_str::<PaymentThenValidated>(body).unwrap();

    let issues_validating_after_serde = read_by_serde.validate().unwrap_err();

    assert_eq!(
        deserialize::<Payment>(body).unwrap_err().issues(),
        issues_validating_after_serde.iter().cloned().collect::<Vec<_>>()
    );
}

fn last_night_follows_the_first(first_night: u8, last_night: u8) -> Result<(), Problem> {
    if last_night < first_night {
        return Err(Problem::custom(
            "ends_before_it_starts",
            "must not end before it starts",
        ));
    }
    Ok(())
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
enum Stay {
    #[validate(custom(|&Stay::Hotel { first_night, last_night }| last_night_follows_the_first(first_night, last_night)))]
    Hotel {
        #[validate(range(max = 31))]
        first_night: u8,
        last_night: u8,
    },
}

#[test]
fn a_variant_s_rule_without_a_field_reports_at_the_variant_s_name() {
    let body = r#"{ "hotel": { "first_night": 12, "last_night": 9 } }"#;

    assert_eq!(issues_reading::<Stay>(body), ["hotel: ends_before_it_starts"]);
}

#[test]
fn a_variant_s_rule_without_a_field_runs_whatever_the_fields_broke() {
    let body = r#"{ "hotel": { "first_night": 40, "last_night": 9 } }"#;

    assert_eq!(
        issues_reading::<Stay>(body),
        ["hotel.first_night: range", "hotel: ends_before_it_starts"]
    );
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "camelCase")]
enum TaggedStay {
    #[validate(custom(|&TaggedStay::Hotel { first_night, last_night }| last_night_follows_the_first(first_night, last_night)))]
    #[validate(custom(
        |&TaggedStay::Hotel { first_night, last_night }| last_night_follows_the_first(first_night, last_night),
        field = last_night
    ))]
    Hotel { first_night: u8, last_night: u8 },
}

#[test]
fn the_rules_of_an_internally_tagged_variant_report_at_the_enum_s_location() {
    let body = r#"{ "type": "hotel", "first_night": 12, "last_night": 9 }"#;

    assert_eq!(
        issues_reading::<TaggedStay>(body),
        ["ends_before_it_starts", "last_night: ends_before_it_starts"]
    );
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", content = "details", rename_all = "camelCase")]
enum StayWithDetails {
    #[validate(custom(|&StayWithDetails::Hotel { first_night, last_night }| last_night_follows_the_first(first_night, last_night)))]
    #[validate(custom(
        |&StayWithDetails::Hotel { first_night, last_night }| last_night_follows_the_first(first_night, last_night),
        field = last_night
    ))]
    Hotel { first_night: u8, last_night: u8 },
}

#[test]
fn the_rules_of_an_adjacently_tagged_variant_report_under_the_content_s_name() {
    let body = r#"{ "type": "hotel", "details": { "first_night": 12, "last_night": 9 } }"#;

    assert_eq!(
        issues_reading::<StayWithDetails>(body),
        [
            "details: ends_before_it_starts",
            "details.last_night: ends_before_it_starts"
        ]
    );
}

#[test]
fn the_rules_of_an_adjacently_tagged_variant_are_checked_when_its_content_comes_before_its_tag() {
    let body = r#"{ "details": { "first_night": 12, "last_night": 9 }, "type": "hotel" }"#;

    assert_eq!(
        issues_reading::<StayWithDetails>(body),
        [
            "details: ends_before_it_starts",
            "details.last_night: ends_before_it_starts"
        ]
    );
}

fn a_hostel_has_beds(stay: &UntaggedStay) -> Result<(), Problem> {
    let UntaggedStay::Hostel { beds, .. } = stay else {
        panic!("checked for a variant the rule isn't written on")
    };
    if *beds == 0 {
        return Err(Problem::custom("no_beds", "must have a bed at least"));
    }
    Ok(())
}

fn a_hotel_stay_ends_after_it_starts(stay: &UntaggedStay) -> Result<(), Problem> {
    let UntaggedStay::Hotel {
        first_night,
        last_night,
    } = stay
    else {
        panic!("checked for a variant the rule isn't written on")
    };
    last_night_follows_the_first(*first_night, *last_night)
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
#[serde(untagged)]
enum UntaggedStay {
    #[validate(custom(a_hostel_has_beds, field = beds))]
    Hostel { beds: u8, first_night: u8 },
    #[validate(custom(a_hotel_stay_ends_after_it_starts))]
    Hotel { first_night: u8, last_night: u8 },
}

#[test]
fn an_untagged_variant_is_checked_by_its_rule_once_it_is_the_one_built() {
    let body = r#"{ "first_night": 12, "last_night": 9 }"#;

    assert_eq!(issues_reading::<UntaggedStay>(body), ["ends_before_it_starts"]);
}

#[test]
fn a_broken_rule_does_not_stop_an_untagged_variant_from_being_built() {
    let body = r#"{ "beds": 0, "first_night": 12, "last_night": 9 }"#;

    assert_eq!(issues_reading::<UntaggedStay>(body), ["beds: no_beds"]);
}

fn the_upper_bound_follows_the_lower(lower: u32, upper: u32) -> Result<(), Problem> {
    if upper < lower {
        return Err(Problem::custom(
            "bounds_out_of_order",
            "must not be below the lower bound",
        ));
    }
    Ok(())
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
enum Filter {
    #[validate(custom(|&Filter::Between(lower, upper)| the_upper_bound_follows_the_lower(lower, upper), field = 1))]
    Between(u32, u32),
}

#[test]
fn a_rule_on_a_tuple_variant_names_an_element_by_its_index() {
    let body = r#"{ "between": [10, 5] }"#;

    assert_eq!(issues_reading::<Filter>(body), ["between[1]: bounds_out_of_order"]);
}

#[derive(serde::Deserialize, Debug)]
struct Bounds {
    lower: u32,
    upper: u32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
enum Selection {
    #[validate(custom(|Selection::Within(bounds)| the_upper_bound_follows_the_lower(bounds.lower, bounds.upper)))]
    Within(Bounds),
}

#[test]
fn a_rule_on_a_newtype_variant_reports_at_the_variant_s_name() {
    let body = r#"{ "within": { "lower": 10, "upper": 5 } }"#;

    assert_eq!(issues_reading::<Selection>(body), ["within: bounds_out_of_order"]);
}

fn an_announced_event(event: &Event) -> Result<(), Problem> {
    let Event::Gig { is_announced, .. } = event;
    if !is_announced {
        return Err(Problem::custom("not_announced", "must be announced"));
    }
    Ok(())
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

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
#[validate(custom(an_announced_event))]
enum Event {
    #[validate(custom(|&Event::Gig { starts_at, ends_at, .. }| ends_after_it_starts(starts_at, ends_at), field = ends_at))]
    #[validate(custom(|&Event::Gig { starts_at, ends_at, .. }| lasts_an_hour_at_least(starts_at, ends_at), field = ends_at))]
    Gig {
        #[validate(range(min = 1200))]
        starts_at: u16,
        ends_at: u16,
        is_announced: bool,
    },
}

#[test]
fn a_variant_s_rules_are_checked_between_its_fields_and_the_enum_s_rules() {
    let body = r#"{ "gig": { "startsAt": 1100, "endsAt": 1030, "isAnnounced": false } }"#;

    assert_eq!(
        issues_reading::<Event>(body),
        [
            "gig.startsAt: range",
            "gig.endsAt: ends_before_it_starts",
            "not_announced"
        ]
    );
}

#[test]
fn failing_fast_answers_the_first_issue_of_a_variant_s_rules() {
    let body = r#"{ "gig": { "startsAt": 2000, "endsAt": 2030, "isAnnounced": false } }"#;

    assert_eq!(fail_fast_issues_reading::<Event>(body), ["gig.endsAt: too_short"]);
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
#[validate(report_all)]
enum Show {
    #[validate(custom(|&Show::Matinee { starts_at, ends_at }| ends_after_it_starts(starts_at, ends_at), field = ends_at))]
    #[validate(custom(|&Show::Matinee { starts_at, ends_at }| lasts_an_hour_at_least(starts_at, ends_at), field = ends_at))]
    Matinee {
        starts_at: u16,
        #[validate(range(min = 1200))]
        ends_at: u16,
    },
}

#[test]
fn report_all_on_the_enum_checks_every_rule_of_a_variant_at_a_field_whatever_the_field_broke() {
    let body = r#"{ "matinee": { "starts_at": 1400, "ends_at": 1130 } }"#;

    assert_eq!(
        issues_reading::<Show>(body),
        [
            "matinee.ends_at: range",
            "matinee.ends_at: ends_before_it_starts",
            "matinee.ends_at: too_short"
        ]
    );
}

fn route_must_not_be_checked(_: &Route) -> Result<(), Problem> {
    panic!("checked after failing fast found its issue")
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
#[serde(rename_all = "camelCase")]
enum Route {
    #[validate(custom(route_must_not_be_checked))]
    Direct {
        #[validate(required)]
        destination: String,
    },
}

#[test]
fn failing_fast_checks_no_rule_of_the_variant_after_the_first_issue() {
    let body = r#"{ "direct": { "destination": "" } }"#;

    assert_eq!(
        fail_fast_issues_reading::<Route>(body),
        ["direct.destination: required"]
    );
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
#[annotated(collect_type_errors)]
#[serde(rename_all = "camelCase")]
enum Reading {
    #[validate(custom(|_| Err(Problem::custom("always_broken", "is never accepted"))))]
    Measured { celsius: i16, metres: u16 },
}

#[test]
fn a_variant_with_a_value_of_the_wrong_type_is_not_checked_by_its_rule() {
    let body = r#"{ "measured": { "celsius": "mild", "metres": 300 } }"#;

    assert_eq!(issues_reading::<Reading>(body), ["measured.celsius: invalid_type"]);
}

#[test]
fn a_variant_collecting_type_errors_is_checked_by_its_rule_once_it_is_read() {
    let body = r#"{ "measured": { "celsius": 21, "metres": 300 } }"#;

    assert_eq!(issues_reading::<Reading>(body), ["measured: always_broken"]);
}

fn an_ordered_pair<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<(u32, u32), D::Error> {
    let text = <&str as serde::Deserialize>::deserialize(deserializer)?;
    let (lower, upper) = text
        .split_once("..")
        .ok_or_else(|| serde::de::Error::custom("no bounds"))?;
    let bound = |text: &str| text.parse().map_err(serde::de::Error::custom);
    Ok((bound(lower)?, bound(upper)?))
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
enum Span {
    #[serde(deserialize_with = "an_ordered_pair")]
    #[validate(custom(|&Span::Between(lower, upper)| the_upper_bound_follows_the_lower(lower, upper), field = 1))]
    Between(u32, u32),
}

#[test]
fn a_variant_read_by_a_function_is_checked_by_its_rule() {
    let body = r#"{ "between": "10..5" }"#;

    assert_eq!(issues_reading::<Span>(body), ["between[1]: bounds_out_of_order"]);
}

#[derive(serde::Deserialize, Debug)]
struct Period {
    first_night: u8,
    last_night: u8,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
#[serde(tag = "type", rename_all = "camelCase")]
enum Booking {
    #[validate(custom(
        |Booking::Room { period, .. }| last_night_follows_the_first(period.first_night, period.last_night),
        field = period
    ))]
    Room {
        guests: u8,
        #[serde(flatten)]
        period: Period,
    },
}

#[test]
fn a_rule_naming_a_flattened_field_of_a_variant_reports_where_the_variant_s_content_is() {
    let body = r#"{ "type": "room", "guests": 2, "first_night": 12, "last_night": 9 }"#;

    assert_eq!(issues_reading::<Booking>(body), ["ends_before_it_starts"]);
}
