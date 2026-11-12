use annotated::Deserialize;
use serde::Deserializer;

struct Address {
    street: String,
}

fn street_only<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Address, D::Error> {
    let street = <String as serde::Deserialize>::deserialize(deserializer)?;
    Ok(Address { street })
}

#[derive(Deserialize)]
struct Order {
    #[serde(deserialize_with = "street_only")]
    #[validate(nested)]
    shipping_address: Address,
}

fn main() {}
