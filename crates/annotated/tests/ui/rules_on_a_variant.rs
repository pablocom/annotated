use annotated::Validate;

#[derive(Validate)]
enum Payment {
    #[validate(custom(is_valid_card))]
    Card { number: String },
    Cash,
}

fn main() {}
