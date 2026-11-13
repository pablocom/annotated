use annotated::Validate;

#[derive(Validate)]
enum Payment {
    Card { number: String },
    #[validate(custom(is_accepted))]
    Cash,
}

fn main() {}
