use annotated::Validate;

#[derive(Validate)]
struct Card {
    #[validate(length(equal = 16, max = 19))]
    number: String,
}

#[derive(Validate)]
struct SecurityCode {
    #[validate(length(min = 3, equal = 3))]
    digits: String,
}

fn main() {}
