use annotated::Validate;

#[derive(Validate)]
struct Account {
    #[validate(pattern = r"^[\p{L}\p{N}_-]{1,1000}$")]
    name: String,
}

fn main() {}
