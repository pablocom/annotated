use annotated::{Issue, Issues, Validate};

pub fn issues_of(value: &impl Validate) -> Vec<String> {
    paths_and_codes(value.validate())
}

pub fn paths_and_codes(outcome: Result<(), Issues>) -> Vec<String> {
    match outcome {
        Ok(()) => Vec::new(),
        Err(issues) => issues.iter().map(path_and_code).collect(),
    }
}

fn path_and_code(issue: &Issue) -> String {
    let code = issue.problem().code();
    if issue.path().is_root() {
        return code.to_owned();
    }
    format!("{}: {code}", issue.path())
}
