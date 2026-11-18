use std::collections::{BTreeSet, HashMap};

use annotated::{Deserialize, Validate};

#[derive(Validate, Deserialize, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Address {
    #[validate(required)]
    street: String,
}

#[derive(Deserialize)]
struct Order {
    #[validate(required)]
    reference: String,
    shipping_address: Address,
}

#[derive(Deserialize)]
struct Invoice {
    billing_address: Option<Address>,
}

#[derive(Deserialize)]
#[annotated(collect_type_errors)]
struct Route {
    #[validate(length(max = 10))]
    stops: Vec<Address>,
}

#[derive(Deserialize)]
enum Delivery {
    Courier(Address),
    Pickup,
}

#[derive(Deserialize)]
struct Directory {
    addresses_by_city: HashMap<String, Vec<Address>>,
}

#[derive(Deserialize)]
struct Draft {
    #[serde(skip)]
    address: Address,
}

#[derive(Deserialize)]
struct Mailing {
    recipients: BTreeSet<Address>,
}

fn main() {}
