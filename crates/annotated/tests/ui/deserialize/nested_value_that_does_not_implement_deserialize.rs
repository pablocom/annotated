use annotated::{Deserialize, Validate};

#[derive(Validate, serde::Deserialize)]
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
    #[validate(each(nested))]
    stops: Vec<Address>,
}

fn main() {}
