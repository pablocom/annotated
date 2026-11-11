use annotated::Validate;

#[derive(Validate)]
struct Address {
    #[validate(required)]
    street: String,
}

#[derive(Validate)]
struct Order {
    #[validate(nested(message = "must be a valid address"))]
    shipping_address: Address,
}

fn main() {}
