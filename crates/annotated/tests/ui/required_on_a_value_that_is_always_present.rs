use annotated::Validate;

#[derive(Validate)]
struct NewMember {
    #[validate(required)]
    age: u8,
}

fn main() {}
