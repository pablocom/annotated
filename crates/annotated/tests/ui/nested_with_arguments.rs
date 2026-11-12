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

#[derive(Validate)]
struct Invoice {
    #[validate(nested = true)]
    billing_address: Address,
}

#[derive(Validate)]
struct Parcel {
    #[validate(nested(depth = 2))]
    return_address: Address,
}

fn main() {}
