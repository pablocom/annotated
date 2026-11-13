//! Rules on a variant, checked after its fields when the enum holds that variant.

mod support;

use annotated::{Problem, Validate};
use support::{issues_of, paths_and_codes};

fn a_card_expires_after_it_starts(payment: &Payment) -> Result<(), Problem> {
    let Payment::Card { valid_from, expires_in } = payment else {
        panic!("checked for a variant the rule isn't written on")
    };
    if expires_in <= valid_from {
        return Err(Problem::custom("expires_too_soon", "must be after the card starts"));
    }
    Ok(())
}

#[derive(Validate)]
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
fn a_variant_following_its_rule_has_no_issues() {
    let payment = Payment::Card {
        valid_from: 2026,
        expires_in: 2030,
    };

    assert_eq!(payment.validate(), Ok(()));
}

#[test]
fn a_variant_s_rule_naming_a_field_reports_at_it_by_its_serde_name() {
    let payment = Payment::Card {
        valid_from: 2030,
        expires_in: 2026,
    };

    assert_eq!(issues_of(&payment), ["card.expiresIn: expires_too_soon"]);
}

#[test]
fn a_variant_s_rule_is_not_checked_when_the_enum_holds_another_variant() {
    let payment = Payment::BankTransfer {
        iban: "ES9121000418450200051332".to_owned(),
    };

    assert_eq!(payment.validate(), Ok(()));
}

#[test]
fn a_variant_s_rules_are_checked_after_its_fields() {
    let payment = Payment::Card {
        valid_from: 1990,
        expires_in: 1980,
    };

    assert_eq!(
        issues_of(&payment),
        ["card.validFrom: range", "card.expiresIn: expires_too_soon"]
    );
}

#[test]
fn a_variant_s_rule_at_a_field_is_skipped_when_the_field_breaks_its_own_rules() {
    let payment = Payment::Card {
        valid_from: 2300,
        expires_in: 2200,
    };

    assert_eq!(issues_of(&payment), ["card.expiresIn: range"]);
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

#[derive(Validate)]
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
    let stay = Stay::Hotel {
        first_night: 12,
        last_night: 9,
    };

    assert_eq!(issues_of(&stay), ["hotel: ends_before_it_starts"]);
}

#[test]
fn a_variant_s_rule_without_a_field_runs_whatever_the_fields_broke() {
    let stay = Stay::Hotel {
        first_night: 40,
        last_night: 9,
    };

    assert_eq!(
        issues_of(&stay),
        ["hotel.first_night: range", "hotel: ends_before_it_starts"]
    );
}

#[derive(Validate)]
#[serde(tag = "type", rename_all = "camelCase")]
enum TaggedStay {
    #[validate(custom(|&TaggedStay::Hotel { first_night, last_night }| last_night_follows_the_first(first_night, last_night)))]
    Hotel { first_night: u8, last_night: u8 },
}

#[test]
fn a_rule_on_an_internally_tagged_variant_reports_at_the_enum_s_location() {
    let stay = TaggedStay::Hotel {
        first_night: 12,
        last_night: 9,
    };

    assert_eq!(issues_of(&stay), ["ends_before_it_starts"]);
}

#[derive(Validate)]
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
fn a_rule_on_an_adjacently_tagged_variant_reports_under_the_content_s_name() {
    let stay = StayWithDetails::Hotel {
        first_night: 12,
        last_night: 9,
    };

    assert_eq!(
        issues_of(&stay),
        [
            "details: ends_before_it_starts",
            "details.last_night: ends_before_it_starts"
        ]
    );
}

#[derive(Validate)]
#[serde(untagged)]
enum UntaggedStay {
    #[validate(custom(|&UntaggedStay::Hotel { first_night, last_night }| last_night_follows_the_first(first_night, last_night)))]
    Hotel { first_night: u8, last_night: u8 },
}

#[test]
fn a_rule_on_an_untagged_variant_reports_at_the_enum_s_location() {
    let stay = UntaggedStay::Hotel {
        first_night: 12,
        last_night: 9,
    };

    assert_eq!(issues_of(&stay), ["ends_before_it_starts"]);
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

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
enum Filter {
    #[validate(custom(|&Filter::Between(lower, upper)| the_upper_bound_follows_the_lower(lower, upper), field = 1))]
    Between(u32, u32),
}

#[test]
fn a_rule_on_a_tuple_variant_names_an_element_by_its_index() {
    let filter = Filter::Between(10, 5);

    assert_eq!(issues_of(&filter), ["between[1]: bounds_out_of_order"]);
}

struct Bounds {
    lower: u32,
    upper: u32,
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
enum Selection {
    #[validate(custom(|Selection::Within(bounds)| the_upper_bound_follows_the_lower(bounds.lower, bounds.upper)))]
    Within(Bounds),
}

#[test]
fn a_rule_on_a_newtype_variant_reports_at_the_variant_s_name() {
    let selection = Selection::Within(Bounds { lower: 10, upper: 5 });

    assert_eq!(issues_of(&selection), ["within: bounds_out_of_order"]);
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
enum Search {
    #[validate(custom(
        |Search::Among(bounds)| the_upper_bound_follows_the_lower(bounds.lower, bounds.upper),
        field = 0
    ))]
    Among(Bounds),
}

