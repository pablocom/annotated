use annotated::{Deserialize, Validate};

#[derive(Validate)]
struct Address {
    #[validate(required)]
    street: String,
}

#[derive(Deserialize)]
struct Order {
    #[validate(nested)]
    shipping_address: Address,
}

#[derive(Deserialize)]
struct Route {
    #[validate(length(max = 10), each(nested))]
    stops: Vec<Address>,
}

fn main() {}
