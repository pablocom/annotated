use annotated::Validate;

const POSTAL_CODE: &str = r"^\d{5}$";

#[derive(Validate)]
struct Address {
    #[validate(pattern = POSTAL_CODE)]
    postal_code: String,
}

#[derive(Validate)]
struct Office {
    #[validate(pattern(expression = r"^\d{5}$"))]
    postal_code: String,
}

fn main() {}
