use annotated::Validate;

struct Address {
    street: String,
}

#[derive(Validate)]
struct Order {
    #[validate(nested)]
    shipping_address: Address,
}

fn main() {}
