use annotated::{Validate, Violation};

pub fn violations_of(value: &impl Validate) -> Vec<String> {
    match value.validate() {
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
