use annotated::Deserialize;

struct Address {
    street: String,
}

#[derive(Deserialize)]
struct Order {
    shipping_address: Address,
}

fn main() {}
