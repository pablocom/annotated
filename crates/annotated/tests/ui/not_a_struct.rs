use annotated::Validate;

#[derive(Validate)]
enum Payment {
    Card,
    Transfer,
}

fn main() {}
