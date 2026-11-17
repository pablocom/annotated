use annotated::Deserialize;

#[derive(Deserialize)]
#[serde(crate = "elsewhere::serde")]
struct NewMember {
    #[validate(required)]
    full_name: String,
}

fn main() {}
