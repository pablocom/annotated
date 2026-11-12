use annotated::Deserialize;

#[derive(Default)]
struct Address {
    street: String,
}

#[derive(Deserialize)]
struct Order {
    #[serde(skip)]
    #[validate(nested)]
    billing_address: Address,
}

fn main() {}
