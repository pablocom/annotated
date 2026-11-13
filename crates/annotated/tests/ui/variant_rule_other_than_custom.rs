use annotated::Validate;

#[derive(Validate)]
enum Payment {
    #[validate(required)]
    Card { number: String },
    Cash,
}

#[derive(Validate)]
enum Username {
    #[validate(report_all)]
    Chosen { name: String },
}

fn main() {}
