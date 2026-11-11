use annotated::Validate;

#[derive(Validate)]
struct Address {
    #[validate(pattern = r"^\d{5}$")]
    postal_code: String,
}

fn main() {}
