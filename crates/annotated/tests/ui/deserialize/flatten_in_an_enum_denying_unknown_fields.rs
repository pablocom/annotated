use annotated::Deserialize;

#[derive(Deserialize)]
struct Address {
    street: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
enum Shipping {
    Pallet {
        #[serde(flatten)]
        address: Address,
    },
}

fn main() {}
