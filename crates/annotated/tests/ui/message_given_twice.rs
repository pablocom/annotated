use annotated::{Problem, Validate};

#[derive(Validate)]
struct Password {
    #[validate(required(message = "is needed", message = "can't be blank"))]
    password: String,
}

fn is_consistent(_: &NewPassword) -> Result<(), Problem> {
    Ok(())
}

#[derive(Validate)]
#[validate(custom(is_consistent, message = "must match", field = repeated, message = "must be equal"))]
struct NewPassword {
    password: String,
    repeated: String,
}

fn is_unique(_: &Pair) -> Result<(), Problem> {
    Ok(())
}

#[derive(Validate)]
#[validate(custom(is_unique, field = first, field = second))]
struct Pair {
    first: String,
    second: String,
}

fn main() {}
