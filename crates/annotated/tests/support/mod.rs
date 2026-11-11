use annotated::{Validate, Violation, Violations};

pub fn violations_of(value: &impl Validate) -> Vec<String> {
    paths_and_codes(value.validate())
}

pub fn paths_and_codes(outcome: Result<(), Violations>) -> Vec<String> {
    match outcome {
        Ok(()) => Vec::new(),
        Err(violations) => violations.iter().map(path_and_code).collect(),
    }
}

fn path_and_code(violation: &Violation) -> String {
    let code = violation.problem().code();
    if violation.path().is_root() {
        return code.to_owned();
    }
    format!("{}: {code}", violation.path())
}
