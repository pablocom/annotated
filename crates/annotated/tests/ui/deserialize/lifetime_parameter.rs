use annotated::Deserialize;

#[derive(Deserialize)]
struct NewMember<'a> {
    #[validate(required)]
    full_name: &'a str,
}

fn main() {}
