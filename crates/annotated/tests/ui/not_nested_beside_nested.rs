use annotated::Validate;

#[derive(Validate)]
struct Address {
    #[validate(required)]
    street: String,
}

#[derive(Validate)]
struct Order {
    #[validate(nested, not_nested)]
    shipping_address: Address,
}

#[derive(Validate)]
struct Route {
    #[validate(not_nested, each(nested))]
    stops: Vec<Address>,
}

fn main() {}
