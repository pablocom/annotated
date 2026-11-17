//! An app moves an issue it already has into a report: one found by another validation, or by a check of its own.

use annotated::{Issue, Location, Path, Problem, Report, Rule, Validate, rules};

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
struct NewMember {
    #[validate(required)]
    full_name: String,
    #[validate(email)]
    email: String,
}

fn email_taken() -> Issue {
    Issue::new(
        Path::root().field("email"),
        Problem::custom("email_taken", "belongs to another member"),
    )
}

#[test]
fn an_issue_pushed_whole_keeps_its_path_and_its_message() {
    let too_long = Rule::<str>::problem(&rules::length().max(50)).unwrap();
    let issue = Issue::new(Path::root().field("bio"), too_long).with_message("keep it under {max}");
    let mut found = Report::new();

    found.push_issue(issue);

    assert_eq!(found.into_result().unwrap_err().to_string(), "bio: keep it under 50");
}

#[test]
fn an_issue_of_an_app_s_own_check_joins_the_issues_of_a_validation() {
    let new_member = NewMember {
        full_name: String::new(),
        email: "alex@example.com".to_owned(),
    };
    let mut found = Report::new();
    new_member.validate_into(&Location::root(), &mut found);

    found.push_issue(email_taken());

    assert_eq!(
        found.into_result().unwrap_err().to_string(),
        "fullName: is required\nemail: belongs to another member"
    );
}

#[test]
fn the_issues_of_another_validation_are_pushed_into_a_report_in_their_order() {
    let new_member = NewMember {
        full_name: String::new(),
        email: "alex".to_owned(),
    };
    let issues = new_member.validate().unwrap_err();
    let [blank_name, not_an_email] = issues.as_slice() else {
        panic!("expected two issues, got {issues:?}");
    };
    let mut found = Report::new();

    found.push_issue(blank_name.clone());
    found.push_issue(not_an_email.clone());

    assert_eq!(found.into_result(), Err(issues));
}

#[test]
fn a_report_at_its_limit_records_no_issue_pushed_whole() {
    let mut found = Report::failing_fast();
    found.push(&Location::root().field("fullName"), Problem::Required);

    found.push_issue(email_taken());

    assert_eq!(found.into_result().unwrap_err().to_string(), "fullName: is required");
}
