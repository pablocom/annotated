use annotated::Validate;

#[derive(Validate)]
struct Address {
    #[validate(required)]
    street: String,
}

#[derive(Validate)]
struct Route {
    #[validate(each(not_nested))]
    stops: Vec<Address>,
}

fn main() {}
