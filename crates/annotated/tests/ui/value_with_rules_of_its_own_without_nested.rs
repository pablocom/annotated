use std::collections::{HashMap, HashSet};

use annotated::Validate;

#[derive(Validate)]
struct Address {
    #[validate(required)]
    street: String,
}

#[derive(Validate)]
struct Order {
    #[validate(required)]
    reference: String,
    shipping_address: Address,
}

#[derive(Validate)]
struct Invoice {
    billing_address: Option<Address>,
}

#[derive(Validate)]
struct Route {
    #[validate(length(max = 10))]
    stops: Vec<Address>,
}

#[derive(Validate)]
enum Delivery {
    Courier(Address),
    Pickup,
}

#[derive(Validate)]
struct Directory {
    addresses_by_city: HashMap<String, Vec<Address>>,
}

#[derive(Validate)]
struct Warehouse {
    manager_address: Box<Address>,
    docks: HashMap<String, Address>,
    gates: &'static [Address],
    suppliers: HashSet<Address>,
}

fn main() {}
