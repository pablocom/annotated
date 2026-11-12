use annotated::Deserialize;

#[derive(Deserialize, Default)]
struct Address {
    #[validate(required)]
    street: String,
}

#[derive(Deserialize)]
struct Order {
    #[serde(default)]
    #[validate(nested)]
    shipping_address: Address,
}

fn main() {}
