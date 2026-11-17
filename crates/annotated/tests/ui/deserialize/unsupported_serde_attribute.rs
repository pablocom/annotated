use annotated::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", transparent)]
struct NewMember {
    #[validate(required)]
    full_name: String,
}

#[derive(Deserialize)]
#[serde(try_from = "String")]
struct Handle {
    name: String,
}

#[derive(Deserialize)]
struct Receipt {
    #[serde(getter = "Receipt::total")]
    total: u32,
}

fn main() {}
