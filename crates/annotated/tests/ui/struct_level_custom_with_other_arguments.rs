use annotated::Validate;

#[derive(Validate)]
#[validate(custom(passwords_match, passwords_are_long))]
struct NewPassword {
    password: String,
    repeated_password: String,
}

#[derive(Validate)]
#[validate(custom(pins_match, at = repeated_pin))]
struct NewPin {
    pin: String,
    repeated_pin: String,
}

#[derive(Validate)]
#[validate(custom = phrases_match)]
struct NewPassphrase {
    passphrase: String,
    repeated_passphrase: String,
}

fn main() {}
