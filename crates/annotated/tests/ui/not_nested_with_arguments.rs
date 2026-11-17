use annotated::Validate;

#[derive(Validate)]
struct Address {
    #[validate(required)]
    street: String,
}

#[derive(Validate)]
struct Order {
    #[validate(not_nested(message = "is checked elsewhere"))]
    shipping_address: Address,
}

#[derive(Validate)]
struct Invoice {
    #[validate(not_nested = true)]
    billing_address: Address,
}

fn main() {}