#[test]
fn a_rule_naming_a_newtype_variant_s_value_reports_at_the_variant_s_name() {
    let search = Search::Among(Bounds { lower: 10, upper: 5 });

    assert_eq!(issues_of(&search), ["among: bounds_out_of_order"]);
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

fn doors_open_before_it_starts(event: &Event) -> Result<(), Problem> {
    let Event::Gig {
        doors_open, starts_at, ..
    } = event;
    if doors_open > starts_at {
        return Err(Problem::custom("doors_open_too_late", "must open by the start"));
    }
    Ok(())
}

fn is_announced(event: &Event) -> Result<(), Problem> {
    let Event::Gig { is_announced, .. } = event;
    if !is_announced {
        return Err(Problem::custom("not_announced", "must be announced"));
    }
    Ok(())
}

#[derive(Validate)]
#[serde(rename_all = "camelCase", rename_all_fields = "camelCase")]
#[validate(custom(is_announced))]
enum Event {
    #[validate(custom(|&Event::Gig { starts_at, ends_at, .. }| ends_after_it_starts(starts_at, ends_at), field = ends_at))]
    #[validate(custom(|&Event::Gig { starts_at, ends_at, .. }| lasts_an_hour_at_least(starts_at, ends_at), field = ends_at))]
    #[validate(custom(
        doors_open_before_it_starts,
        field = doors_open,
        message = "doors must open by the start"
    ))]
    Gig {
        #[serde(rename = "doors")]
        doors_open: u16,
        starts_at: u16,
        ends_at: u16,
        is_announced: bool,
    },
}

#[test]
fn every_rule_of_a_variant_is_checked_in_the_order_written() {
    let event = Event::Gig {
        doors_open: 2030,
        starts_at: 2000,
        ends_at: 2030,
        is_announced: true,
    };

    assert_eq!(
        issues_of(&event),
        ["gig.endsAt: too_short", "gig.doors: doors_open_too_late"]
    );
}

#[test]
fn the_first_rule_of_a_variant_a_field_breaks_stops_its_later_ones() {
    let event = Event::Gig {
        doors_open: 1900,
        starts_at: 2000,
        ends_at: 1930,
        is_announced: true,
    };

    assert_eq!(issues_of(&event), ["gig.endsAt: ends_before_it_starts"]);
}

#[test]
fn a_variant_s_rules_are_checked_before_the_enum_s() {
    let event = Event::Gig {
        doors_open: 1900,
        starts_at: 2000,
        ends_at: 1930,
        is_announced: false,
    };

    assert_eq!(
        issues_of(&event),
        ["gig.endsAt: ends_before_it_starts", "not_announced"]
    );
}

#[test]
fn a_message_replaces_the_problem_s_own_on_a_variant_s_rule() {
    let event = Event::Gig {
        doors_open: 2030,
        starts_at: 2000,
        ends_at: 2200,
        is_announced: true,
    };

    assert_eq!(
        event.validate().unwrap_err().to_string(),
        "gig.doors: doors must open by the start"
    );
}

#[derive(Validate)]
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
    let show = Show::Matinee {
        starts_at: 1400,
        ends_at: 1130,
    };

    assert_eq!(
        issues_of(&show),
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

#[derive(Validate)]
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
    let route = Route::Direct {
        destination: String::new(),
    };

    assert_eq!(
        paths_and_codes(route.validate_fail_fast()),
        ["direct.destination: required"]
    );
}

fn within_the_weight_limit<T>(parcel: &Parcel<T>) -> Result<(), Problem> {
    let Parcel::Weighed { grams, .. } = parcel;
    if *grams > 30_000 {
        return Err(Problem::custom("too_heavy", "must weigh 30 kg at most"));
    }
    Ok(())
}

#[derive(Validate)]
#[allow(dead_code)]
enum Parcel<T> {
    #[validate(custom(within_the_weight_limit, field = grams))]
    Weighed { grams: u32, contents: T },
}

#[test]
fn a_rule_on_a_variant_of_an_enum_with_a_type_parameter_checks_the_enum() {
    let parcel = Parcel::Weighed {
        grams: 45_000,
        contents: "books",
    };

    assert_eq!(issues_of(&parcel), ["Weighed.grams: too_heavy"]);
}
