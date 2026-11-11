use annotated::Validate;

#[derive(Validate)]
#[validate(custom)]
struct NewPassword {
    password: String,
    repeated_password: String,
}

#[derive(Validate)]
#[validate(custom(field = repeated_pin))]
struct NewPin {
    pin: String,
    repeated_pin: String,
}

fn main() {}
