use annotated::Validate;

#[derive(Validate)]
#[validate(required)]
enum Payment {
    Card {
        #[validate(length(min = 16))]
        number: String,
    },
    Cash,
}

fn main() {}
