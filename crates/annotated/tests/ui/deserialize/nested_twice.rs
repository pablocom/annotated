use annotated::Deserialize;

#[derive(Deserialize)]
struct Address {
    #[validate(required)]
    street: String,
}

#[derive(Deserialize)]
struct Route {
    #[validate(nested, each(nested))]
    stops: Vec<Address>,
}

fn main() {}
