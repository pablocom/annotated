use annotated::Validate;

#[derive(Validate)]
struct NewMember {
    #[validate(length(max = 3))]
    age: u8,
}

fn main() {}
