use annotated::Validate;

#[derive(Validate)]
#[validate(custom(passwords_match, field = repeated_pasword))]
struct NewPassword {
    password: String,
    repeated_password: String,
}

#[derive(Validate)]
#[serde(rename_all = "camelCase")]
#[validate(custom(pins_match, field = "repeatedPin"))]
struct NewPin {
    pin: String,
    repeated_pin: String,
}

fn main() {}
