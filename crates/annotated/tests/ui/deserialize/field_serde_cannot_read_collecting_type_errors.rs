use annotated::Deserialize;

struct Address {
    street: String,
}

#[derive(Deserialize)]
#[annotated(collect_type_errors)]
struct Order {
    shipping_address: Address,
}

fn main() {}
